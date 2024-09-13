use core::{
    cell::Cell,
    fmt::{Display, Formatter, Result},
    iter::{once, repeat},
};

use crate::instrs::{Color, Shift};

pub type Count = u64;
pub type Counts = (Vec<Count>, Vec<Count>);

/**************************************/

pub trait Block: Display {
    fn new(color: Color, count: Count) -> Self;

    fn get_color(&self) -> Color;
    fn set_color(&mut self, color: Color);

    fn get_count(&self) -> Count;
    fn set_count(&mut self, count: Count);

    fn add_count(&mut self, count: Count);

    fn inc_count(&mut self) {
        self.add_count(1);
    }

    fn dec_count(&mut self);

    fn show(&self, f: &mut Formatter) -> Result {
        let (color, count) = (self.get_color(), self.get_count());

        write!(
            f,
            "{}",
            if count == 1 {
                format!("{color}")
            } else {
                format!("{color}^{count}")
            }
        )
    }
}

#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub struct BasicBlock {
    pub color: Color,
    pub count: Count,
}

impl Block for BasicBlock {
    fn new(color: Color, count: Count) -> Self {
        Self { color, count }
    }

    fn get_color(&self) -> Color {
        self.color
    }

    fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    fn get_count(&self) -> Count {
        self.count
    }

    fn set_count(&mut self, count: Count) {
        self.count = count;
    }

    fn add_count(&mut self, count: Count) {
        self.count += count;
    }

    fn dec_count(&mut self) {
        self.count -= 1;
    }
}

impl Display for BasicBlock {
    fn fmt(&self, f: &mut Formatter) -> Result {
        self.show(f)
    }
}

/**************************************/

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub enum ColorCount {
    Just(Color),
    Mult(Color),
}

impl ColorCount {
    const fn get_color(&self) -> Color {
        match self {
            Self::Just(color) | Self::Mult(color) => *color,
        }
    }
}

impl<B: Block> From<&B> for ColorCount {
    fn from(block: &B) -> Self {
        (if block.get_count() == 1 {
            Self::Just
        } else {
            Self::Mult
        })(block.get_color())
    }
}

#[derive(PartialEq, Eq, Debug, Hash, Clone)]
pub struct Signature {
    pub scan: Color,
    pub lspan: Vec<ColorCount>,
    pub rspan: Vec<ColorCount>,
}

pub trait GetSig {
    fn scan(&self) -> Color;
    fn signature(&self) -> Signature;
}

/**************************************/

#[expect(clippy::partial_pub_fields)]
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct Tape<B: Block> {
    pub scan: Color,

    lspan: Vec<B>,
    rspan: Vec<B>,
}

pub type BasicTape = Tape<BasicBlock>;

impl<B: Block> Display for Tape<B> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "{}",
            self.lspan
                .iter()
                .map(ToString::to_string)
                .rev()
                .chain(once(format!("[{}]", self.scan)))
                .chain(self.rspan.iter().map(ToString::to_string))
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}

impl<B: Block> GetSig for Tape<B> {
    fn scan(&self) -> Color {
        self.scan
    }

    fn signature(&self) -> Signature {
        Signature {
            scan: self.scan,
            lspan: self.lspan.iter().map(Into::into).collect(),
            rspan: self.rspan.iter().map(Into::into).collect(),
        }
    }
}

impl GetSig for EnumTape {
    fn scan(&self) -> Color {
        self.tape.scan
    }

    fn signature(&self) -> Signature {
        self.tape.signature()
    }
}

macro_rules! tape {
    (
        $ scan : expr,
        [ $ ( $ lspan : expr ), * ],
        [ $ ( $ rspan : expr ), * ]
    ) => {
        Tape {
            scan: $ scan,
            lspan: vec! [ $ ( Block::new( $ lspan.0, $ lspan.1) ), * ],
            rspan: vec! [ $ ( Block::new( $ rspan.0, $ rspan.1) ), * ],
        }
    };
}

impl<B: Block> Tape<B> {
    pub const fn init(scan: Color) -> Self {
        tape! { scan, [], [] }
    }

    pub fn init_stepped() -> Self {
        tape! { 0, [(1, 1)], [] }
    }

    pub fn marks(&self) -> Count {
        Count::from(self.scan != 0)
            + self
                .lspan
                .iter()
                .chain(self.rspan.iter())
                .filter(|block| block.get_color() != 0)
                .map(Block::get_count)
                .sum::<Count>()
    }

    pub fn blocks(&self) -> usize {
        self.lspan.len() + self.rspan.len()
    }

