use pyo3::pyfunction;

use crate::{
    instrs::{Color, CompProg, Shift},
    parse::tcompile,
    tape::{BasicTape, Count},
};

struct BlockMeasure {
    tape: BasicTape,

    steps: Count,
    max_blocks: Count,

    max_blocks_step: Count,
}

impl BlockMeasure {
    const fn new() -> Self {
        Self {
            tape: BasicTape::init(0),

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

fn measure_blocks(comp: &CompProg, steps: Count) -> Option<Count> {
    let mut state = 0;
    let mut tape = BlockMeasure::new();

    for _ in 0..steps {
        let (color, shift, next_state) =
            *(comp.get(&(state, tape.tape.scan))?);

        let same = state == next_state;

        if same && tape.tape.at_edge(shift) {
            return None;
        }

        tape.step(shift, color, same);

        state = next_state;
    }

    Some(tape.max_blocks_step)
}

fn unroll_tape(comp: &CompProg, steps: Count) -> Vec<Color> {
    let mut state = 0;
    let mut tape = BasicTape::init(0);

    for _ in 0..steps {
        let instr = comp[&(state, tape.scan)];

        let (color, shift, next_state) = instr;

        tape.step(shift, color, state == next_state);

        state = next_state;
    }

    tape.unroll()
}

fn compr_eff(tape: &[Color], k: usize) -> usize {
    let mut compr_size = tape.len();

    for i in (0..tape.len() - 2 * k).step_by(k) {
        if tape[i..i + k] == tape[i + k..i + 2 * k] {
            compr_size -= k;
        }
    }

    compr_size
}

#[pyfunction]
pub fn opt_block(prog: &str, steps: Count) -> usize {
    let comp = tcompile(prog);

    let Some(max_blocks_step) = measure_blocks(&comp, steps) else {
        return 1;
    };

    let tape = unroll_tape(&comp, max_blocks_step);

    let mut opt_size = 1;
    let mut min_comp = 1 + tape.len();

    for block_size in 1..tape.len() / 2 {
        let compr_size = compr_eff(&tape, block_size);
        if compr_size < min_comp {
            min_comp = compr_size;
            opt_size = block_size;
        }
    }

    opt_size
}
