use pyo3::prelude::*;

use crate::instrs::{Color, Shift};
use crate::parse::comp_thin;
use crate::tape::{Count, Tape};

struct BlockMeasure {
    tape: Tape,

    steps: Count,
    max_blocks: Count,

    max_blocks_step: Count,
}

impl BlockMeasure {
    fn new() -> Self {
        Self {
            tape: Tape::init(0),

            steps: 0,
            max_blocks: 0,
            max_blocks_step: 0,
        }
    }

    fn step(&mut self, shift: Shift, color: Color, skip: bool) {
        self.steps += 1;

        let blocks = self.tape.blocks();

        if blocks > self.max_blocks {
            self.max_blocks = blocks;
            self.max_blocks_step = self.steps;
        }

        let _ = self.tape.step(shift, color, skip);
    }
}

#[pyfunction]
pub fn measure_blocks(prog: &str, steps: Count) -> Option<Count> {
    let comp = comp_thin(prog);

    let mut state = 0;
    let mut tape = BlockMeasure::new();

    for _ in 0..steps {
        let (color, shift, next_state) = *(comp.get(&(state, tape.tape.scan))?);

        let same = state == next_state;

        if same && tape.tape.at_edge(shift) {
            return None;
        }

        tape.step(shift, color, same);

        state = next_state;
    }

    Some(tape.max_blocks_step)
}

#[pyfunction]
pub fn unroll_tape(prog: &str, steps: Count) -> Vec<Color> {
    let comp = comp_thin(prog);

    let mut state = 0;
    let mut tape = Tape::init(0);

    for _ in 0..steps {
        let instr = comp[&(state, tape.scan)];

        let (color, shift, next_state) = instr;

        tape.step(shift, color, state == next_state);

        state = next_state;
    }

    tape.unroll()
}
