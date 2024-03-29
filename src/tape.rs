use std::fmt;

use crate::instrs::{Color, Shift};

pub type Count = u64;

/**************************************/

#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub struct Block {
    pub color: Color,
    pub count: Count,
}

impl Block {
    pub const fn new(color: Color, count: Count) -> Self {
        Self { color, count }
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}^{}", self.color, self.count)
    }
}

/**************************************/

#[derive(PartialEq, Eq, Debug)]
pub enum ColorCount {
    Just(Color),
    Mult(Color),
}

impl From<&Block> for ColorCount {
    fn from(block: &Block) -> Self {
        (if block.count == 1 {
            Self::Just
        } else {
            Self::Mult
        })(block.color)
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct Signature {
    scan: Color,
    lspan: Vec<ColorCount>,
    rspan: Vec<ColorCount>,
}

/**************************************/

type Pos = isize;
type TapeSlice = Vec<Color>;

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct Tape {
    pub scan: Color,

    lspan: Vec<Block>,
    rspan: Vec<Block>,

    pub head: Pos,
}

impl fmt::Display for Tape {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let lspan = self
            .lspan
            .iter()
            .rev()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ");

        let rspan = self
            .rspan
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ");

        write!(f, "{} [{} ({})] {}", lspan, self.scan, self.head, rspan)
    }
}

impl Tape {
    pub fn init(scan: Color) -> Self {
        Self {
            scan,
            lspan: vec![],
            rspan: vec![],
            head: 0,
        }
    }

    pub fn init_stepped() -> Self {
        Self {
            scan: 0,
            lspan: vec![Block::new(1, 1)],
            rspan: vec![],
            head: 1,
        }
    }

    pub fn marks(&self) -> Count {
        Count::from(self.scan != 0)
            + self
                .lspan
                .iter()
                .chain(self.rspan.iter())
                .filter(|block| block.color != 0)
                .map(|block| block.count)
                .sum::<Count>()
    }

    #[cfg(test)]
    pub fn signature(&self) -> Signature {
        Signature {
            scan: self.scan,
            lspan: self.lspan.iter().map(std::convert::Into::into).collect(),
            rspan: self.rspan.iter().map(std::convert::Into::into).collect(),
        }
    }
    pub fn at_edge(&self, edge: Shift) -> bool {
        self.scan == 0 && (if edge { &self.rspan } else { &self.lspan }).is_empty()
    }

    pub fn blank(&self) -> bool {
        self.scan == 0 && self.lspan.is_empty() && self.rspan.is_empty()
    }

    pub fn backstep(&mut self, shift: Shift, color: Color) {
        let _ = self.step(!shift, self.scan, false);

        self.scan = color;
    }

    pub fn step(&mut self, shift: Shift, color: Color, skip: bool) -> Count {
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

        if shift {
            self.head += stepped as Pos;
        } else {
            self.head -= stepped as Pos;
        };

        stepped
    }

    pub fn aligns_with(&self, prev: &Self, leftmost: Pos, rightmost: Pos) -> bool {
        let diff = self.head - prev.head;

        #[allow(clippy::comparison_chain)]
        let (slice1, slice2) = if diff > 0 {
            (prev.get_ltr(leftmost), self.get_ltr(leftmost + diff))
        } else if diff < 0 {
            (prev.get_rtl(rightmost), self.get_rtl(rightmost + diff))
        } else {
            (
                prev.get_cnt(leftmost, rightmost),
                self.get_cnt(leftmost, rightmost),
            )
        };

        slice1 == slice2
    }

    fn get_slice(&self, start: Pos, ltr: bool) -> TapeSlice {
        let (lspan, rspan, diff) = if ltr {
            (&self.lspan, &self.rspan, self.head - start)
        } else {
            (&self.rspan, &self.lspan, start - self.head)
        };

        let mut tape = TapeSlice::new();

        if diff > 0 {
            let mut remaining = diff as Count;
            for block in lspan {
                let count = block.count.min(remaining);
                tape.extend(vec![block.color; count as usize]);
                remaining -= count;
            }
            if remaining > 0 {
                tape.extend(vec![0; remaining as usize]);
            }
            tape.reverse();
        }

        tape.push(self.scan);

        for block in rspan {
            tape.extend(vec![block.color; block.count as usize]);
        }

        tape
    }

    fn get_ltr(&self, start: Pos) -> TapeSlice {
        self.get_slice(start, true)
    }

    fn get_rtl(&self, start: Pos) -> TapeSlice {
        self.get_slice(start, false)
    }

    fn get_cnt(&self, start: Pos, stop: Pos) -> TapeSlice {
        assert!(start <= self.head && self.head <= stop);
        if start == self.head {
            self.get_ltr(start)
        } else if stop == self.head {
            self.get_rtl(start)
        } else {
            [self.get_rtl(self.head - 1), self.get_ltr(self.head)].concat()
        }
    }
}

/**************************************/

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_marks(tape: &Tape, marks: Count) {
        assert_eq!(tape.marks(), marks);
    }

    #[test]
    fn test_marks() {
        assert_marks(&Tape::init(0), 0);

        let mut tape = Tape::init_stepped();

        assert_marks(&tape, 1);

        tape.step(true, 1, false);

        assert_marks(&tape, 2);

        tape.step(false, 0, false);

        assert_marks(&tape, 2);

        tape.step(false, 0, true);

        assert_marks(&tape, 0);
    }

    fn assert_tape(tape: &Tape, tape_str: &str) {
        assert_eq!(tape.to_string(), tape_str);
    }

    fn assert_sig(tape: &Tape, sig: Signature) {
        assert_eq!(tape.signature(), sig);
    }

    macro_rules! tape {
        ($scan:expr, [$($lspan:expr),*], [$($rspan:expr),*]) => {
            Tape {
                scan: $scan,
                head: 0,
                lspan: vec![$(Block::new($lspan.0, $lspan.1)),*],
                rspan: vec![$(Block::new($rspan.0, $rspan.1)),*],
            }
        };
    }

    macro_rules! sig {
        ($scan:expr, [$($lspan:expr),*], [$($rspan:expr),*]) => {
            Signature {
                scan: $scan,
                lspan: vec![$($lspan),*],
                rspan: vec![$($rspan),*],
            }
        };
    }

    #[test]
    fn test_sig() {
        let tape = tape! {2, [(1, 1), (0, 1), (1, 1)], [(2, 1), (1, 2)]};

        assert_marks(&tape, 6);
        assert!(!tape.blank());

        let just = ColorCount::Just;
        let mult = ColorCount::Mult;

        assert_tape(&tape, "1^1 0^1 1^1 [2 (0)] 2^1 1^2");

        assert_sig(
            &tape,
            sig! { 2, [just(1), just(0), just(1)], [just(2), mult(1)] },
        );

        let mut copy_1 = tape.clone();
        let mut copy_2 = tape.clone();

        let _ = copy_1.step(false, 2, false);
        let _ = copy_2.step(true, 1, false);

        assert_tape(&copy_1, "1^1 0^1 [1 (-1)] 2^2 1^2");

        assert_sig(&copy_1, sig! { 1, [just(0), just(1)], [mult(2), mult(1)] });

        assert_tape(&copy_2, "1^1 0^1 1^2 [2 (1)] 1^2");

        assert_sig(
            &copy_2,
            sig! {
                2,
                [mult(1), just(0), just(1)],
                [mult(1)]
            },
        );

        assert_tape(&tape, "1^1 0^1 1^1 [2 (0)] 2^1 1^2");

        assert_sig(
            &tape,
            sig! { 2, [just(1), just(0), just(1)], [just(2), mult(1)] },
        );
    }
}
