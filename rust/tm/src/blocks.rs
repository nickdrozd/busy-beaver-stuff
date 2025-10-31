use core::iter::{once, repeat_n};

use crate::{
    Color, Prog, Shift, Steps,
    config::Config,
    tape::{Init, MedSpan as Span, MedTape as Tape, Scan},
};

type UnrolledTape = Vec<Color>;

/**************************************/

impl Prog {
    pub fn opt_block(&self, steps: Steps) -> usize {
        let unrolled = self.unroll_at_max_blocks(steps);

        get_compr(&unrolled)
    }
}

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

    fn unroll(&self) -> UnrolledTape {
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

    steps: Steps,
    max_blocks: usize,

    max_blocks_step: usize,
}

impl Scan for BlockMeasure {
    fn scan(&self) -> Color {
        self.tape.scan()
    }
}

impl Init for BlockMeasure {
    fn init() -> Self {
        Self {
            tape: Tape::init(),

            steps: 0,
            max_blocks: 0,
            max_blocks_step: 0,
        }
    }

    fn init_stepped() -> Self {
        unimplemented!()
    }
}

impl BlockMeasure {
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
    fn unroll_at_max_blocks(&self, steps: Steps) -> UnrolledTape {
        self.run_and_unroll(self.max_block_step(steps))
    }

    fn max_block_step(&self, steps: Steps) -> Steps {
        let mut config: Config<BlockMeasure> = Config::init();

        for _ in 0..steps {
            let Some(&(color, shift, next_state)) =
                self.get(&config.slot())
            else {
                break;
            };

            let same = config.state == next_state;

            if same && config.tape.tape.at_edge(shift) {
                break;
            }

            config.tape.step(shift, color, same);

            config.state = next_state;
        }

        config.tape.max_blocks_step
    }

    fn run_and_unroll(&self, steps: Steps) -> UnrolledTape {
        let mut config: Config<Tape> = Config::init();

        for _ in 0..steps {
            let &(color, shift, next_state) =
                self.get(&config.slot()).unwrap();

            config.tape.step(shift, color, config.state == next_state);

            config.state = next_state;
        }

        config.tape.unroll()
    }
}

fn compr_eff(tape: &UnrolledTape, k: usize) -> usize {
    let mut compr_size = tape.len();

    for i in (0..tape.len() - 2 * k).step_by(k) {
        if tape[i..i + k] == tape[i + k..i + 2 * k] {
            compr_size -= k;
        }
    }

    compr_size
}

fn get_compr(tape: &UnrolledTape) -> usize {
    let mut opt_size = 1;
    let mut min_comp = 1 + tape.len();

    for block_size in 1..tape.len() / 2 {
        let compr_size = compr_eff(tape, block_size);
        if compr_size < min_comp {
            min_comp = compr_size;
            opt_size = block_size;
        }
    }

    opt_size
}
