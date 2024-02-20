use std::collections::HashMap;

use pyo3::prelude::*;

use crate::instrs::{Color, Prog, Shift, State};
use crate::parse::tcompile;
use crate::tape::{Block, Count};

type Step = u64;

#[pyclass]
pub struct BackstepMachine {
    comp: Prog,

    #[pyo3(get, set)]
    blanks: HashMap<State, Step>,

    #[pyo3(get, set)]
    halted: Option<Step>,

    #[pyo3(get, set)]
    spnout: Option<Step>,

    #[pyo3(get, set)]
    undfnd: Option<Step>,
}

type TupleBlock = (Color, Count);
type TupleTape = (Vec<TupleBlock>, Color, Vec<TupleBlock>);

#[pymethods]
impl BackstepMachine {
    #[new]
    fn new(prog: &str) -> Self {
        Self {
            comp: tcompile(prog),

            blanks: HashMap::new(),

            halted: None,
            spnout: None,
            undfnd: None,
        }
    }

    fn backstep_run(
        &mut self,
        sim_lim: Step,
        init_tape: TupleTape,
        mut state: State,
        shift: Shift,
        color: Color,
    ) {
        let mut step = 0;

        self.blanks.clear();

        let mut tape = BackstepTape::new(init_tape);

        tape.backstep(shift, color);

        for _ in 0..sim_lim {
            let Some(&(color, shift, next_state)) = self.comp.get(&(state, tape.scan)) else {
                self.undfnd = Some(step);
                break;
            };

            let same = state == next_state;

            if same && tape.at_edge(shift) {
                self.spnout = Some(step);
                break;
            }

            let stepped = tape.step(shift, color, same);

            step += stepped;

            if next_state == -1 {
                self.halted = Some(step);
                break;
            }

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
    }
}

struct BackstepTape {
    lspan: Vec<Block>,

    pub scan: Color,

    rspan: Vec<Block>,
}

impl From<TupleBlock> for Block {
    fn from((color, count): TupleBlock) -> Self {
        Self { color, count }
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
                push_block = Some(Block { color, count: 1 });
            }

            if let Some(block) = push_block {
                push.insert(0, block);
            }
        }

        self.scan = next_scan;

        stepped
    }
}
