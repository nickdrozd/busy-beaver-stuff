use core::{
    cell::Cell,
    fmt::{self, Display, Formatter},
    iter::once,
};

use crate::instrs::{Color, Shift};

/**************************************/

pub type BigCount = u64;

type Count = BigCount;

/**************************************/

pub trait Block: Display {
    fn new(color: Color, count: Count) -> Self;

    fn get_color(&self) -> Color;

    fn get_count(&self) -> Count;

    fn add_count(&mut self, count: Count);

    fn decrement(&mut self);

    fn is_single(&self) -> bool {
        self.get_count() == 1
    }

    fn is_indef(&self) -> bool {
        self.get_count() == 0
    }

    fn show(&self, f: &mut Formatter) -> fmt::Result {
        let (color, count) = (self.get_color(), self.get_count());

        write!(
            f,
            "{}",
            match count {
                1 => format!("{color}"),
                0 => format!("{color}.."),
                _ => format!("{color}^{count}"),
            }
        )
    }
}

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct BigBlock {
    pub color: Color,
    pub count: BigCount,
}

impl Block for BigBlock {
    fn new(color: Color, count: Count) -> Self {
        Self { color, count }
    }

    fn get_color(&self) -> Color {
        self.color
    }

    fn get_count(&self) -> Count {
        self.count
    }

    fn add_count(&mut self, count: Count) {
        self.count += count;
    }

    fn decrement(&mut self) {
        self.count -= 1;
    }
}

impl Display for BigBlock {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.show(f)
    }
}

/**************************************/

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Span<B: Block>(pub Vec<B>);

pub type BigSpan = Span<BigBlock>;

impl<B: Block> Span<B> {
    pub const fn new(blocks: Vec<B>) -> Self {
        Self(blocks)
    }

    pub const fn len(&self) -> usize {
        self.0.len()
    }

    const fn blank(&self) -> bool {
        self.0.is_empty()
    }

    fn signature(&self) -> Vec<ColorCount> {
        self.0.iter().map(Into::into).collect()
    }

    pub fn str_iter(&self) -> impl DoubleEndedIterator<Item = String> {
        self.0.iter().map(ToString::to_string)
    }

    pub fn compare_take(&self, prev: &Self, mut take: usize) -> bool {
        let mut s_blocks = self.0.iter();
        let mut p_blocks = prev.0.iter();

        let mut s_next = s_blocks.next();
        let mut p_next = p_blocks.next();

        while take > 0 {
            match (s_next, p_next) {
                (None, None) => return true,
                (None, Some(_)) | (Some(_), None) => return false,
                (Some(s_block), Some(p_block)) => {
                    if s_block.get_color() != p_block.get_color() {
                        return false;
                    }

                    let s_rem = s_block.get_count() as usize;
                    let p_rem = p_block.get_count() as usize;

                    if s_rem == 0 || p_rem == 0 {
                        return false;
                    }

                    let min = take.min(s_rem.min(p_rem));

                    take -= min;

                    if s_rem == min {
                        s_next = s_blocks.next();
                    }

                    if p_rem == min {
                        p_next = p_blocks.next();
                    }
                },
            }
        }

        true
    }

    fn pull(&mut self, scan: Color, skip: bool) -> (Color, Count) {
        let stepped =
            (skip && !self.blank() && self.0[0].get_color() == scan)
                .then(|| self.0.remove(0))
                .map_or_else(|| 1, |block| 1 + block.get_count());

        let next_scan = if self.blank() {
            0
        } else {
            let next_pull = &mut self.0[0];

            let pull_color = next_pull.get_color();

            if next_pull.is_single() {
                self.0.remove(0);
            } else {
                next_pull.decrement();
            }

            pull_color
        };

        (next_scan, stepped)
    }

    pub fn push(&mut self, print: Color, stepped: Count) {
        match self.0.first_mut() {
            Some(block) if block.get_color() == print => {
                block.add_count(stepped);
            },
            None if print == 0 => {},
            _ => {
                self.push_block(print, stepped);
            },
        }
    }

    pub fn push_block(&mut self, color: Color, count: Count) {
        self.0.insert(0, Block::new(color, count));
    }
}

/**************************************/

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub enum ColorCount {
    Just(Color),
    Mult(Color),
}

use ColorCount::*;

