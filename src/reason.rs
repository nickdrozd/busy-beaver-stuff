use std::collections::{HashMap, HashSet};

use pyo3::prelude::*;

use crate::instrs::{Color, CompProg, Instr, Shift, State};
use crate::parse::{parse as prim_parse, tcompile};
use crate::tape::{Tape, TupleTape};

/**************************************/

type Graph = HashMap<State, Vec<State>>;
type Program = HashMap<State, Vec<Instr>>;

fn entry_points(program: &Program) -> Graph {
    let mut exits: HashMap<State, HashSet<State>> = HashMap::new();

    for (state, instrs) in program {
        exits.insert(*state, instrs.iter().map(|instr| instr.2).collect());
    }

    let mut entries: Graph = (0..program.len())
        .map(|state| (state as State, Vec::new()))
        .collect();

    for (state, cons) in exits {
        for exit_point in cons {
            if let Some(states) = entries.get_mut(&exit_point) {
                states.push(state);
            }
        }
    }

    for entr in entries.values_mut() {
        entr.sort_unstable();
    }

    entries
}

#[pyfunction]
pub fn reason_parse(prog: &str) -> (usize, Graph, Program) {
    let mut program = Program::new();

    let parsed = prim_parse(prog);

    for (state, instrs) in parsed.iter().enumerate() {
        program.insert(
            state as State,
            instrs.iter().filter_map(|instr| *instr).collect(),
        );
    }

    (parsed[0].len(), entry_points(&program), program)
}

/**************************************/

type Step = u64;

#[pyclass]
pub struct BackstepMachineHalt {
    comp: CompProg,
}

#[pyclass]
pub struct BackstepMachineBlank {
    comp: CompProg,
    blanks: HashMap<State, Step>,
}

#[pyclass]
pub struct BackstepMachineSpinout {
    comp: CompProg,
}

#[pymethods]
impl BackstepMachineHalt {
    #[new]
    fn new(prog: &str) -> Self {
        Self {
            comp: tcompile(prog),
        }
    }

    fn backstep_run(
        &mut self,
        sim_lim: Step,
        init_tape: TupleTape,
        mut state: State,
        shift: Shift,
        color: Color,
    ) -> Option<Step> {
        let mut step = 0;

        let mut tape = Tape::from_tuples(init_tape);

        tape.backstep(shift, color);

        for _ in 0..sim_lim {
            let Some(&(color, shift, next_state)) = self.comp.get(&(state, tape.scan)) else {
                return Some(step);
            };

            let same = state == next_state;

            if same && tape.at_edge(shift) {
                break;
            }

            let stepped = tape.step(shift, color, same);

            step += stepped;

            state = next_state;
        }

        None
    }
}

#[pymethods]
impl BackstepMachineBlank {
    #[new]
    fn new(prog: &str) -> Self {
        Self {
            comp: tcompile(prog),
            blanks: HashMap::new(),
        }
    }

    fn backstep_run(
        &mut self,
        sim_lim: Step,
        init_tape: TupleTape,
        mut state: State,
        shift: Shift,
        color: Color,
    ) -> Option<Step> {
        let mut step = 0;

        let mut tape = Tape::from_tuples(init_tape);

        tape.backstep(shift, color);

        for _ in 0..sim_lim {
            let Some(&(color, shift, next_state)) = self.comp.get(&(state, tape.scan)) else {
                break;
            };

            let same = state == next_state;

            if same && tape.at_edge(shift) {
                break;
            }

            let stepped = tape.step(shift, color, same);

            step += stepped;

            state = next_state;

            if color == 0 && tape.blank() {
                if self.blanks.contains_key(&state) {
                    break;
                }

                self.blanks.insert(state, step);

                if state == 0 {
                    break;
                }
            }
        }

        self.blanks.drain().map(|(_, value)| value).min()
    }
}

#[pymethods]
impl BackstepMachineSpinout {
    #[new]
    fn new(prog: &str) -> Self {
        Self {
            comp: tcompile(prog),
        }
    }

    fn backstep_run(
        &mut self,
        sim_lim: Step,
        init_tape: TupleTape,
        mut state: State,
        shift: Shift,
        color: Color,
    ) -> Option<Step> {
        let mut step = 0;

        let mut tape = Tape::from_tuples(init_tape);

        tape.backstep(shift, color);

        for _ in 0..sim_lim {
            let Some(&(color, shift, next_state)) = self.comp.get(&(state, tape.scan)) else {
                break;
            };

            let same = state == next_state;

            if same && tape.at_edge(shift) {
                return Some(step);
            }

            let stepped = tape.step(shift, color, same);

            step += stepped;

            state = next_state;
        }

        None
    }
}
