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

        write!(f, "{} [{}] {}", lspan, self.scan, rspan)
    }
}

macro_rules! tape {
    (
        $ scan : expr,
        [ $ ( $ lspan : expr ), * ],
        [ $ ( $ rspan : expr ), * ],
        $head : expr
    ) => {
        Tape {
            scan: $ scan,
            lspan: vec! [ $ ( Block::new( $ lspan.0, $ lspan.1) ), * ],
            rspan: vec! [ $ ( Block::new( $ rspan.0, $ rspan.1) ), * ],
            head: $ head,
        }
    };
}

impl Tape {
    pub fn init(scan: Color) -> Self {
        tape! { scan, [], [], 0 }
    }

    pub fn init_stepped() -> Self {
        tape! { 0, [(1, 1)], [], 1 }
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
impl Tape {
    fn assert_marks(&self, marks: Count) {
        assert_eq!(self.marks(), marks);
    }

    fn assert_tape(&self, tape_str: &str) {
        assert_eq!(self.to_string(), tape_str);
    }

    fn assert_sig(&self, sig: Signature) {
        assert_eq!(self.signature(), sig);
    }

    fn stepp(&mut self, shift: u8, color: Color, skip: u8) {
        assert!(matches!(shift, 0 | 1));
        assert!(matches!(skip, 0 | 1));

        let _ = self.step(shift != 0, color, skip != 0);
    }
}

#[cfg(test)]
macro_rules! sig {
    (
        $ scan : expr,
        [ $ ( $ lspan : expr ), * ],
        [ $ ( $ rspan : expr ), * ]
    ) => {
        Signature {
            scan: $ scan,
            lspan: vec! [ $ ( $ lspan ), * ],
            rspan: vec! [ $ ( $ rspan ), * ],
        }
    };
}

#[test]
fn test_marks() {
    Tape::init(0).assert_marks(0);

    let mut tape = Tape::init_stepped();

    tape.assert_marks(1);

    tape.stepp(1, 1, 0);

    tape.assert_marks(2);

    tape.stepp(0, 0, 0);

    tape.assert_marks(2);

    tape.stepp(0, 0, 1);

    tape.assert_marks(0);
}

#[test]
fn test_sig() {
    let tape = tape! { 2, [(1, 1), (0, 1), (1, 1)], [(2, 1), (1, 2)], 0 };

    tape.assert_marks(6);
    assert!(!tape.blank());

    let just = ColorCount::Just;
    let mult = ColorCount::Mult;

    tape.assert_tape("1^1 0^1 1^1 [2] 2^1 1^2");

    tape.assert_sig(sig! { 2, [just(1), just(0), just(1)], [just(2), mult(1)] });

    let mut copy_1 = tape.clone();
    let mut copy_2 = tape.clone();

    copy_1.stepp(0, 2, 0);
    copy_2.stepp(1, 1, 0);

    copy_1.assert_tape("1^1 0^1 [1] 2^2 1^2");

    copy_1.assert_sig(sig! { 1, [just(0), just(1)], [mult(2), mult(1)] });

    copy_2.assert_tape("1^1 0^1 1^2 [2] 1^2");

    copy_2.assert_sig(sig! { 2, [mult(1), just(0), just(1)], [mult(1)] });

    tape.assert_tape("1^1 0^1 1^1 [2] 2^1 1^2");

    tape.assert_sig(sig! { 2, [just(1), just(0), just(1)], [just(2), mult(1)] });
}