impl ColorCount {
    const fn get_color(&self) -> Color {
        match self {
            Just(color) | Mult(color) => *color,
        }
    }
}

impl<B: Block> From<&B> for ColorCount {
    fn from(block: &B) -> Self {
        (if block.is_single() { Just } else { Mult })(block.get_color())
    }
}

type SigSpan = Vec<ColorCount>;

#[derive(PartialEq, Eq, Debug, Hash, Clone)]
pub struct Signature {
    pub scan: Color,
    pub lspan: SigSpan,
    pub rspan: SigSpan,
}

pub trait GetSig {
    fn scan(&self) -> Color;
    fn signature(&self) -> Signature;
}

pub type MinSig = (Signature, (bool, bool));

impl Signature {
    pub fn matches(&self, (other, (lex, rex)): &MinSig) -> bool {
        self.scan == other.scan
            && (if *lex {
                self.lspan == other.lspan
            } else {
                self.lspan.starts_with(&other.lspan)
            })
            && (if *rex {
                self.rspan == other.rspan
            } else {
                self.rspan.starts_with(&other.rspan)
            })
    }
}

/**************************************/

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct Tape<B: Block> {
    pub scan: Color,

    pub lspan: Span<B>,
    pub rspan: Span<B>,
}

pub type BigTape = Tape<BigBlock>;

impl<B: Block> Display for Tape<B> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.lspan
                .str_iter()
                .rev()
                .chain(once(format!("[{}]", self.scan)))
                .chain(self.rspan.str_iter())
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
            lspan: self.lspan.signature(),
            rspan: self.rspan.signature(),
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
            lspan: Span ( vec! [ $ ( Block::new( $ lspan.0, $ lspan.1) ), * ] ),
            rspan: Span ( vec! [ $ ( Block::new( $ rspan.0, $ rspan.1) ), * ] ),
        }
    };
}

impl<B: Block> Tape<B> {
    pub const fn init() -> Self {
        tape! { 0, [], [] }
    }

    pub fn init_stepped() -> Self {
        tape! { 0, [(1, 1)], [] }
    }

    pub const fn at_edge(&self, edge: Shift) -> bool {
        self.scan == 0
            && (if edge { &self.rspan } else { &self.lspan }).blank()
    }

    pub const fn blank(&self) -> bool {
        self.scan == 0 && self.lspan.blank() && self.rspan.blank()
    }
}

pub trait MachineTape {
    fn step(&mut self, shift: Shift, color: Color, skip: bool)
        -> Count;
}

impl<B: Block> MachineTape for Tape<B> {
    fn step(
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

        let (next_scan, stepped) = pull.pull(self.scan, skip);

        push.push(color, stepped);

        self.scan = next_scan;

        stepped
    }
}

/**************************************/

pub type Index = (Shift, usize);

trait IndexBlock: Block {
    fn set_count(&mut self, count: Count);
}

impl IndexBlock for BigBlock {
    fn set_count(&mut self, count: Count) {
        self.count = count;
    }
}

impl IndexBlock for EnumBlock {
    fn set_count(&mut self, count: Count) {
        self.block.set_count(count);
    }
}

pub trait IndexTape {
    fn get_count(&self, index: &Index) -> Count;
    fn set_count(&mut self, index: &Index, val: Count);
}

impl<B: IndexBlock> IndexTape for Tape<B> {
    fn get_count(&self, &(side, pos): &Index) -> Count {
        let span = if side { &self.rspan } else { &self.lspan };

        span.0[pos].get_count()
    }

    fn set_count(&mut self, &(side, pos): &Index, val: Count) {
        let span = if side {
            &mut self.rspan
        } else {
            &mut self.lspan
        };

        span.0[pos].set_count(val);
    }
}

/**************************************/

pub type Pos = isize;

#[derive(Clone, PartialEq, Eq)]
pub struct HeadTape {
    head: Pos,
    tape: BigTape,
}

impl Display for HeadTape {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "({}) {}", self.head, self.tape)
    }
}

impl HeadTape {
    pub fn init_stepped() -> Self {
        Self {
            head: 1,
            tape: BigTape::init_stepped(),
        }
    }

    pub const fn at_edge(&self, edge: Shift) -> bool {
        self.tape.at_edge(edge)
    }

