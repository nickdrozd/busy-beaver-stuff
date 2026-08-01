use core::{
    cell::Cell,
    fmt::{self, Debug, Display, Formatter},
    hash::{Hash, Hasher},
    iter::{Sum, once},
    ops::{AddAssign, Index as IndexTrait, IndexMut, SubAssign},
};

use std::{collections::hash_map::DefaultHasher, sync::Arc};

use num_bigint::BigUint;
use num_traits::{One, ToPrimitive, Zero};

use crate::{Color, Shift};

/**************************************/

pub type LilCount = u8;
pub type MedCount = u16;
pub type BigCount = u64;
pub type AlgCount = BigUint;

/**************************************/

pub trait Countable:
    AddAssign
    + Clone
    + Display
    + PartialEq
    + Eq
    + From<bool>
    + ToPrimitive
    + One
    + SubAssign
    + Sum
    + Zero
{
}

impl Countable for LilCount {}
impl Countable for MedCount {}
impl Countable for BigCount {}
impl Countable for AlgCount {}

/**************************************/

pub trait Block: Clone + PartialEq + Eq + Display {
    type Count: Countable;

    fn new(color: Color, count: Self::Count) -> Self;

    fn get_color(&self) -> Color;

    fn get_count(&self) -> &Self::Count;

    fn add_count(&mut self, count: Self::Count);

    fn set_count(&mut self, count: Self::Count);

    fn decrement(&mut self);

    fn is_single(&self) -> bool {
        self.get_count().is_one()
    }

    fn is_indef(&self) -> bool {
        self.get_count().is_zero()
    }

    fn blank(&self) -> bool {
        self.get_color() == 0
    }
}

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct BasicBlock<Count> {
    pub color: Color,
    pub count: Count,
}

pub type LilBlock = BasicBlock<LilCount>;
pub type MedBlock = BasicBlock<MedCount>;
pub type BigBlock = BasicBlock<BigCount>;
pub type AlgBlock = BasicBlock<AlgCount>;

impl<Count: Countable> Block for BasicBlock<Count> {
    type Count = Count;

    fn new(color: Color, count: Count) -> Self {
        Self { color, count }
    }

    fn get_color(&self) -> Color {
        self.color
    }

    fn get_count(&self) -> &Count {
        &self.count
    }

    fn add_count(&mut self, count: Count) {
        self.count += count;
    }

    fn decrement(&mut self) {
        self.count -= Count::one();
    }

    fn set_count(&mut self, count: Count) {
        self.count = count;
    }
}

impl<Count: Countable> Display for BasicBlock<Count> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let (color, count) = (self.get_color(), self.get_count());

        let fmt = match count {
            c if c.is_one() => format!("{color}"),
            c if c.is_zero() => format!("{color}.."),
            _ => format!("{color}^{count}"),
        };

        write!(f, "{fmt}")
    }
}

/**************************************/

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Span<B: Block> {
    blocks: Vec<B>,
}

impl<B: Block> Span<B> {
    pub const fn new(blocks: Vec<B>) -> Self {
        Self { blocks }
    }

    pub const fn init_blank() -> Self {
        Self::new(vec![])
    }

    pub fn init_stepped() -> Self {
        Self::new(vec![B::new(1, B::Count::one())])
    }

    pub const fn len(&self) -> usize {
        self.blocks.len()
    }

    pub const fn blank(&self) -> bool {
        self.blocks.is_empty()
    }

    pub fn iter(&self) -> impl DoubleEndedIterator<Item = &B> {
        self.blocks.iter().rev()
    }

    pub fn str_iter(&self) -> impl DoubleEndedIterator<Item = String> {
        self.iter().map(ToString::to_string)
    }

    fn marks(&self) -> B::Count {
        self.iter()
            .filter(|block| !block.blank())
            .map(|block| block.get_count().clone())
            .sum()
    }

    pub fn push_block(&mut self, color: Color, count: B::Count) {
        self.blocks.push(Block::new(color, count));
    }

    pub fn pop_block(&mut self) -> B {
        self.blocks.pop().unwrap()
    }

    pub fn first(&self) -> Option<&B> {
        self.blocks.last()
    }

    pub fn first_mut(&mut self) -> Option<&mut B> {
        self.blocks.last_mut()
    }

    const fn last_pos(&self) -> usize {
        self.blocks.len() - 1
    }

    fn pull(&mut self, scan: Color, skip: bool) -> (Color, B::Count) {
        let stepped = (skip
            && self
                .first()
                .is_some_and(|block| block.get_color() == scan))
        .then(|| self.pop_block())
        .map_or_else(B::Count::one, |block| {
            B::Count::one() + block.get_count().clone()
        });

        let next_scan = if self.blank() {
            0
        } else {
            let next_pull = &mut self[0];

            let pull_color = next_pull.get_color();

            if next_pull.is_single() {
                self.pop_block();
            } else {
                next_pull.decrement();
            }

            pull_color
        };

        (next_scan, stepped)
    }

    fn push(&mut self, print: Color, stepped: &B::Count) {
        match self.first_mut() {
            Some(block) if block.get_color() == print => {
                block.add_count(stepped.clone());
            },
            None if print == 0 => {},
            _ => {
                self.push_block(print, stepped.clone());
            },
        }
    }

    fn counts(&self) -> Vec<B::Count> {
        self.iter().map(|block| block.get_count().clone()).collect()
    }

    fn signature(&self) -> Vec<ColorCount> {
        self.iter().map(Into::into).collect()
    }

    fn sig_compatible(&self, span: &SigSpan) -> bool {
        self.iter()
            .take(span.len())
            .zip(span.iter())
            .all(|(bk, cc)| bk.get_color() == cc.get_color())
    }
}

impl<B: Block> IndexTrait<usize> for Span<B> {
    type Output = B;

    fn index(&self, pos: usize) -> &Self::Output {
        &self.blocks[self.last_pos() - pos]
    }
}

impl<B: Block> IndexMut<usize> for Span<B> {
    fn index_mut(&mut self, pos: usize) -> &mut Self::Output {
        let last_pos = self.last_pos();

        &mut self.blocks[last_pos - pos]
    }
}

pub type MedSpan = Span<MedBlock>;

