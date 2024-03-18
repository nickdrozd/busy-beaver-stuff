use std::collections::HashMap;

use pyo3::prelude::*;

use crate::instrs::{Slot, State};
use crate::parse::tcompile;
use crate::tape::{Count, Tape};

type Step = u64;

/**************************************/

#[allow(non_camel_case_types)]
#[pyclass]
#[derive(Clone)]
pub enum TermRes {
    xlimit,
    infrul,
    spnout,
    halted,
    undfnd,
}

/**************************************/

#[pyclass]
pub struct MachineResult {
    result: TermRes,

    steps: Step,
    cycles: Step,
    marks: Count,

    blanks: HashMap<State, Step>,

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
        blanks: HashMap<State, Step>,
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
    fn blanks(&self) -> HashMap<State, Step> {
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
            }
            _ => None,
        }
    }

    #[getter]
    const fn simple_termination(&self) -> Option<Step> {
        match self.result {
            TermRes::halted | TermRes::spnout => Some(self.steps),
            _ => None,
        }
    }

    #[getter]
    const fn halted(&self) -> Option<Step> {
        match self.result {
            TermRes::halted => Some(self.steps),
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
pub fn run_machine(prog: &str, sim_lim: Step) -> MachineResult {
    let comp = tcompile(prog);

    let mut tape = Tape::init();

    let mut blanks = HashMap::new();

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

        if next_state == -1 {
            cycles = cycle;
            result = Some(TermRes::halted);
            break;
        }

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