    pub fn step(&mut self, shift: Shift, color: Color, skip: bool) {
        let stepped = self.tape.step(shift, color, skip);

        #[expect(clippy::cast_possible_wrap)]
        if shift {
            self.head += stepped as Pos;
        } else {
            self.head -= stepped as Pos;
        }
    }
}

pub trait Alignment {
    fn head(&self) -> Pos;
    fn scan(&self) -> Color;

    fn l_len(&self) -> usize;
    fn r_len(&self) -> usize;

    fn l_eq(&self, prev: &Self) -> bool;
    fn r_eq(&self, prev: &Self) -> bool;

    fn l_compare_take(&self, prev: &Self, take: usize) -> bool;
    fn r_compare_take(&self, prev: &Self, take: usize) -> bool;

    #[expect(clippy::comparison_chain)]
    fn aligns_with(
        &self,
        prev: &Self,
        leftmost: Pos,
        rightmost: Pos,
    ) -> bool {
        if self.scan() != prev.scan() {
            return false;
        }

        if self.l_len() != prev.l_len() && self.r_len() != prev.r_len()
        {
            return false;
        }

        let p_head = prev.head();

        let (l_take, r_take): (usize, usize) =
            (p_head.abs_diff(leftmost), p_head.abs_diff(rightmost));

        let diff = self.head() - p_head;

        if 0 < diff {
            self.l_compare_take(prev, l_take) && self.r_eq(prev)
        } else if diff < 0 {
            self.r_compare_take(prev, r_take) && self.l_eq(prev)
        } else {
            self.l_compare_take(prev, l_take)
                && self.r_compare_take(prev, r_take)
        }
    }
}

impl Alignment for HeadTape {
    fn head(&self) -> Pos {
        self.head
    }

    fn scan(&self) -> Color {
        self.tape.scan
    }

    fn l_len(&self) -> usize {
        self.tape.lspan.len()
    }

    fn r_len(&self) -> usize {
        self.tape.rspan.len()
    }

    fn l_eq(&self, prev: &Self) -> bool {
        self.tape.lspan == prev.tape.lspan
    }

    fn r_eq(&self, prev: &Self) -> bool {
        self.tape.rspan == prev.tape.rspan
    }

    fn l_compare_take(&self, prev: &Self, take: usize) -> bool {
        self.tape.lspan.compare_take(&prev.tape.lspan, take)
    }

    fn r_compare_take(&self, prev: &Self, take: usize) -> bool {
        self.tape.rspan.compare_take(&prev.tape.rspan, take)
    }
}

/**************************************/

struct EnumBlock {
    block: BigBlock,
    index: Option<Index>,
}

impl Block for EnumBlock {
    fn new(color: Color, count: Count) -> Self {
        Self {
            block: BigBlock::new(color, count),
            index: None,
        }
    }

    fn get_color(&self) -> Color {
        self.block.get_color()
    }

    fn get_count(&self) -> Count {
        self.block.get_count()
    }

    fn add_count(&mut self, count: Count) {
        self.block.add_count(count);
    }

    fn decrement(&mut self) {
        self.block.decrement();
    }
}

impl Display for EnumBlock {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.block.show(f)
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
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.tape)
    }
}

impl From<&BigTape> for EnumTape {
    fn from(tape: &BigTape) -> Self {
        Self {
            tape: Tape {
                scan: tape.scan,
                lspan: Span(
                    tape.lspan
                        .0
                        .iter()
                        .enumerate()
                        .map(|(i, block)| EnumBlock {
                            block: block.clone(),
                            index: Some((false, 1 + i)),
                        })
                        .collect(),
                ),
                rspan: Span(
                    tape.rspan
                        .0
                        .iter()
                        .enumerate()
                        .map(|(i, block)| EnumBlock {
                            block: block.clone(),
                            index: Some((true, 1 + i)),
                        })
                        .collect(),
                ),
            },

            l_offset: Cell::new(0),
            r_offset: Cell::new(0),
            l_edge: Cell::new(false),
            r_edge: Cell::new(false),
        }
    }
}

impl EnumTape {
    const fn offsets(&self) -> (usize, usize) {
        (self.l_offset.get(), self.r_offset.get())
    }

    const fn edges(&self) -> (bool, bool) {
        (self.l_edge.get(), self.r_edge.get())
    }

    fn touch_edge(&self, shift: Shift) {
        (if shift { &self.r_edge } else { &self.l_edge }).set(true);
    }

