use std::collections::HashMap;

use pyo3::prelude::*;

use crate::instrs::{Color, Prog, Shift, State};
use crate::parse::tcompile;
use crate::tape::{Block, Count};

type Step = u64;

type TupleBlock = (Color, Count);
type TupleTape = (Vec<TupleBlock>, Color, Vec<TupleBlock>);

/**************************************/

#[pyclass]
pub struct BackstepMachineHalt {
    comp: Prog,
}

#[pyclass]
pub struct BackstepMachineBlank {
    comp: Prog,
}

#[pyclass]
pub struct BackstepMachineSpinout {
    comp: Prog,
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

        let mut tape = BackstepTape::new(init_tape);

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

            if next_state == -1 {
                return Some(step);
            }

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

        let mut blanks: HashMap<State, Step> = HashMap::new();

        let mut tape = BackstepTape::new(init_tape);

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

            if next_state == -1 {
                break;
            }

            state = next_state;

            if color == 0 && tape.blank() {
                if blanks.contains_key(&state) {
                    break;
                }

                blanks.insert(state, step);

                if state == 0 {
                    break;
                }
            }
        }

        blanks.drain().map(|(_, value)| value).min()
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

        let mut tape = BackstepTape::new(init_tape);

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

            if next_state == -1 {
                break;
            }

            state = next_state;
        }

        None
    }
}

/**************************************/

struct BackstepTape {
    lspan: Vec<Block>,

    pub scan: Color,

    rspan: Vec<Block>,
}

impl From<TupleBlock> for Block {
    fn from((color, count): TupleBlock) -> Self {
        Self::new(color, count)
    }
}

impl BackstepTape {
    fn new(tape: TupleTape) -> Self {
        let (lspan, scan, rspan) = tape;

        Self {
            lspan: lspan.into_iter().map(std::convert::Into::into).collect(),
            scan,
            rspan: rspan.into_iter().map(std::convert::Into::into).collect(),
        }
    }

    fn at_edge(&self, edge: Shift) -> bool {
        self.scan == 0 && (if edge { &self.rspan } else { &self.lspan }).is_empty()
    }

    fn blank(&self) -> bool {
        self.scan == 0 && self.lspan.is_empty() && self.rspan.is_empty()
    }

    fn backstep(&mut self, shift: Shift, color: Color) {
        let _ = self.step(!shift, self.scan, false);

        self.scan = color;
    }

    fn step(&mut self, shift: Shift, color: Color, skip: bool) -> Count {
        let (pull, push) = if shift {
            (&mut self.rspan, &mut self.lspan)
        } else {
            (&mut self.lspan, &mut self.rspan)
        };

        let mut push_block = if skip && !pull.is_empty() && pull[0].color == self.scan {
            Some(pull.remove(0))
        } else {
            None
        };

        let stepped = push_block
            .as_ref()
            .map_or_else(|| 1, |block| 1 + block.count);

        let next_scan: Color;

        if pull.is_empty() {
            next_scan = 0;
        } else {
            let next_pull = &mut pull[0];

            next_scan = next_pull.color;

            if next_pull.count > 1 {
                next_pull.count -= 1;
            } else {
                let mut popped = pull.remove(0);

                if push_block.is_none() {
                    popped.count = 0;
                    push_block = Some(popped);
                }
            }
        }

        if !push.is_empty() && push[0].color == color {
            push[0].count += stepped;
        } else if !push.is_empty() || color != 0 {
            if let Some(block) = &mut push_block {
                block.color = color;
                block.count += 1;
            } else {
                push_block = Some(Block::new(color, 1));
            }

            if let Some(block) = push_block {
                push.insert(0, block);
            }
        }

        self.scan = next_scan;

        stepped
    }
}
