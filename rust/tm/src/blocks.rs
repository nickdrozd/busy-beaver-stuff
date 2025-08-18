use core::iter::{once, repeat_n};

use crate::{
    instrs::{Color, Shift},
    prog::Prog,
    tape::{MachineTape as _, MedSpan as Span, MedTape as Tape},
};

/**************************************/

impl Span {
    fn unroll(&self) -> impl DoubleEndedIterator<Item = Color> {
        self.iter().flat_map(|block| {
            repeat_n(block.color, block.count as usize)
        })
    }
}

impl Tape {
    const fn blocks(&self) -> usize {
        self.lspan.len() + self.rspan.len()
    }

    fn unroll(&self) -> Vec<Color> {
        self.lspan
            .unroll()
            .rev()
            .chain(once(self.scan))
            .chain(self.rspan.unroll())
            .collect()
    }
}

/**************************************/

struct BlockMeasure {
    tape: Tape,

    steps: usize,
    max_blocks: usize,

    max_blocks_step: usize,
}

impl BlockMeasure {
    const fn new() -> Self {
        Self {
            tape: Tape::init(),

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

        self.tape.step(shift, color, skip);
    }
}

/**************************************/

impl Prog {
    pub fn opt_block(&self, steps: usize) -> usize {
        let max_blocks_step = self.measure_blocks(steps);

        let tape = self.unroll_tape(max_blocks_step);

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

    fn measure_blocks(&self, steps: usize) -> usize {
        let mut state = 0;
        let mut tape = BlockMeasure::new();

        for _ in 0..steps {
            let Some(&(color, shift, next_state)) =
                self.get((state, tape.tape.scan))
            else {
                break;
            };

            let same = state == next_state;

            if same && tape.tape.at_edge(shift) {
                break;
            }

            tape.step(shift, color, same);

            state = next_state;
        }

        tape.max_blocks_step
    }

    fn unroll_tape(&self, steps: usize) -> Vec<Color> {
        let mut state = 0;
        let mut tape = Tape::init();

        for _ in 0..steps {
            let &(color, shift, next_state) =
                self.get((state, tape.scan)).unwrap();

            tape.step(shift, color, state == next_state);

            state = next_state;
        }

        tape.unroll()
    }
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
