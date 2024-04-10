use pyo3::prelude::*;

use crate::instrs::{Color, Shift};
use crate::parse::comp_thin;
use crate::tape::{Block, Count};

struct BlockMeasure {
    lspan: Vec<Block>,

    scan: Color,

    rspan: Vec<Block>,

    steps: Count,
    max_blocks: Count,

    max_blocks_step: Count,
}

impl BlockMeasure {
    fn new() -> Self {
        Self {
            lspan: vec![],
            scan: 0,
            rspan: vec![],

            steps: 0,
            max_blocks: 0,
            max_blocks_step: 0,
        }
    }

    fn blocks(&self) -> Count {
        (self.lspan.len() + self.rspan.len()) as Count
    }

    fn at_edge(&self, edge: Shift) -> bool {
        self.scan == 0 && (if edge { &self.rspan } else { &self.lspan }).is_empty()
    }

    fn step(&mut self, shift: Shift, color: Color, skip: bool) {
        self.steps += 1;

        let blocks = self.blocks();

        if blocks > self.max_blocks {
            self.max_blocks = blocks;
            self.max_blocks_step = self.steps;
        }

        let (pull, push) = if shift {
            (&mut self.rspan, &mut self.lspan)
        } else {
            (&mut self.lspan, &mut self.rspan)
        };

        let mut push_block =
            (skip && !pull.is_empty() && pull[0].color == self.scan).then(|| pull.remove(0));

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
    }
}

#[pyfunction]
pub fn measure_blocks(prog: &str, steps: Count) -> Option<Count> {
    let comp = comp_thin(prog);

    let mut state = 0;
    let mut tape = BlockMeasure::new();

    for _ in 0..steps {
        let (color, shift, next_state) = *(comp.get(&(state, tape.scan))?);

        let same = state == next_state;

        if same && tape.at_edge(shift) {
            return None;
        }

        tape.step(shift, color, same);

        state = next_state;
    }

    Some(tape.max_blocks_step)
}

struct QuickTape {
    lspan: Vec<Block>,
    scan: Color,
    rspan: Vec<Block>,
}

impl QuickTape {
    fn new() -> Self {
        Self {
            lspan: vec![],
            scan: 0,
            rspan: vec![],
        }
    }

    fn unroll(&self) -> Vec<Color> {
        let left_colors = self
            .lspan
            .iter()
            .rev()
            .flat_map(|block| std::iter::repeat(block.color).take(block.count as usize));

        let right_colors = self
            .rspan
            .iter()
            .flat_map(|block| std::iter::repeat(block.color).take(block.count as usize));

        left_colors
            .chain(std::iter::once(self.scan))
            .chain(right_colors)
            .collect()
    }

    fn step(&mut self, shift: Shift, color: Color, skip: bool) {
        let (pull, push) = if shift {
            (&mut self.rspan, &mut self.lspan)
        } else {
            (&mut self.lspan, &mut self.rspan)
        };

        let mut push_block =
            (skip && !pull.is_empty() && pull[0].color == self.scan).then(|| pull.remove(0));

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
    }
}

#[pyfunction]
pub fn unroll_tape(prog: &str, steps: Count) -> Vec<Color> {
    let comp = comp_thin(prog);

    let mut state = 0;
    let mut tape = QuickTape::new();

    for _ in 0..steps {
        let instr = comp[&(state, tape.scan)];

        let (color, shift, next_state) = instr;

        tape.step(shift, color, state == next_state);

        state = next_state;
    }

    tape.unroll()
}
