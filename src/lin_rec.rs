use std::cmp::Ordering;

use pyo3::prelude::*;

use crate::instrs::{Color, Shift};
use crate::parse::tcompile;

type Count = u32;

struct Block {
    color: Color,
    count: Count,
}

type Pos = i32;

struct HeadTape {
    lspan: Vec<Block>,

    scan: Color,

    rspan: Vec<Block>,

    head: Pos,
}

impl HeadTape {
    fn init_stepped() -> Self {
        Self {
            lspan: vec![Block { color: 1, count: 1 }],
            scan: 0,
            rspan: vec![],
            head: 1,
        }
    }

    fn to_ptr(&self) -> PtrTape {
        let init = self.lspan.iter().map(|b| b.count as Pos).sum::<Pos>() - self.head;

        let mut tape = Vec::new();

        for block in self.lspan.iter().rev() {
            tape.extend(std::iter::repeat(block.color).take(block.count as usize));
        }

        tape.push(self.scan);

        for block in &self.rspan {
            tape.extend(std::iter::repeat(block.color).take(block.count as usize));
        }

        PtrTape {
            init,
            scan: self.scan,
            tape,
        }
    }

    fn at_edge(&self, edge: Shift) -> bool {
        self.scan == 0 && (if edge { &self.rspan } else { &self.lspan }).is_empty()
    }

    fn step(&mut self, shift: bool, color: Color, skip: bool) -> Count {
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
        } else {
            if let Some(block) = &mut push_block {
                block.color = color;
                block.count += 1;
            } else {
                push_block = Some(Block { color, count: 1 });
            }

            if !push.is_empty() || color != 0 {
                if let Some(block) = push_block {
                    push.insert(0, block);
                }
            }
        }

        self.scan = next_scan;

        if shift {
            self.head += stepped as Pos;
        } else {
            self.head -= stepped as Pos;
        };

        stepped
    }
}

type TapeSlice = Vec<Color>;

struct PtrTape {
    init: Pos,

    scan: Color,

    tape: TapeSlice,
}

impl PtrTape {
    fn get_ltr(&self, start: Pos) -> TapeSlice {
        let start = start + self.init;
        if start >= 0 {
            self.tape[start as usize..].to_vec()
        } else {
            let mut result = vec![0; -start as usize];
            result.extend_from_slice(&self.tape);
            result
        }
    }

    fn get_rtl(&self, stop: Pos) -> TapeSlice {
        let stop = stop + self.init + 1;
        let tape_len = self.tape.len() as Pos;

        if stop <= tape_len {
            self.tape[..stop as usize].to_vec()
        } else {
            let mut result = self.tape[..].to_vec();
            result.extend(vec![0; (stop - tape_len) as usize]);
            result
        }
    }

    fn get_cnt(&self, start: Pos, stop: Pos) -> TapeSlice {
        let start = start + self.init;
        let stop = stop + self.init + 1;
        let tape_len = self.tape.len() as Pos;

        (start..stop)
            .map(|pos| {
                if pos >= 0 && pos < tape_len {
                    self.tape[pos as usize]
                } else {
                    0
                }
            })
            .collect()
    }
}

#[pyfunction]
pub fn quick_term_or_rec(prog: &str, sim_lim: u32) -> bool {
    let comp = tcompile(prog); // Assume tcompile is defined elsewhere

    let mut state = 1;

    let mut tape = HeadTape::init_stepped();

    let (mut step, mut cycle) = (1, 1);

    while cycle < sim_lim {
        let steps_reset = 2 * step;

        let (mut leftmost, mut rightmost, init_pos) = (tape.head, tape.head, tape.head);

        let init_state = state;

        let init_tape = tape.to_ptr();

        while step < steps_reset && cycle < sim_lim {
            let Some((color, shift, next_state)) = comp[&(state, tape.scan)] else {
                return true;
            };

            let same = state == next_state;

            if same && tape.at_edge(shift) {
                return true;
            }

            let stepped = tape.step(shift, color, same);

            step += stepped;

            cycle += 1;

            state = next_state;
            if state == -1 {
                return true;
            }

            let curr = tape.head;
            if curr < leftmost {
                leftmost = curr;
            } else if rightmost < curr {
                rightmost = curr;
            }

            if state != init_state {
                continue;
            }

            if tape.scan != init_tape.scan {
                continue;
            }

            let ptr = tape.to_ptr();

            let diff = curr - init_pos;

            let (slice1, slice2) = match diff.cmp(&0) {
                Ordering::Greater => (init_tape.get_ltr(leftmost), ptr.get_ltr(leftmost + diff)),
                Ordering::Less => (init_tape.get_rtl(rightmost), ptr.get_rtl(rightmost + diff)),
                Ordering::Equal => (
                    init_tape.get_cnt(leftmost, rightmost),
                    ptr.get_cnt(leftmost, rightmost),
                ),
            };

            if slice1 == slice2 {
                return true;
            }
        }
    }

    false
}