#[expect(clippy::multiple_inherent_impl)]
impl<B: Block> Span<B> {
    pub fn compare_take(&self, prev: &Self, mut take: usize) -> bool {
        let mut s_blocks = self.iter();
        let mut p_blocks = prev.iter();

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

                    let s_rem = s_block.get_count().to_usize().unwrap();
                    let p_rem = p_block.get_count().to_usize().unwrap();

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
}

/**************************************/

#[derive(PartialEq, Eq, Debug, Clone, Hash)]
pub enum ColorCount {
    Just(Color),
    Mult(Color),
}

use ColorCount::*;

impl ColorCount {
    pub(crate) const fn get_color(&self) -> Color {
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

#[derive(PartialEq, Eq, Hash, Debug)]
pub struct Signature {
    pub scan: Color,
    pub lspan: SigSpan,
    pub rspan: SigSpan,
}

pub trait GetSig: Scan {
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

pub type LilTape = Tape<LilBlock>;
pub type MedTape = Tape<MedBlock>;
pub type BigTape = Tape<BigBlock>;
pub type AlgTape = Tape<AlgBlock>;

impl From<&MedTape> for BigTape {
    fn from(tape: &MedTape) -> Self {
        fn convert_span(span: &MedSpan) -> Span<BigBlock> {
            Span::new(
                span.iter()
                    .rev()
                    .map(|block| {
                        BigBlock::new(
                            block.color,
                            BigCount::from(block.count),
                        )
                    })
                    .collect(),
            )
        }

        Self {
            scan: tape.scan,
            lspan: convert_span(&tape.lspan),
            rspan: convert_span(&tape.rspan),
        }
    }
}

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
    fn signature(&self) -> Signature {
        Signature {
            scan: self.scan(),
            lspan: self.lspan.signature(),
            rspan: self.rspan.signature(),
        }
    }
}

impl Scan for EnumTape {
    fn scan(&self) -> Color {
        self.tape.scan
    }
}

impl GetSig for EnumTape {
    fn signature(&self) -> Signature {
        self.tape.signature()
    }
}

impl<B: Block> Tape<B> {
    pub const fn at_edge(&self, edge: Shift) -> bool {
        self.scan == 0
            && (if edge { &self.rspan } else { &self.lspan }).blank()
    }

    pub const fn blank(&self) -> bool {
        self.scan == 0 && self.lspan.blank() && self.rspan.blank()
    }

    pub fn step(
        &mut self,
        shift: Shift,
        color: Color,
        skip: bool,
    ) -> B::Count {
        let (pull, push) = if shift {
            (&mut self.rspan, &mut self.lspan)
        } else {
            (&mut self.lspan, &mut self.rspan)
        };

        let (next_scan, stepped) = pull.pull(self.scan, skip);

        push.push(color, &stepped);

        self.scan = next_scan;

        stepped
    }

    pub fn marks(&self) -> B::Count {
        B::Count::from(self.scan != 0)
            + self.lspan.marks()
            + self.rspan.marks()
    }

    pub const fn length_one_spans(&self) -> bool {
        self.lspan.len() == 1 && self.rspan.len() == 1
    }

    pub const fn blocks(&self) -> usize {
        self.lspan.len() + self.rspan.len()
    }

    pub fn counts(&self) -> (Vec<B::Count>, Vec<B::Count>) {
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

pub trait MachineTape {
    fn mstep(&mut self, shift: Shift, color: Color, skip: bool);
}

impl<B: Block> MachineTape for Tape<B> {
    fn mstep(&mut self, shift: Shift, color: Color, skip: bool) {
        self.step(shift, color, skip);
    }
}

pub trait Scan {
    fn scan(&self) -> Color;
}

impl<B: Block> Scan for Tape<B> {
    fn scan(&self) -> Color {
        self.scan
    }
}

pub trait Init {
    fn init() -> Self;
    fn init_stepped() -> Self;
}

impl<B: Block> Init for Tape<B> {
    fn init() -> Self {
        Self {
            scan: 0,
            lspan: Span::init_blank(),
            rspan: Span::init_blank(),
        }
    }

    fn init_stepped() -> Self {
        Self {
            scan: 0,
            lspan: Span::init_stepped(),
            rspan: Span::init_blank(),
        }
    }
}

/**************************************/

pub type Index = (Shift, usize);

pub trait IndexTape<Count: Countable> {
    fn get_count(&self, index: &Index) -> &Count;
    fn set_count(&mut self, index: &Index, val: Count);
}

impl<B: Block> IndexTape<B::Count> for Tape<B> {
    fn get_count(&self, &(side, pos): &Index) -> &B::Count {
        let span = if side { &self.rspan } else { &self.lspan };

        span[pos].get_count()
    }

    fn set_count(&mut self, &(side, pos): &Index, val: B::Count) {
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

pub struct HeadTape<'t, B: Block> {
    head: Pos,
    tape: &'t Tape<B>,
}

impl<'t, B: Block> HeadTape<'t, B> {
    pub const fn new(head: Pos, tape: &'t Tape<B>) -> Self {
        Self { head, tape }
    }
}

pub enum LinRec {
    Stationary,
    Translated,
}

impl<B: Block> HeadTape<'_, B> {
    pub fn aligns_with(
        &self,
        prev: &Self,
        leftmost: Pos,
        rightmost: Pos,
    ) -> Option<LinRec> {
        if self.tape.scan != prev.tape.scan {
            return None;
        }

        if self.tape.lspan.len() != prev.tape.lspan.len()
            && self.tape.rspan.len() != prev.tape.rspan.len()
        {
            return None;
        }

        let (l_take, r_take): (usize, usize) = (
            prev.head.abs_diff(leftmost),
            prev.head.abs_diff(rightmost),
        );

        let diff = self.head - prev.head;

        #[expect(clippy::comparison_chain)]
        if 0 < diff {
            (self.tape.lspan.compare_take(&prev.tape.lspan, l_take)
                && self.tape.rspan == prev.tape.rspan)
                .then_some(LinRec::Translated)
        } else if diff < 0 {
            (self.tape.rspan.compare_take(&prev.tape.rspan, r_take)
                && self.tape.lspan == prev.tape.lspan)
                .then_some(LinRec::Translated)
        } else {
            (self.tape.lspan.compare_take(&prev.tape.lspan, l_take)
                && self
                    .tape
                    .rspan
                    .compare_take(&prev.tape.rspan, r_take))
            .then_some(LinRec::Stationary)
        }
    }
}

/**************************************/

#[derive(Clone, PartialEq, Eq)]
struct EnumBlock {
    block: AlgBlock,
    index: Option<Index>,
}

impl Block for EnumBlock {
    type Count = AlgCount;

    fn new(color: Color, count: Self::Count) -> Self {
        Self {
            block: AlgBlock::new(color, count),
            index: None,
        }
    }

    fn get_color(&self) -> Color {
        self.block.get_color()
    }

    fn get_count(&self) -> &Self::Count {
        self.block.get_count()
    }

    fn add_count(&mut self, count: Self::Count) {
        self.block.add_count(count);
    }

    fn decrement(&mut self) {
        self.block.decrement();
    }

    fn set_count(&mut self, count: Self::Count) {
        self.block.set_count(count);
    }
}

impl Display for EnumBlock {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.block)
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

type AlgSpan = Span<AlgBlock>;
type EnumSpan = Span<EnumBlock>;

impl EnumSpan {
    fn from(span: &AlgSpan, side: Shift) -> Self {
        let len = span.len();

        Self::new(
            span.iter()
                .rev()
                .enumerate()
                .map(|(i, block)| EnumBlock {
                    block: block.clone(),
                    index: Some((side, len - i)),
                })
                .collect(),
        )
    }
}

impl From<&AlgTape> for EnumTape {
    fn from(tape: &AlgTape) -> Self {
        Self {
            tape: Tape {
                scan: tape.scan,
                lspan: EnumSpan::from(&tape.lspan, false),
                rspan: EnumSpan::from(&tape.rspan, true),
            },

            l_offset: 0.into(),
            r_offset: 0.into(),
            l_edge: false.into(),
            r_edge: false.into(),
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
            let near_block = &pull[0];
            self.check_offsets(near_block);

            if skip && near_block.get_color() == self.tape.scan {
                if pull.len() == 1 {
                    self.touch_edge(shift);
                } else {
                    self.check_offsets(&pull[1]);
                }
            }
        }

        if !push.blank() {
            let opp = &push[0];

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

impl IndexTape<AlgCount> for EnumTape {
    fn get_count(&self, index: &Index) -> &AlgCount {
        self.tape.get_count(index)
    }

    fn set_count(&mut self, index: &Index, val: AlgCount) {
        self.tape.set_count(index, val);
    }
}

impl MachineTape for EnumTape {
    fn mstep(&mut self, shift: Shift, color: Color, skip: bool) {
        self.check_step(shift, color, skip);

        self.tape.step(shift, color, skip);
    }
}

/**************************************/
/* Dynamic prover tape               */
/**************************************/

const DYNAMIC_REBALANCE_WINDOW: usize = 64;
const DYNAMIC_MAX_PATTERN: usize = 16;
const DYNAMIC_MIN_PATTERN_REPEATS: usize = 3;

pub type DynamicWord = Arc<[Color]>;

fn dynamic_word_hash(word: &[Color]) -> u64 {
    let mut hasher = DefaultHasher::new();
    word.hash(&mut hasher);
    hasher.finish()
}

fn dynamic_primitive(word: &[Color]) -> (&[Color], usize) {
    for width in 1..=word.len() / 2 {
        if !word.len().is_multiple_of(width) {
            continue;
        }

        let root = &word[..width];
        if word.chunks_exact(width).all(|chunk| chunk == root) {
            return (root, word.len() / width);
        }
    }

    (word, 1)
}

fn dynamic_word_display(word: &[Color], reverse: bool) -> String {
    let shown: Vec<String> = if reverse {
        word.iter().rev().map(ToString::to_string).collect()
    } else {
        word.iter().map(ToString::to_string).collect()
    };

    if shown.len() == 1 {
        shown[0].clone()
    } else {
        format!("({})", shown.join(" "))
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct DynamicBlock {
    word: DynamicWord,
    count: AlgCount,
    origins: Vec<Index>,
    word_hash: u64,
}

impl DynamicBlock {
    fn new(color: Color, count: AlgCount) -> Self {
        Self::new_word(&[color], count)
    }

    fn new_word(word: &[Color], count: AlgCount) -> Self {
        assert!(
            !word.is_empty(),
            "a tape block cannot have an empty word"
        );

        let (root, factor) = dynamic_primitive(word);
        let count = if factor == 1 {
            count
        } else {
            count * AlgCount::from(factor)
        };
        #[expect(clippy::shadow_unrelated)]
        let word = DynamicWord::from(root.to_vec());
        let word_hash = dynamic_word_hash(&word);

        Self {
            word,
            count,
            origins: vec![],
            word_hash,
        }
    }

    fn fragment(&self, word: &[Color], count: AlgCount) -> Self {
        let mut block = Self::new_word(word, count);
        block.origins.clone_from(&self.origins);
        block
    }

    fn first(&self) -> Color {
        self.word[0]
    }

    fn width(&self) -> usize {
        self.word.len()
    }

    fn homogeneous(&self) -> bool {
        self.word.len() == 1
    }

    fn is_single(&self) -> bool {
        self.count.is_one()
    }

    fn is_indef(&self) -> bool {
        self.count.is_zero()
    }

    fn blank(&self) -> bool {
        self.word.iter().all(|&color| color == 0)
    }

    fn marked_width(&self) -> usize {
        self.word.iter().filter(|&&color| color != 0).count()
    }

    fn merge_origins(&mut self, other: &Self) {
        for origin in &other.origins {
            if !self.origins.contains(origin) {
                self.origins.push(*origin);
            }
        }
    }

    fn normalize_word(&mut self) {
        let (root, factor) = dynamic_primitive(&self.word);
        if factor == 1 {
            return;
        }

        let root = root.to_vec();
        self.word = DynamicWord::from(root);
        self.word_hash = dynamic_word_hash(&self.word);
        self.count *= AlgCount::from(factor);
    }

    fn display(&self, reverse: bool) -> String {
        let shown = dynamic_word_display(&self.word, reverse);

        if self.count.is_one() {
            shown
        } else if self.count.is_zero() {
            format!("{shown}..")
        } else {
            format!("{shown}^{}", self.count)
        }
    }
}

impl Display for DynamicBlock {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.display(false))
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct DynamicSpan {
    // Farthest-to-nearest, matching the original Span storage convention.
    blocks: Vec<DynamicBlock>,
}

impl DynamicSpan {
    const fn new(blocks: Vec<DynamicBlock>) -> Self {
        Self { blocks }
    }

    const fn init_blank() -> Self {
        Self::new(vec![])
    }

    fn init_stepped() -> Self {
        Self::new(vec![DynamicBlock::new(1, AlgCount::one())])
    }

    const fn len(&self) -> usize {
        self.blocks.len()
    }

    const fn blank(&self) -> bool {
        self.blocks.is_empty()
    }

    fn iter(&self) -> impl DoubleEndedIterator<Item = &DynamicBlock> {
        self.blocks.iter().rev()
    }

    fn first(&self) -> Option<&DynamicBlock> {
        self.blocks.last()
    }

    fn first_mut(&mut self) -> Option<&mut DynamicBlock> {
        self.blocks.last_mut()
    }

    fn pop_block(&mut self) -> DynamicBlock {
        self.blocks.pop().unwrap()
    }

    fn index(&self, pos: usize) -> &DynamicBlock {
        &self.blocks[self.blocks.len() - 1 - pos]
    }

    fn index_mut(&mut self, pos: usize) -> &mut DynamicBlock {
        let physical = self.blocks.len() - 1 - pos;
        &mut self.blocks[physical]
    }

    fn merge_metadata(
        into: &mut Option<DynamicBlock>,
        block: &DynamicBlock,
    ) {
        if let Some(into) = into {
            into.merge_origins(block);
        } else {
            *into = Some(block.clone());
        }
    }

    fn subtract_usize(count: &mut AlgCount, value: usize) {
        *count -= AlgCount::from(value);
    }

    fn discard_prefix_from_first_block(&mut self, cells: usize) {
        let block = self.pop_block();
        let full_copies = cells / block.width();
        let offset = cells % block.width();
        let mut copies_left = block.count.clone();
        Self::subtract_usize(&mut copies_left, full_copies);

        if offset != 0 {
            copies_left -= AlgCount::one();
        }

        if !copies_left.is_zero() {
            self.blocks.push(block.fragment(&block.word, copies_left));
        }

        if offset != 0 {
            self.blocks.push(
                block.fragment(&block.word[offset..], AlgCount::one()),
            );
        }
    }

    fn consume_matching_prefix(
        &mut self,
        color: Color,
    ) -> (AlgCount, Option<DynamicBlock>) {
        let mut consumed = AlgCount::zero();
        let mut metadata = None;

        while let Some(block) = self.first() {
            if block.homogeneous() {
                if block.first() != color {
                    break;
                }

                #[expect(clippy::shadow_unrelated)]
                let block = self.pop_block();
                consumed += &block.count;
                Self::merge_metadata(&mut metadata, &block);
                continue;
            }

            let prefix = block
                .word
                .iter()
                .take_while(|&&symbol| symbol == color)
                .count();

            if prefix == 0 {
                break;
            }

            consumed += AlgCount::from(prefix);
            let block = block.clone();
            Self::merge_metadata(&mut metadata, &block);
            self.discard_prefix_from_first_block(prefix);
            break;
        }

        (consumed, metadata)
    }

    fn pop_symbol(&mut self) -> Color {
        let Some(block) = self.first().cloned() else {
            return 0;
        };

        let next_scan = block.first();

        if block.homogeneous() {
            if block.is_single() {
                self.pop_block();
            } else {
                self.first_mut().unwrap().count -= AlgCount::one();
            }

            return next_scan;
        }

        self.pop_block();

        let mut remaining = block.count.clone();
        remaining -= AlgCount::one();

        if !remaining.is_zero() {
            self.blocks.push(block.fragment(&block.word, remaining));
        }

        self.blocks
            .push(block.fragment(&block.word[1..], AlgCount::one()));

        next_scan
    }

    fn pull(
        &mut self,
        scan: Color,
        skip: bool,
    ) -> (Color, AlgCount, Option<DynamicBlock>) {
        let (skipped, metadata) = if skip {
            self.consume_matching_prefix(scan)
        } else {
            (AlgCount::zero(), None)
        };

        let stepped = AlgCount::one() + skipped;

        (self.pop_symbol(), stepped, metadata)
    }

    fn push(
        &mut self,
        print: Color,
        stepped: &AlgCount,
        metadata: Option<&DynamicBlock>,
    ) {
        match self.first_mut() {
            Some(block)
                if block.homogeneous() && block.first() == print =>
            {
                block.count += stepped;
                if let Some(metadata) = metadata {
                    block.merge_origins(metadata);
                }
            },
            None if print == 0 => {},
            _ => {
                let mut block =
                    DynamicBlock::new(print, stepped.clone());
                if let Some(metadata) = metadata {
                    block.merge_origins(metadata);
                }
                self.blocks.push(block);
            },
        }
    }

    fn counts(&self) -> Vec<AlgCount> {
        self.iter().map(|block| block.count.clone()).collect()
    }

    fn signature(&self) -> Vec<DynamicColorCount> {
        self.iter().map(DynamicColorCount::from).collect()
    }

    fn sig_compatible(&self, span: &[DynamicColorCount]) -> bool {
        self.len() == span.len()
            && self
                .iter()
                .zip(span)
                .all(|(block, sig)| block.word.as_ref() == sig.word())
    }

    fn merge_equal_neighbors(&mut self) {
        let mut merged: Vec<DynamicBlock> =
            Vec::with_capacity(self.blocks.len());

        for block in self.blocks.drain(..) {
            if let Some(previous) = merged.last_mut()
                && previous.word == block.word
            {
                previous.count += &block.count;
                previous.merge_origins(&block);
            } else {
                merged.push(block);
            }
        }

        self.blocks = merged;
    }

    fn merge_near_neighbors(&mut self) {
        while self.blocks.len() >= 2 {
            let near_pos = self.blocks.len() - 1;
            let far_pos = near_pos - 1;

            if self.blocks[far_pos].word != self.blocks[near_pos].word {
                break;
            }

            let near = self.blocks.pop().unwrap();
            let far = self.blocks.last_mut().unwrap();
            far.count += &near.count;
            far.merge_origins(&near);
        }
    }

    fn trim_blank_edge(&mut self) {
        while self.blocks.first().is_some_and(DynamicBlock::blank) {
            self.blocks.remove(0);
        }

        let Some(block) = self.blocks.first().cloned() else {
            return;
        };

        let Some(last_mark) =
            block.word.iter().rposition(|&color| color != 0)
        else {
            return;
        };

        if last_mark + 1 == block.width() || block.is_indef() {
            return;
        }

        self.blocks.remove(0);

        if !block.is_single() {
            let mut remaining = block.count.clone();
            remaining -= AlgCount::one();
            self.blocks
                .insert(0, block.fragment(&block.word, remaining));
        }

        self.blocks.insert(
            0,
            block.fragment(&block.word[..=last_mark], AlgCount::one()),
        );
    }

    fn prefix_cells(
        &self,
        outward_index: usize,
        cells: &mut [Color; DYNAMIC_REBALANCE_WINDOW],
    ) -> usize {
        let mut size = 0;

        for block in self.iter().skip(outward_index) {
            if !block.homogeneous() || block.is_indef() {
                break;
            }

            let remaining = DYNAMIC_REBALANCE_WINDOW - size;
            if remaining == 0 {
                break;
            }

            let count = block.count.to_usize().unwrap_or(usize::MAX);
            let take = count.min(remaining);
            cells[size..size + take].fill(block.first());
            size += take;

            if take < count {
                break;
            }
        }

        size
    }

    fn best_prefix_repeat(
        &self,
        outward_index: usize,
    ) -> Option<(Vec<Color>, usize, usize)> {
        let mut cells = [0; DYNAMIC_REBALANCE_WINDOW];
        let size = self.prefix_cells(outward_index, &mut cells);
        let mut best: Option<(usize, usize, usize)> = None;

        for width in 1..=DYNAMIC_MAX_PATTERN.min(size / 2) {
            let word = &cells[..width];
            let mut copies = 1;

            while (copies + 1) * width <= size
                && &cells[copies * width..(copies + 1) * width] == word
            {
                copies += 1;
            }

            if copies < DYNAMIC_MIN_PATTERN_REPEATS {
                continue;
            }

            let covered = width * copies;
            let replace = best.is_none_or(
                |(best_width, best_copies, best_covered)| {
                    copies > best_copies
                        || (copies == best_copies
                            && covered > best_covered)
                        || (copies == best_copies
                            && covered == best_covered
                            && width < best_width)
                },
            );

            if replace {
                best = Some((width, copies, covered));
            }
        }

        best.map(|(width, copies, covered)| {
            (cells[..width].to_vec(), copies, covered)
        })
    }

    fn consume_prefix_at(
        &mut self,
        outward_index: usize,
        mut cells: usize,
    ) -> (usize, Option<DynamicBlock>) {
        let mut physical = self.blocks.len() - 1 - outward_index;
        let insertion;
        let mut metadata = None;

        loop {
            let block = self.blocks[physical].clone();
            debug_assert!(block.homogeneous());
            let available =
                block.count.to_usize().unwrap_or(usize::MAX);

            if cells < available {
                self.blocks[physical].count -= AlgCount::from(cells);
                Self::merge_metadata(&mut metadata, &block);
                insertion = physical + 1;
                break;
            }

            cells -= available;
            let removed = self.blocks.remove(physical);
            Self::merge_metadata(&mut metadata, &removed);

            if cells == 0 {
                insertion = physical;
                break;
            }

            assert!(
                physical > 0,
                "adaptive reblock consumed past span"
            );
            physical -= 1;
        }

        (insertion, metadata)
    }

    fn discover_at(&mut self, outward_index: usize) -> bool {
        let Some((word, copies, covered)) =
            self.best_prefix_repeat(outward_index)
        else {
            return false;
        };

        let (insertion, metadata) =
            self.consume_prefix_at(outward_index, covered);
        let mut compound =
            DynamicBlock::new_word(&word, AlgCount::from(copies));
        if let Some(metadata) = &metadata {
            compound.merge_origins(metadata);
        }
        self.blocks.insert(insertion, compound);

        self.merge_equal_neighbors();
        self.trim_blank_edge();
        true
    }

    fn discover_boundary(&mut self) -> bool {
        let Some((word, copies, covered)) = self.best_prefix_repeat(0)
        else {
            return false;
        };

        let (insertion, metadata) = self.consume_prefix_at(0, covered);
        let mut compound =
            DynamicBlock::new_word(&word, AlgCount::from(copies));
        if let Some(metadata) = &metadata {
            compound.merge_origins(metadata);
        }
        self.blocks.insert(insertion, compound);

        self.merge_near_neighbors();
        if self.blocks.first().is_some_and(DynamicBlock::blank) {
            self.trim_blank_edge();
        }
        true
    }

    fn normalize_boundary(&mut self) {
        self.merge_near_neighbors();

        if self.blocks.first().is_some_and(DynamicBlock::blank) {
            self.trim_blank_edge();
        }

        if self.blocks.len() >= 2 {
            self.discover_boundary();
        }
    }

    fn normalize(&mut self, full: bool) {
        for block in &mut self.blocks {
            block.normalize_word();
        }

        self.merge_equal_neighbors();
        self.trim_blank_edge();

        if self.blocks.len() < 2 {
            return;
        }

        let mut outward_index = 0;
        let mut operations = 0;
        let operation_limit = 4 * self.blocks.len() + 32;

        while outward_index + 1 < self.blocks.len()
            && operations < operation_limit
        {
            if self.discover_at(outward_index) {
                operations += 1;
                if !full {
                    break;
                }
                outward_index = outward_index.saturating_sub(1);
            } else if full {
                outward_index += 1;
            } else {
                break;
            }
        }
    }

    fn visit_dependency_prefix(
        &self,
        mut visit: impl FnMut(&DynamicBlock),
    ) {
        let mut cells = 0_usize;

        for block in self.iter() {
            visit(block);

            if block.is_indef() {
                break;
            }

            let Some(count) = block.count.to_usize() else {
                break;
            };

            cells = cells
                .saturating_add(block.width().saturating_mul(count));
            if cells >= DYNAMIC_REBALANCE_WINDOW {
                break;
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum DynamicSigWord {
    Single(Color),
    Compound { word: DynamicWord, hash: u64 },
}

impl DynamicSigWord {
    fn from_block(block: &DynamicBlock) -> Self {
        if block.homogeneous() {
            Self::Single(block.first())
        } else {
            Self::Compound {
                word: Arc::clone(&block.word),
                hash: block.word_hash,
            }
        }
    }

    fn word(&self) -> &[Color] {
        match self {
            Self::Single(color) => core::slice::from_ref(color),
            Self::Compound { word, .. } => word,
        }
    }
}

impl PartialEq for DynamicSigWord {
    fn eq(&self, other: &Self) -> bool {
        self.word() == other.word()
    }
}

impl Eq for DynamicSigWord {}

impl Hash for DynamicSigWord {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Self::Single(color) => {
                0_u8.hash(state);
                color.hash(state);
            },
            Self::Compound { hash, .. } => {
                1_u8.hash(state);
                hash.hash(state);
            },
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum DynamicColorCount {
    Just(DynamicSigWord),
    Mult(DynamicSigWord),
}

impl DynamicColorCount {
    fn word(&self) -> &[Color] {
        match self {
            Self::Just(word) | Self::Mult(word) => word.word(),
        }
    }
}

impl From<&DynamicBlock> for DynamicColorCount {
    fn from(block: &DynamicBlock) -> Self {
        let word = DynamicSigWord::from_block(block);
        if block.is_single() {
            Self::Just(word)
        } else {
            Self::Mult(word)
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DynamicSignature {
    pub scan: Color,
    pub lspan: Vec<DynamicColorCount>,
    pub rspan: Vec<DynamicColorCount>,
}

pub type DynamicMinSig = (DynamicSignature, (bool, bool));

impl DynamicSignature {
    pub fn matches(&self, (other, (lex, rex)): &DynamicMinSig) -> bool {
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

impl From<&Signature> for DynamicSignature {
    fn from(sig: &Signature) -> Self {
        fn convert(span: &[ColorCount]) -> Vec<DynamicColorCount> {
            span.iter()
                .map(|entry| {
                    let word =
                        DynamicSigWord::Single(entry.get_color());
                    match entry {
                        ColorCount::Just(_) => {
                            DynamicColorCount::Just(word)
                        },
                        ColorCount::Mult(_) => {
                            DynamicColorCount::Mult(word)
                        },
                    }
                })
                .collect()
        }

        Self {
            scan: sig.scan,
            lspan: convert(&sig.lspan),
            rspan: convert(&sig.rspan),
        }
    }
}

pub trait DynamicGetSig: Scan {
    fn dynamic_signature(&self) -> DynamicSignature;
}

#[derive(Clone, Eq, PartialEq)]
pub struct DynamicTape {
    pub scan: Color,
    pub lspan: DynamicSpan,
    pub rspan: DynamicSpan,
}

impl DynamicTape {
    pub const fn at_edge(&self, edge: Shift) -> bool {
        self.scan == 0
            && (if edge { &self.rspan } else { &self.lspan }).blank()
    }

    pub const fn blank(&self) -> bool {
        self.scan == 0 && self.lspan.blank() && self.rspan.blank()
    }

    pub fn step(
        &mut self,
        shift: Shift,
        color: Color,
        skip: bool,
    ) -> AlgCount {
        let (pull, push) = if shift {
            (&mut self.rspan, &mut self.lspan)
        } else {
            (&mut self.lspan, &mut self.rspan)
        };

        let (next_scan, stepped, metadata) = pull.pull(self.scan, skip);
        push.push(color, &stepped, metadata.as_ref());
        self.scan = next_scan;

        pull.normalize_boundary();
        push.normalize_boundary();

        stepped
    }

    pub fn normalize(&mut self) {
        self.lspan.normalize(false);
        self.rspan.normalize(false);
    }

    pub fn rebalance(&mut self) {
        self.lspan.normalize(true);
        self.rspan.normalize(true);
    }

    pub fn marks(&self) -> AlgCount {
        let mut marks = AlgCount::from(self.scan != 0);

        for block in self.lspan.iter().chain(self.rspan.iter()) {
            marks += block.count.clone()
                * AlgCount::from(block.marked_width());
        }

        marks
    }

    pub const fn blocks(&self) -> usize {
        self.lspan.len() + self.rspan.len()
    }

    pub const fn length_one_spans(&self) -> bool {
        self.lspan.len() == 1 && self.rspan.len() == 1
    }

    pub fn counts(&self) -> (Vec<AlgCount>, Vec<AlgCount>) {
        (self.lspan.counts(), self.rspan.counts())
    }

    pub fn sig_compatible(&self, sig: &DynamicSignature) -> bool {
        self.scan == sig.scan
            && self.lspan.sig_compatible(&sig.lspan)
            && self.rspan.sig_compatible(&sig.rspan)
    }
}

impl Display for DynamicTape {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let left =
            self.lspan.iter().rev().map(|block| block.display(true));
        let right = self.rspan.iter().map(|block| block.display(false));

        write!(
            f,
            "{}",
            left.chain(once(format!("[{}]", self.scan)))
                .chain(right)
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}

impl Scan for DynamicTape {
    fn scan(&self) -> Color {
        self.scan
    }
}

impl Init for DynamicTape {
    fn init() -> Self {
        Self {
            scan: 0,
            lspan: DynamicSpan::init_blank(),
            rspan: DynamicSpan::init_blank(),
        }
    }

    fn init_stepped() -> Self {
        Self {
            scan: 0,
            lspan: DynamicSpan::init_stepped(),
            rspan: DynamicSpan::init_blank(),
        }
    }
}

impl DynamicGetSig for DynamicTape {
    fn dynamic_signature(&self) -> DynamicSignature {
        DynamicSignature {
            scan: self.scan,
            lspan: self.lspan.signature(),
            rspan: self.rspan.signature(),
        }
    }
}

impl MachineTape for DynamicTape {
    fn mstep(&mut self, shift: Shift, color: Color, skip: bool) {
        self.step(shift, color, skip);
    }
}

impl IndexTape<AlgCount> for DynamicTape {
    fn get_count(&self, &(side, pos): &Index) -> &AlgCount {
        let span = if side { &self.rspan } else { &self.lspan };
        &span.index(pos).count
    }

    fn set_count(&mut self, &(side, pos): &Index, val: AlgCount) {
        let span = if side {
            &mut self.rspan
        } else {
            &mut self.lspan
        };
        span.index_mut(pos).count = val;
    }
}

impl From<&AlgTape> for DynamicTape {
    fn from(tape: &AlgTape) -> Self {
        fn convert(span: &Span<AlgBlock>) -> DynamicSpan {
            DynamicSpan::new(
                span.iter()
                    .rev()
                    .map(|block| {
                        DynamicBlock::new(
                            block.color,
                            block.count.clone(),
                        )
                    })
                    .collect(),
            )
        }

        let mut dynamic = Self {
            scan: tape.scan,
            lspan: convert(&tape.lspan),
            rspan: convert(&tape.rspan),
        };
        dynamic.rebalance();
        dynamic
    }
}

pub trait DynamicTapeOps:
    DynamicGetSig + MachineTape + IndexTape<AlgCount>
{
    fn normalize_dynamic(&mut self);
}

impl DynamicTapeOps for DynamicTape {
    fn normalize_dynamic(&mut self) {
        self.normalize();
    }
}

#[derive(Clone)]
pub struct DynamicEnumTape {
    tape: DynamicTape,
    l_offset: Cell<usize>,
    r_offset: Cell<usize>,
    l_edge: Cell<bool>,
    r_edge: Cell<bool>,
}

impl From<&DynamicTape> for DynamicEnumTape {
    fn from(tape: &DynamicTape) -> Self {
        fn convert(span: &DynamicSpan, side: Shift) -> DynamicSpan {
            let len = span.len();

            DynamicSpan::new(
                span.iter()
                    .rev()
                    .enumerate()
                    .map(|(i, block)| {
                        let mut block = block.clone();
                        block.origins = vec![(side, len - i)];
                        block
                    })
                    .collect(),
            )
        }

        Self {
            tape: DynamicTape {
                scan: tape.scan,
                lspan: convert(&tape.lspan, false),
                rspan: convert(&tape.rspan, true),
            },
            l_offset: 0.into(),
            r_offset: 0.into(),
            l_edge: false.into(),
            r_edge: false.into(),
        }
    }
}

impl DynamicEnumTape {
    fn touch_edge(&self, shift: Shift) {
        (if shift { &self.r_edge } else { &self.l_edge }).set(true);
    }

    fn check_offsets(&self, block: &DynamicBlock) {
        for &(side, offset) in &block.origins {
            let target =
                if side { &self.r_offset } else { &self.l_offset };
            target.set(target.get().max(offset));
        }
    }

    fn check_dependency_prefix(&self, span: &DynamicSpan) {
        span.visit_dependency_prefix(|block| self.check_offsets(block));
    }

    fn check_step(&self, shift: Shift, skip: bool) {
        let (pull, push) = if shift {
            (&self.tape.rspan, &self.tape.lspan)
        } else {
            (&self.tape.lspan, &self.tape.rspan)
        };

        if pull.blank() {
            self.touch_edge(shift);
        } else {
            self.check_dependency_prefix(pull);

            let near = pull.index(0);
            if skip
                && near.homogeneous()
                && near.first() == self.tape.scan
            {
                if pull.len() == 1 {
                    self.touch_edge(shift);
                } else {
                    self.check_offsets(pull.index(1));
                }
            }
        }

        self.check_dependency_prefix(push);
    }

    pub fn get_min_sig(&self, sig: &DynamicSignature) -> DynamicMinSig {
        let lmax = self.l_offset.get().min(sig.lspan.len());
        let rmax = self.r_offset.get().min(sig.rspan.len());

        (
            DynamicSignature {
                scan: sig.scan,
                lspan: sig.lspan[..lmax].to_vec(),
                rspan: sig.rspan[..rmax].to_vec(),
            },
            (self.l_edge.get(), self.r_edge.get()),
        )
    }
}

impl Display for DynamicEnumTape {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.tape)
    }
}

impl Scan for DynamicEnumTape {
    fn scan(&self) -> Color {
        self.tape.scan
    }
}

impl DynamicGetSig for DynamicEnumTape {
    fn dynamic_signature(&self) -> DynamicSignature {
        self.tape.dynamic_signature()
    }
}

impl MachineTape for DynamicEnumTape {
    fn mstep(&mut self, shift: Shift, color: Color, skip: bool) {
        self.check_step(shift, skip);
        self.tape.step(shift, color, skip);
    }
}

impl IndexTape<AlgCount> for DynamicEnumTape {
    fn get_count(&self, &(side, pos): &Index) -> &AlgCount {
        let span = if side {
            &self.tape.rspan
        } else {
            &self.tape.lspan
        };
        let block = span.index(pos);
        self.check_offsets(block);
        &block.count
    }

    fn set_count(&mut self, index: &Index, val: AlgCount) {
        self.tape.set_count(index, val);
    }
}

impl DynamicTapeOps for DynamicEnumTape {
    fn normalize_dynamic(&mut self) {
        self.tape.normalize();
    }
}

#[cfg(test)]
mod dynamic_tape_tests {
    use super::*;

    fn dynamic_from_cells(cells: &[Color]) -> DynamicTape {
        let blocks = cells
            .iter()
            .rev()
            .map(|&color| DynamicBlock::new(color, AlgCount::one()))
            .collect();

        let mut tape = DynamicTape {
            scan: 0,
            lspan: DynamicSpan::new(blocks),
            rspan: DynamicSpan::init_blank(),
        };
        tape.rebalance();
        tape
    }

    #[test]
    fn dynamic_discovers_repeated_word() {
        let tape =
            dynamic_from_cells(&[0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 1]);
        let block = tape.lspan.index(0);

        assert_eq!(block.word.as_ref(), &[0, 0, 0, 1]);
        assert_eq!(block.count, AlgCount::from(3_u8));
    }

    #[test]
    fn dynamic_sides_rebalance_independently() {
        let mut tape = DynamicTape {
            scan: 2,
            lspan: DynamicSpan::new(
                [0, 1, 0, 1, 0, 1]
                    .into_iter()
                    .rev()
                    .map(|color| {
                        DynamicBlock::new(color, AlgCount::one())
                    })
                    .collect(),
            ),
            rspan: DynamicSpan::new(
                [1, 0, 0, 1, 0, 0, 1, 0, 0]
                    .into_iter()
                    .rev()
                    .map(|color| {
                        DynamicBlock::new(color, AlgCount::one())
                    })
                    .collect(),
            ),
        };

        tape.rebalance();

        assert_eq!(tape.lspan.index(0).word.as_ref(), &[0, 1]);
        assert_eq!(tape.rspan.index(0).word.as_ref(), &[1]);
    }
}

/**************************************/

#[cfg(test)]
impl AlgTape {
    #[track_caller]
    fn assert(&self, marks: u32, tape_str: &str, sig: &str) {
        assert_eq!(self.blank(), marks == 0);

        assert_eq!(self.to_string(), tape_str);

        let signature = self.signature();

        assert_eq!(signature, sig.into());

        assert!(
            signature
                .matches(&EnumTape::from(self).get_min_sig(&signature))
        );
    }

    #[track_caller]
    fn tstep(&mut self, shift: u8, color: Color, skip: u8) {
        assert!(matches!(shift, 0 | 1));
        assert!(matches!(skip, 0 | 1));

        self.step(shift != 0, color, skip != 0);
    }
}

#[cfg(test)]
impl AlgSpan {
    fn from_data(data: Vec<(Color, usize)>) -> Self {
        Self::new(
            data.into_iter()
                .map(|(cr, ct)| AlgBlock::new(cr, AlgCount::from(ct)))
                .rev()
                .collect(),
        )
    }
}

#[cfg(test)]
macro_rules! tape {
    ($tape:expr) => {
        AlgTape::from($tape)
    };
}

#[cfg(test)]
impl From<&str> for AlgTape {
    fn from(s: &str) -> Self {
        fn parse_block(part: &str) -> (Color, usize) {
            if let Some((color, count)) = part.split_once('^') {
                (color.parse().unwrap(), count.parse().unwrap())
            } else if let Some(color) = part.strip_suffix("..") {
                (color.parse().unwrap(), 0)
            } else {
                (part.parse().unwrap(), 1)
            }
        }

        let parts: Vec<&str> = s.split_whitespace().collect();

        let scan_pos = parts
            .iter()
            .position(|p| p.starts_with('[') && p.ends_with(']'))
            .unwrap();

        let scan = parts[scan_pos]
            .trim_matches(|c| c == '[' || c == ']')
            .parse()
            .unwrap();

        let lspan = Span::from_data(
            parts[..scan_pos]
                .iter()
                .rev()
                .map(|part| parse_block(part))
                .collect(),
        );

        let rspan = Span::from_data(
            parts[scan_pos + 1..]
                .iter()
                .map(|part| parse_block(part))
                .collect(),
        );

        Self { scan, lspan, rspan }
    }
}

#[cfg(test)]
impl From<&str> for EnumTape {
    fn from(s: &str) -> Self {
        (&AlgTape::from(s)).into()
    }
}

#[cfg(test)]
impl From<&str> for Signature {
    fn from(s: &str) -> Self {
        let parts: Vec<&str> = s.split_whitespace().collect();

        let lspan: Vec<ColorCount> = parts
            .iter()
            .take_while(|p| !p.starts_with('['))
            .map(|&p| p.into())
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        let scan = parts
            .iter()
            .find(|p| p.starts_with('['))
            .and_then(|p| {
                p.trim_matches(|c| c == '[' || c == ']').parse().ok()
            })
            .unwrap();

        let rspan_start = parts
            .iter()
            .position(|&p| p.starts_with('['))
            .map_or(parts.len(), |pos| pos + 1);

        let rspan: Vec<ColorCount> =
            parts[rspan_start..].iter().map(|&p| p.into()).collect();

        Self { scan, lspan, rspan }
    }
}

#[cfg(test)]
impl From<&str> for ColorCount {
    fn from(s: &str) -> Self {
        s.strip_prefix('(')
            .and_then(|s| s.strip_suffix(')'))
            .map_or_else(
                || Mult(s.parse().unwrap()),
                |t| Just(t.parse().unwrap()),
            )
    }
}

#[test]
fn test_init() {
    Tape::init().assert(0, "[0]", "[0]");

    let mut tape = Tape::init_stepped();

    tape.assert(1, "1 [0]", "(1) [0]");

    tape.tstep(1, 1, 0);

    tape.assert(2, "1^2 [0]", "1 [0]");

    tape.tstep(0, 0, 0);

    tape.assert(2, "1 [1]", "(1) [1]");

    tape.tstep(0, 0, 1);

    tape.assert(0, "[0]", "[0]");
}

#[test]
fn test_clone() {
    let tape = tape!("1 0 1 [2] 2 1^2");

    let mut copy_1 = tape.clone();
    let mut copy_2 = tape.clone();

    copy_1.tstep(0, 2, 0);
    copy_2.tstep(1, 1, 0);

    copy_1.assert(6, "1 0 [1] 2^2 1^2", "(1) (0) [1] 2 1");
    copy_2.assert(6, "1 0 1^2 [2] 1^2", "(1) (0) 1 [2] 1");

    tape.assert(6, "1 0 1 [2] 2 1^2", "(1) (0) (1) [2] (2) 1");
}

#[cfg(test)]
use crate::rules::{ApplyRule as _, Diff, Op, Rule};

#[cfg(test)]
macro_rules! plus {
    ($diff:literal) => {
        Op::Plus(Diff::from($diff))
    };
}

#[cfg(test)]
macro_rules! mult {
    ($mul:literal, $add:literal) => {
        #[expect(trivial_numeric_casts)]
        Op::Mult((
            AlgCount::from($mul as u8),
            AlgCount::from($add as u8),
        ))
    };
}

#[cfg(test)]
macro_rules! rule {
    (
        $ ( ( $ shift : literal, $ index : literal ) => $ op : expr ), *
        $ ( , ) *
    ) => {
        Rule([$ ( (( $ shift == 1, $ index ), $ op ) ), *].into())
    }
}

#[test]
fn test_apply_1() {
    let mut tape = tape!("2^3 1^12 [3] 4^15 5^2 6^2");

    tape.assert(35, "2^3 1^12 [3] 4^15 5^2 6^2", "2 1 [3] 4 5 6");

    tape.apply_rule(&rule![
        (0, 1) => plus!(3),
        (1, 0) => plus!(-2),
    ]);

    tape.assert(42, "2^24 1^12 [3] 4 5^2 6^2", "2 1 [3] (4) 5 6");
}

#[test]
fn test_apply_2() {
    let mut tape = tape!("4^2 [4] 5^60 2 4 5^7 1");

    tape.assert(73, "4^2 [4] 5^60 2 4 5^7 1", "4 [4] 5 (2) (4) 5 (1)");

    tape.apply_rule(&rule![
        (0, 0) => plus!(4),
        (1, 0) => plus!(-2),
    ]);

    tape.assert(
        131,
        "4^118 [4] 5^2 2 4 5^7 1",
        "4 [4] 5 (2) (4) 5 (1)",
    );
}

#[test]
fn test_apply_3() {
    let mut tape = tape!("3 2^655345 1^152 [0]");

    tape.assert(655_498, "3 2^655345 1^152 [0]", "(3) 2 1 [0]");

    let rule = rule! [
        (0, 1) => plus!(-2),
        (0, 0) => mult!(2, 8),
    ];

    let (times, _, _) = tape.count_apps(&rule).unwrap();

    assert_eq!(times, 327_672_u32.into());

    tape.apply_rule(&rule);

    assert_eq!(tape.to_string().len(), 98652);
}

#[test]
fn test_apply_4() {
    let mut tape = tape!("2^506 [2] 2 1 0^10 1");

    tape.assert(510, "2^506 [2] 2 1 0^10 1", "2 [2] (2) (1) 0 (1)");

    tape.apply_rule(&rule![
        (0, 0) => mult!(2, 6),
        (1, 2) => plus!(-1),
    ]);

    tape.assert(
        0x0003_FFFE,
        "2^262138 [2] 2 1 0 1",
        "2 [2] (2) (1) (0) (1)",
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

        self.mstep(shift != 0, color, skip != 0);
    }
}

#[cfg(test)]
macro_rules! enum_tape {
    ($tape:expr) => {
        EnumTape::from($tape)
    };
}

#[test]
fn test_offsets_1() {
    let mut tape = enum_tape!("2 3^11 4 1^11 [0]");

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
    let mut tape = enum_tape!("3^6 2^414422565 [0]");

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
    let mut tape = enum_tape!("3^9 [3] 1^10");

    tape.tstep(0, 1, 0);

    tape.assert("3^8 [3] 1^11", (1, 1), (0, 0));
}

#[test]
fn test_offsets_4() {
    let mut tape = enum_tape!("6 30^6 [28] 27^5 12");

    tape.assert("6 30^6 [28] 27^5 12", (0, 0), (0, 0));

    let sig = tape.signature();

    tape.tstep(1, 29, 0);
    tape.tstep(0, 28, 0);
    tape.tstep(1, 30, 0);

    tape.assert("6 30^7 [28] 27^4 12", (1, 1), (0, 0));

    assert_eq!(
        tape.get_min_sig(&sig),
        ("30 [28] 27".into(), (false, false))
    );
}

#[test]
fn test_edges_1() {
    let mut tape = enum_tape!("[0]");

    tape.tstep(0, 1, 0);

    tape.assert("[0] 1", (0, 0), (1, 0));
}

#[test]
fn test_edges_2() {
    let mut tape = enum_tape!("1^3 [1]");

    tape.tstep(0, 2, 1);

    tape.assert("[0] 2^4", (1, 0), (1, 0));
}
