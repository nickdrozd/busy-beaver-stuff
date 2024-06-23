use std::collections::HashMap as Dict;

use pyo3::{pyclass, pyfunction, pymethods};

use crate::{
    instrs::{CompProg, Slot, State},
    parse::tcompile,
    tape::{BasicTape as Tape, Count, HeadTape},
};

type Step = u64;

type Blanks = Dict<State, Step>;

/**************************************/

#[allow(non_camel_case_types)]
#[pyclass]
#[derive(Clone)]
pub enum TermRes {
    xlimit,
    infrul,
    spnout,
    undfnd,
}

/**************************************/

#[pyclass]
pub struct MachineResult {
    result: TermRes,

    steps: Step,
    cycles: Step,
    marks: Count,

    blanks: Blanks,

    last_slot: Option<Slot>,
}

#[pymethods]
impl MachineResult {
    #[new]
    const fn new(
        result: TermRes,
        steps: Step,
        cycles: Step,
        marks: Count,
        blanks: Blanks,
        last_slot: Option<Slot>,
    ) -> Self {
        Self {
            result,
            steps,
            cycles,
            marks,
            blanks,
            last_slot,
        }
    }

    #[getter]
    const fn cycles(&self) -> Step {
        self.cycles
    }

    #[getter]
    const fn steps(&self) -> Step {
        self.steps
    }

    #[getter]
    const fn marks(&self) -> Count {
        self.marks
    }

    #[getter]
    fn blanks(&self) -> Blanks {
        self.blanks.clone()
    }

    #[getter]
    const fn undfnd(&self) -> Option<(Step, Slot)> {
        match self.result {
            TermRes::undfnd => {
                if let Some(slot) = self.last_slot {
                    Some((self.steps, slot))
                } else {
                    panic!()
                }
            },
            _ => None,
        }
    }

    #[getter]
    const fn simple_termination(&self) -> Option<Step> {
        match self.result {
            TermRes::undfnd | TermRes::spnout => Some(self.steps),
            _ => None,
        }
    }

    #[getter]
    const fn halted(&self) -> Option<Step> {
        match self.result {
            TermRes::undfnd => Some(self.steps),
            _ => None,
        }
    }

    #[getter]
    const fn infrul(&self) -> Option<Step> {
        match self.result {
            TermRes::infrul => Some(self.steps),
            _ => None,
        }
    }

    #[getter]
    const fn spnout(&self) -> Option<Step> {
        match self.result {
            TermRes::spnout => Some(self.steps),
            _ => None,
        }
    }

    #[getter]
    const fn xlimit(&self) -> Option<Step> {
        match self.result {
            TermRes::xlimit => Some(self.steps),
            _ => None,
        }
    }
}

#[pyfunction]
#[pyo3(signature = (prog, sim_lim=100_000_000))]
pub fn run_quick_machine(prog: &str, sim_lim: Step) -> MachineResult {
    let comp = tcompile(prog);

    let mut tape = Tape::init(0);

    let mut blanks = Blanks::new();

    let mut state = 0;
    let mut steps = 0;
    let mut cycles = 0;

    let mut result: Option<TermRes> = None;
    let mut last_slot: Option<Slot> = None;

    for cycle in 0..sim_lim {
        let slot = (state, tape.scan);

        let Some(&(color, shift, next_state)) = comp.get(&slot) else {
            cycles = cycle;
            result = Some(TermRes::undfnd);
            last_slot = Some(slot);
            break;
        };

        let same = state == next_state;

        if same && tape.at_edge(shift) {
            cycles = cycle;
            result = Some(TermRes::spnout);
            break;
        }

        let stepped = tape.step(shift, color, same);

        steps += stepped;

        state = next_state;

        if color == 0 && tape.blank() {
            if blanks.contains_key(&state) {
                result = Some(TermRes::infrul);
                break;
            }

            blanks.insert(state, steps);

            if state == 0 {
                result = Some(TermRes::infrul);
                break;
            }
        }
    }

    MachineResult {
        result: result.unwrap_or(TermRes::xlimit),
        steps,
        cycles,
        marks: tape.marks(),
        last_slot,
        blanks,
    }
}

/**************************************/

#[pyfunction]
pub fn quick_term_or_rec_py(prog: &str, sim_lim: u32) -> bool {
    quick_term_or_rec(&tcompile(prog), sim_lim, false)
}

pub fn quick_term_or_rec(
    comp: &CompProg,
    sim_lim: u32,
    drop_halt: bool,
) -> bool {
    let mut state = 1;

    let mut tape = HeadTape::init_stepped();

    let (mut step, mut cycle) = (1, 1);

    while cycle < sim_lim {
        let steps_reset = 2 * step;

        let (mut leftmost, mut rightmost) = (tape.head, tape.head);

        let init_state = state;

        let init_tape = tape.clone();

        while step < steps_reset && cycle < sim_lim {
            let Some(&(color, shift, next_state)) =
                comp.get(&(state, tape.scan()))
            else {
                return drop_halt;
            };

            let same = state == next_state;

            if same && tape.at_edge(shift) {
                return true;
            }

            let stepped = tape.step(shift, color, same);

            step += stepped;

            cycle += 1;

            state = next_state;

            let curr = tape.head;

            if curr < leftmost {
                leftmost = curr;
            } else if rightmost < curr {
                rightmost = curr;
            }

            if state != init_state {
                continue;
            }

            if tape.scan() != init_tape.scan() {
                continue;
            }

            if tape.aligns_with(&init_tape, leftmost, rightmost) {
                return true;
            }
        }
    }

    false
}