    pub fn counts(&self) -> Counts {
        (
            self.lspan.iter().map(B::get_count).collect(),
            self.rspan.iter().map(B::get_count).collect(),
        )
    }

    pub fn span_lens(&self) -> (usize, usize) {
        (self.lspan.len(), self.rspan.len())
    }

    pub fn at_edge(&self, edge: Shift) -> bool {
        self.scan == 0
            && (if edge { &self.rspan } else { &self.lspan }).is_empty()
    }

    pub fn blank(&self) -> bool {
        self.scan == 0 && self.lspan.is_empty() && self.rspan.is_empty()
    }

    pub fn sig_compatible(
        &self,
        Signature { scan, lspan, rspan }: &Signature,
    ) -> bool {
        self.scan == *scan
            && self.lspan.len() >= lspan.len()
            && self.rspan.len() >= rspan.len()
            && self
                .lspan
                .iter()
                .take(lspan.len())
                .zip(lspan.iter())
                .all(|(bk, cc)| bk.get_color() == cc.get_color())
            && self
                .rspan
                .iter()
                .take(rspan.len())
                .zip(rspan.iter())
                .all(|(bk, cc)| bk.get_color() == cc.get_color())
    }

    pub fn unroll(&self) -> Vec<Color> {
        let left_colors = self.lspan.iter().rev().flat_map(|block| {
            repeat(block.get_color()).take(block.get_count() as usize)
        });

        let right_colors = self.rspan.iter().flat_map(|block| {
            repeat(block.get_color()).take(block.get_count() as usize)
        });

        left_colors
            .chain(once(self.scan))
            .chain(right_colors)
            .collect()
    }

