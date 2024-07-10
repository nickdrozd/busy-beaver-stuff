use std::collections::BTreeMap as Dict;

use pyo3::{pyclass, pyfunction, pymethods};

use crate::{
    instrs::{CompProg, Slot, State},
    parse::tcompile,
    prover::{Prover, ProverResult},
    rules::apply_rule,
    tape::{BasicTape as Tape, Count, HeadTape},
};

type Step = u64;

type Blanks = Dict<State, Step>;

/**************************************/

#[expect(non_camel_case_types)]
#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TermRes {
    xlimit,
    cfglim,
    infrul,
    spnout,
    undfnd,
    mulrul,
}

/**************************************/

#[pyclass]
#[derive(Debug, PartialEq, Eq)]
pub struct MachineResult {
    result: TermRes,

    steps: Step,
    cycles: Step,
    marks: Count,
    rulapp: Count,

    blanks: Blanks,

    last_slot: Option<Slot>,
}

#[pymethods]
impl MachineResult {
    #[new]
    #[pyo3(signature = (result, steps, cycles, marks, rulapp, blanks, last_slot))]
    const fn new(
        result: TermRes,
        steps: Step,
        cycles: Step,
        marks: Count,
        rulapp: Count,
        blanks: Blanks,
        last_slot: Option<Slot>,
    ) -> Self {
        Self {
            result,
            steps,
            cycles,
            marks,
            rulapp,
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
    const fn rulapp(&self) -> Count {
        self.rulapp
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

    #[getter]
    const fn cfglim(&self) -> Option<Step> {
        match self.result {
            TermRes::cfglim => Some(self.steps),
            _ => None,
        }
    }
}

/**************************************/

#[cfg(test)]
use crate::macros::GetInstr;

#[cfg(test)]
pub fn run_for_infrul(comp: &impl GetInstr, sim_lim: Step) -> bool {
    let mut tape = Tape::init(0);

    let mut prover = Prover::new(comp);

    let mut state = 0;

    for cycle in 0..sim_lim {
        match prover.try_rule(cycle as i32, state, &tape) {
            None => {},
            Some(
                ProverResult::ConfigLimit | ProverResult::MultRule,
            ) => {
                return false;
            },
            Some(ProverResult::InfiniteRule) => {
                return true;
            },
            Some(ProverResult::Got(rule)) => {
                if apply_rule(&rule, &mut tape).is_some() {
                    // println!("--> applying rule: {:?}", rule);
                    continue;
                }
            },
        }

        let slot = (state, tape.scan);

        let Some((color, shift, next_state)) = comp.get_instr(&slot)
        else {
            return false;
        };

        let same = state == next_state;

        if same && tape.at_edge(shift) {
            return false;
        }

        tape.step(shift, color, same);

        state = next_state;
    }

    false
}

/**************************************/

#[pyfunction]
#[pyo3(signature = (prog, sim_lim=100_000_000))]
pub fn run_prover(prog: &str, sim_lim: Step) -> MachineResult {
    let comp = tcompile(prog);

    let mut tape = Tape::init(0);

    let mut prover = Prover::new(&comp);

    let mut blanks = Blanks::new();

    let mut state = 0;
    let mut steps = 0;
    let mut cycles = 0;
    let mut rulapp = 0;

    let mut result: Option<TermRes> = None;
    let mut last_slot: Option<Slot> = None;

    for cycle in 0..sim_lim {
        match prover.try_rule(cycle as i32, state, &tape) {
            None => {},
            Some(ProverResult::ConfigLimit) => {
                cycles = cycle;
                result = Some(TermRes::cfglim);
                break;
            },
            Some(ProverResult::InfiniteRule) => {
                cycles = cycle;
                result = Some(TermRes::infrul);
                break;
            },
            Some(ProverResult::MultRule) => {
                cycles = cycle;
                result = Some(TermRes::mulrul);
                break;
            },
            Some(ProverResult::Got(rule)) => {
                if let Some(times) = apply_rule(&rule, &mut tape) {
                    // println!("--> applying rule: {:?}", rule);
                    rulapp += times;
                    continue;
                }
            },
        }

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
        rulapp,
        last_slot,
        blanks,
    }
}

/**************************************/

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
        rulapp: 0,
        last_slot,
        blanks,
    }
}

/**************************************/

pub fn quick_term_or_rec(
    comp: &CompProg,
    sim_lim: u32,
    drop_halt: bool,
) -> bool {
    let mut state = 1;

    let mut cycle = 1;

    let mut tape = HeadTape::init_stepped();

    while cycle < sim_lim {
        let reset = 2 * cycle;

        let (mut leftmost, mut rightmost) = (tape.head, tape.head);

        let init_state = state;

        let init_tape = tape.clone();

        while cycle < reset && cycle < sim_lim {
            let Some(&(color, shift, next_state)) =
                comp.get(&(state, tape.scan()))
            else {
                return drop_halt;
            };

            let same = state == next_state;

            if same && tape.at_edge(shift) {
                return true;
            }

            tape.step(shift, color, same);

            let curr = tape.head;

            if curr < leftmost {
                leftmost = curr;
            } else if rightmost < curr {
                rightmost = curr;
            }

            cycle += 1;

            state = next_state;

            if state != init_state {
                continue;
            }

            if tape.aligns_with(&init_tape, leftmost, rightmost) {
                return true;
            }
        }
    }

    false
}

/**************************************/

#[test]
fn test_prover() {
    assert_eq!(
        run_prover("1RB 2LA 1RA 1RA  1LB 1LA 3RB ...", 1000),
        MachineResult {
            result: TermRes::undfnd,
            steps: 36686,
            cycles: 397,
            marks: 2050,
            rulapp: 987,
            blanks: Dict::from([]),
            last_slot: Some((1, 3))
        }
    );

    assert_eq!(
        run_prover("1RB 1LC  1RD 1RB  0RD 0RC  1LD 1LA", 1000),
        MachineResult {
            result: TermRes::spnout,
            steps: 56459,
            cycles: 229,
            marks: 0,
            rulapp: 5073,
            blanks: Dict::from([(2, 56458), (3, 56459)]),
            last_slot: None
        }
    );
}