    fn check_offsets(&self, block: &EnumBlock) {
        let Some((side, offset)) = block.index else {
            return;
        };

        let s_offset =
            if side { &self.r_offset } else { &self.l_offset };

        if offset > s_offset.get() {
            s_offset.set(offset);
        }
    }

    fn check_step(&self, shift: Shift, color: Color, skip: bool) {
        let (pull, push) = if shift {
            (&self.tape.rspan, &self.tape.lspan)
        } else {
            (&self.tape.lspan, &self.tape.rspan)
        };

        if pull.blank() {
            self.touch_edge(shift);
        } else {
            let near_block = &pull.0[0];
            self.check_offsets(near_block);

            if skip && near_block.get_color() == self.tape.scan {
                if pull.len() == 1 {
                    self.touch_edge(shift);
                } else {
                    self.check_offsets(&pull.0[1]);
                }
            }
        }

        if !push.blank() {
            let opp = &push.0[0];

            if color == opp.get_color() {
                self.check_offsets(opp);
            }
        }
    }

    pub fn get_min_sig(&self, sig: &Signature) -> MinSig {
        let (lmax, rmax) = self.offsets();

        (
            Signature {
                scan: sig.scan,
                lspan: sig.lspan[..lmax].to_vec(),
                rspan: sig.rspan[..rmax].to_vec(),
            },
            self.edges(),
        )
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

impl MachineTape for EnumTape {
    fn step(
        &mut self,
        shift: Shift,
        color: Color,
        skip: bool,
    ) -> Count {
        self.check_step(shift, color, skip);

        self.tape.step(shift, color, skip)
    }
}

/**************************************/

impl BigSpan {
    fn marks(&self) -> BigCount {
        self.0
            .iter()
            .filter(|block| block.color != 0)
            .map(|block| block.count)
            .sum::<BigCount>()
    }

    fn counts(&self) -> Vec<Count> {
        self.0.iter().map(BigBlock::get_count).collect()
    }

    fn sig_compatible(&self, span: &SigSpan) -> bool {
        self.0
            .iter()
            .take(span.len())
            .zip(span.iter())
            .all(|(bk, cc)| bk.get_color() == cc.get_color())
    }
}

impl BigTape {
    pub fn marks(&self) -> BigCount {
        BigCount::from(self.scan != 0)
            + self.lspan.marks()
            + self.rspan.marks()
    }

    pub const fn length_one_spans(&self) -> bool {
        self.lspan.len() == 1 && self.rspan.len() == 1
    }
    pub fn counts(&self) -> (Vec<Count>, Vec<Count>) {
        (self.lspan.counts(), self.rspan.counts())
    }

    pub fn sig_compatible(
        &self,
        Signature { scan, lspan, rspan }: &Signature,
    ) -> bool {
        self.scan == *scan
            && self.lspan.len() >= lspan.len()
            && self.rspan.len() >= rspan.len()
            && self.lspan.sig_compatible(lspan)
            && self.rspan.sig_compatible(rspan)
    }
}

/**************************************/

#[cfg(test)]
impl BigTape {
    #[track_caller]
    fn assert(&self, marks: Count, tape_str: &str, sig: &Signature) {
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
        Just( $ num )
    };

    ( @_ $ num : expr ) => {
        Mult( $ num )
    };
}

#[test]
fn test_init() {
    Tape::init().assert(0, "[0]", &sig![0, [], []]);

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
use crate::rules::{ApplyRule as _, Rule};

#[cfg(test)]
macro_rules! rule {
    (
        $ ( ( $ shift : expr, $ index : expr ) => $ diff : expr ), *
        $ ( , ) *
    ) => {
        Rule::from_triples(&[
            $ ( (( $ shift == 1, $ index ), $ diff ) ), *
        ])
    }
}

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

    tape.apply_rule(&rule![
        (0, 1) => 3,
        (1, 0) => -2,
    ]);

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

    tape.apply_rule(&rule![
        (0, 0) => 4,
        (1, 0) => -2,
    ]);

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
            &BigTape {
                scan: $ scan,
                lspan: Span ( vec! [ $ ( Block::new( $ lspan.0, $ lspan.1) ), * ] ),
                rspan: Span ( vec! [ $ ( Block::new( $ rspan.0, $ rspan.1) ), * ] ),
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