    pub fn step(
        &mut self,
        shift: Shift,
        color: Color,
        skip: bool,
    ) -> Count {
        let (pull, push) = if shift {
            (&mut self.rspan, &mut self.lspan)
        } else {
            (&mut self.lspan, &mut self.rspan)
        };

        let mut push_block = (skip
            && !pull.is_empty()
            && pull[0].get_color() == self.scan)
            .then(|| pull.remove(0));

        let stepped = push_block
            .as_ref()
            .map_or_else(|| 1, |block| 1 + block.get_count());

        let next_scan = if pull.is_empty() {
            0
        } else {
            let next_pull = &mut pull[0];

            let pull_color = next_pull.get_color();

            if next_pull.get_count() > 1 {
                next_pull.dec_count();
            } else {
                let mut popped = pull.remove(0);

                if push_block.is_none() {
                    popped.set_count(0);
                    push_block = Some(popped);
                }
            }

            pull_color
        };

        if !push.is_empty() && push[0].get_color() == color {
            push[0].add_count(stepped);
        } else if !push.is_empty() || color != 0 {
            if let Some(block) = &mut push_block {
                block.set_color(color);
                block.inc_count();
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

/**************************************/

pub type Index = (Shift, usize);

pub trait IndexTape {
    fn get_count(&self, index: &Index) -> Count;
    fn set_count(&mut self, index: &Index, val: Count);
}

impl<B: Block> IndexTape for Tape<B> {
    fn get_count(&self, &(side, pos): &Index) -> Count {
        let span = if side { &self.rspan } else { &self.lspan };

        span[pos].get_count()
    }

    fn set_count(&mut self, &(side, pos): &Index, val: Count) {
        let span = if side {
            &mut self.rspan
        } else {
            &mut self.lspan
        };

        span[pos].set_count(val);
    }
}

/**************************************/

pub type Pos = isize;
pub type TapeSlice = Vec<Color>;

#[derive(Clone, PartialEq, Eq)]
pub struct HeadTape {
    head: Pos,
    tape: BasicTape,
}

impl Display for HeadTape {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "({}) {}", self.head, self.tape)
    }
}

impl HeadTape {
    pub fn init_stepped() -> Self {
        Self {
            head: 1,
            tape: tape! { 0, [(1, 1)], [] },
        }
    }

    pub fn at_edge(&self, edge: Shift) -> bool {
        self.tape.at_edge(edge)
    }

    pub fn step(
        &mut self,
        shift: Shift,
        color: Color,
        skip: bool,
    ) -> Count {
        let stepped = self.tape.step(shift, color, skip);

        #[expect(clippy::cast_possible_wrap)]
        if shift {
            self.head += stepped as Pos;
        } else {
            self.head -= stepped as Pos;
        };

        stepped
    }
}

pub trait Alignment {
    fn head(&self) -> Pos;

    fn scan(&self) -> Color {
        unimplemented!()
    }

    #[expect(unused_variables)]
    fn get_slice(&self, start: Pos, ltr: bool) -> TapeSlice {
        unimplemented!()
    }

    fn aligns_with(
        &self,
        prev: &Self,
        leftmost: Pos,
        rightmost: Pos,
    ) -> bool {
        if self.scan() != prev.scan() {
            return false;
        }

        let diff = self.head() - prev.head();

        #[expect(clippy::comparison_chain)]
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

    fn get_ltr(&self, start: Pos) -> TapeSlice {
        self.get_slice(start, true)
    }

    fn get_rtl(&self, start: Pos) -> TapeSlice {
        self.get_slice(start, false)
    }

    fn get_cnt(&self, start: Pos, stop: Pos) -> TapeSlice {
        let head = self.head();

        assert!(start <= head && head <= stop);

        if start == head {
            self.get_ltr(start)
        } else if head == stop {
            self.get_rtl(start)
        } else {
            [self.get_rtl(head - 1), self.get_ltr(head)].concat()
        }
    }
}

impl Alignment for HeadTape {
    fn scan(&self) -> Color {
        self.tape.scan
    }

    fn head(&self) -> Pos {
        self.head
    }

    fn get_slice(&self, start: Pos, ltr: bool) -> TapeSlice {
        let (lspan, rspan, diff) = if ltr {
            (&self.tape.lspan, &self.tape.rspan, self.head() - start)
        } else {
            (&self.tape.rspan, &self.tape.lspan, start - self.head())
        };

        let mut tape = TapeSlice::new();

        if diff > 0 {
            #[expect(clippy::cast_sign_loss)]
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

        tape.push(self.scan());

        for block in rspan {
            tape.extend(vec![block.color; block.count as usize]);
        }

        tape
    }
}

/**************************************/

struct EnumBlock {
    color: Color,
    count: Count,

    index: Option<Index>,
}

impl Block for EnumBlock {
    fn new(color: Color, count: Count) -> Self {
        Self {
            color,
            count,
            index: None,
        }
    }

    fn get_color(&self) -> Color {
        self.color
    }

    fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    fn get_count(&self) -> Count {
        self.count
    }

    fn set_count(&mut self, count: Count) {
        self.count = count;
    }

    fn add_count(&mut self, count: Count) {
        self.count += count;
    }

    fn dec_count(&mut self) {
        self.count -= 1;
    }
}

impl Display for EnumBlock {
    fn fmt(&self, f: &mut Formatter) -> Result {
        self.show(f)
    }
}

pub struct EnumTape {
    tape: Tape<EnumBlock>,

    l_offset: Cell<usize>,
    r_offset: Cell<usize>,

    l_edge: Cell<bool>,
    r_edge: Cell<bool>,
}

impl Display for EnumTape {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.tape)
    }
}

impl From<&BasicTape> for EnumTape {
    fn from(tape: &BasicTape) -> Self {
        let mut lspan = vec![];

        for (i, block) in tape.lspan.iter().enumerate() {
            lspan.push(EnumBlock {
                color: block.get_color(),
                count: block.get_count(),
                index: Some((false, 1 + i)),
            });
        }

        let mut rspan = vec![];

        for (i, block) in tape.rspan.iter().enumerate() {
            rspan.push(EnumBlock {
                color: block.get_color(),
                count: block.get_count(),
                index: Some((true, 1 + i)),
            });
        }

        Self {
            tape: Tape {
                scan: tape.scan,
                lspan,
                rspan,
            },

            l_offset: Cell::new(0),
            r_offset: Cell::new(0),
            l_edge: Cell::new(false),
            r_edge: Cell::new(false),
        }
    }
}

impl EnumTape {
    pub fn offsets(&self) -> (usize, usize) {
        (self.l_offset.get(), self.r_offset.get())
    }

    pub fn edges(&self) -> (bool, bool) {
        (self.l_edge.get(), self.r_edge.get())
    }

    fn touch_edge(&self, shift: Shift) {
        (if shift { &self.r_edge } else { &self.l_edge }).set(true);
    }

    fn check_offsets(&self, block: &EnumBlock) {
        if let Some((side, offset)) = block.index {
            let s_offset =
                if side { &self.r_offset } else { &self.l_offset };

            if offset > s_offset.get() {
                s_offset.set(offset);
            }
        }
    }

    fn check_step(&self, shift: Shift, color: Color, skip: bool) {
        let (pull, push) = if shift {
            (&self.tape.rspan, &self.tape.lspan)
        } else {
            (&self.tape.lspan, &self.tape.rspan)
        };

        if pull.is_empty() {
            self.touch_edge(shift);
        } else {
            let near_block = &pull[0];
            self.check_offsets(near_block);

            if skip && near_block.color == self.tape.scan {
                if pull.len() == 1 {
                    self.touch_edge(shift);
                } else {
                    self.check_offsets(&pull[1]);
                }
            }
        }

        if !push.is_empty() {
            let opp = &push[0];

            if color == opp.color {
                self.check_offsets(opp);
            }
        }
    }

    pub fn step(
        &mut self,
        shift: Shift,
        color: Color,
        skip: bool,
    ) -> Count {
        self.check_step(shift, color, skip);

        self.tape.step(shift, color, skip)
    }
}

impl IndexTape for EnumTape {
    fn get_count(&self, index: &Index) -> Count {
        self.tape.get_count(index)
    }

    fn set_count(&mut self, index: &Index, val: Count) {
        self.tape.set_count(index, val);
    }
}

/**************************************/

#[cfg(test)]
impl BasicTape {
    #[track_caller]
    fn assert(&self, marks: Count, tape_str: &str, sig: &Signature) {
        assert_eq!(self.marks(), marks);
        assert_eq!(self.blank(), marks == 0);

        assert_eq!(self.to_string(), tape_str);

        assert_eq!(self.signature(), *sig);
    }

    #[track_caller]
    fn tstep(&mut self, shift: u8, color: Color, skip: u8) {
        assert!(matches!(shift, 0 | 1));
        assert!(matches!(skip, 0 | 1));

        self.step(shift != 0, color, skip != 0);
    }
}

#[cfg(test)]
macro_rules! sig {
    (
        $ scan : expr,
        [ $ ( $ lspan : tt ), * ],
        [ $ ( $ rspan : tt ), * ]
    ) => {
        Signature {
            scan: $ scan,
            lspan: vec![ $ ( sig!( @_ $ lspan ) ), * ],
            rspan: vec![ $ ( sig!( @_ $ rspan ) ), * ],
        }
    };

    ( @_ [ $ num : expr ] ) => {
        ColorCount::Just( $ num )
    };

    ( @_ $ num : expr ) => {
        ColorCount::Mult( $ num )
    };
}

#[test]
fn test_init() {
    Tape::init(0).assert(0, "[0]", &sig![0, [], []]);

    let mut tape = Tape::init_stepped();

    tape.assert(1, "1 [0]", &sig![0, [[1]], []]);

    tape.tstep(1, 1, 0);

    tape.assert(2, "1^2 [0]", &sig![0, [1], []]);

    tape.tstep(0, 0, 0);

    tape.assert(2, "1 [1]", &sig![1, [[1]], []]);

    tape.tstep(0, 0, 1);

    tape.assert(0, "[0]", &sig![0, [], []]);
}

#[test]
fn test_clone() {
    let tape = tape! { 2, [(1, 1), (0, 1), (1, 1)], [(2, 1), (1, 2)] };

    let mut copy_1 = tape.clone();
    let mut copy_2 = tape.clone();

    copy_1.tstep(0, 2, 0);
    copy_2.tstep(1, 1, 0);

    copy_1.assert(6, "1 0 [1] 2^2 1^2", &sig![1, [[0], [1]], [2, 1]]);
    copy_2.assert(6, "1 0 1^2 [2] 1^2", &sig![2, [1, [0], [1]], [1]]);

    tape.assert(
        6,
        "1 0 1 [2] 2 1^2",
        &sig![2, [[1], [0], [1]], [[2], 1]],
    );
}

#[cfg(test)]
use crate::rules::Op;

#[cfg(test)]
macro_rules! rule {
    (
        $ ( ( $ shift : expr, $ index : expr ) => $ diff : expr ), *
        $ ( , ) *
    ) => {
        Rule::from([
            $ ( (( $ shift == 1, $ index ), Op::Plus( $ diff )) ), *
        ])
    }
}

#[cfg(test)]
use crate::rules::{apply_rule, Rule};

#[test]
fn test_apply_1() {
    let mut tape = tape! {
        3,
        [(1, 12), (2, 3)],
        [(4, 15), (5, 2), (6, 2)]
    };

    tape.assert(
        35,
        "2^3 1^12 [3] 4^15 5^2 6^2",
        &sig![3, [1, 2], [4, 5, 6]],
    );

    apply_rule(
        &rule![
            (0, 1) => 3,
            (1, 0) => -2,
        ],
        &mut tape,
    );

    tape.assert(
        42,
        "2^24 1^12 [3] 4 5^2 6^2",
        &sig![3, [1, 2], [[4], 5, 6]],
    );
}

#[test]
fn test_apply_2() {
    let mut tape = tape! {
        4,
        [(4, 2)],
        [(5, 60), (2, 1), (4, 1), (5, 7), (1, 1)]
    };

    tape.assert(
        73,
        "4^2 [4] 5^60 2 4 5^7 1",
        &sig![4, [4], [5, [2], [4], 5, [1]]],
    );

    apply_rule(
        &rule![
            (0, 0) => 4,
            (1, 0) => -2,
        ],
        &mut tape,
    );

    tape.assert(
        131,
        "4^118 [4] 5^2 2 4 5^7 1",
        &sig![4, [4], [5, [2], [4], 5, [1]]],
    );
}

/**************************************/

#[cfg(test)]
impl EnumTape {
    #[track_caller]
    fn assert(
        &self,
        tape_str: &str,
        offsets: (usize, usize),
        edges: (usize, usize),
    ) {
        assert_eq!(self.to_string(), tape_str);

        assert_eq!(self.offsets(), offsets);

        assert_eq!(self.edges(), {
            let (l_edge, r_edge) = edges;

            assert!(matches!(l_edge, 0 | 1));
            assert!(matches!(r_edge, 0 | 1));

            (l_edge == 1, r_edge == 1)
        });
    }

    #[track_caller]
    fn tstep(&mut self, shift: u8, color: Color, skip: u8) {
        assert!(matches!(shift, 0 | 1));
        assert!(matches!(skip, 0 | 1));

        self.step(shift != 0, color, skip != 0);
    }
}

#[cfg(test)]
macro_rules! enum_tape {
    (
        $ scan : expr,
        [ $ ( $ lspan : expr ), * ],
        [ $ ( $ rspan : expr ), * ]
    ) => {
        EnumTape::from(
            &BasicTape {
                scan: $ scan,
                lspan: vec! [ $ ( BasicBlock::new( $ lspan.0, $ lspan.1) ), * ],
                rspan: vec! [ $ ( BasicBlock::new( $ rspan.0, $ rspan.1) ), * ],
            }
        )
    };
}

#[test]
fn test_offsets_1() {
    let mut tape = enum_tape! {
        0,
        [(1, 11), (4, 1), (3, 11), (2, 1)],
        []
    };

    tape.assert("2 3^11 4 1^11 [0]", (0, 0), (0, 0));

    tape.tstep(0, 0, 0);

    tape.assert("2 3^11 4 1^10 [1]", (1, 0), (0, 0));

    tape.tstep(0, 2, 1);

    tape.assert("2 3^11 [4] 2^11", (2, 0), (0, 0));

    tape.tstep(0, 2, 1);

    tape.assert("2 3^10 [3] 2^12", (3, 0), (0, 0));

    tape.tstep(0, 2, 0);

    tape.assert("2 3^9 [3] 2^13", (3, 0), (0, 0));

    tape.tstep(1, 4, 0);

    tape.assert("2 3^9 4 [2] 2^12", (3, 0), (0, 0));

    tape.tstep(1, 1, 1);

    tape.assert("2 3^9 4 1^13 [0]", (3, 0), (0, 1));

    tape.tstep(1, 1, 0);

    tape.assert("2 3^9 4 1^14 [0]", (3, 0), (0, 1));
}

#[test]
fn test_offsets_2() {
    let mut tape = enum_tape! { 0, [(2, 414_422_565), (3, 6)], [] };

    tape.assert("3^6 2^414422565 [0]", (0, 0), (0, 0));

    tape.tstep(0, 5, 0);

    tape.assert("3^6 2^414422564 [2] 5", (1, 0), (0, 0));

    tape.tstep(0, 5, 1);

    tape.assert("3^5 [3] 5^414422566", (2, 0), (0, 0));

    tape.tstep(1, 2, 0);

    tape.assert("3^5 2 [5] 5^414422565", (2, 0), (0, 0));

    tape.tstep(1, 2, 1);

    tape.assert("3^5 2^414422567 [0]", (2, 0), (0, 1));
}

#[test]
fn test_offsets_3() {
    let mut tape = enum_tape! { 3, [(3, 9)], [(1, 10)] };

    tape.tstep(0, 1, 0);

    tape.assert("3^8 [3] 1^11", (1, 1), (0, 0));
}

#[test]
fn test_edges_1() {
    let mut tape = enum_tape! { 0, [], [] };

    tape.tstep(0, 1, 0);

    tape.assert("[0] 1", (0, 0), (1, 0));
}

#[test]
fn test_edges_2() {
    let mut tape = enum_tape! { 1, [(1, 3)], [] };

    tape.tstep(0, 2, 1);

    tape.assert("[0] 2^4", (1, 0), (1, 0));
}
