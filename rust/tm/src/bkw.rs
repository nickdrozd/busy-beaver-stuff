use core::{
    fmt,
    hash::{Hash, Hasher as _},
    iter::once,
};

use ahash::{AHashMap as Dict, AHashSet as Set, AHasher};
use std::sync::Arc;

use crate::{
    Color, Instr, Prog, Shift, Slot, State, Steps, instrs::Parse as _,
    tape::Scan,
};

const MAX_STACK_DEPTH: usize = 64;

/**************************************/

#[derive(Debug)]
pub enum BackwardResult {
    Init,
    StepLimit,
    DepthLimit,
    CountLimit,
    Refuted(Steps),
}

use BackwardResult::*;

impl BackwardResult {
    pub const fn is_refuted(&self) -> bool {
        matches!(self, Refuted(_))
    }
}

/**************************************/

impl<const s: usize, const c: usize> Prog<s, c> {
    pub fn bkw_cant_halt(&self, steps: Steps) -> BackwardResult {
        let (entrypoints, idx) = self.entrypoints_and_indices();

        let slots = self.halt_slots_disp_side(&idx);
        let slots = self.halt_slots_side_excursion(slots);

        cant_reach(
            self,
            steps,
            slots,
            Some(entrypoints),
            halt_configs,
            false,
        )
    }

    pub fn bkw_cant_blank(&self, steps: Steps) -> BackwardResult {
        if self.cant_blank_by_color_graph() {
            return Refuted(0);
        }

        cant_reach(
            self,
            steps,
            self.blank_slots_side_clean(),
            None,
            erase_configs,
            false,
        )
    }

    pub fn bkw_cant_spinout(&self, steps: Steps) -> BackwardResult {
        cant_reach(
            self,
            steps,
            self.spinout_shifts_side_clean(),
            None,
            zr_configs,
            false,
        )
    }

    pub fn bkw_cant_zloop(&self, steps: Steps) -> BackwardResult {
        cant_reach(
            self,
            steps,
            self.zloop_shifts_side_clean(),
            None,
            zr_configs,
            false,
        )
    }

    pub fn bkw_cant_twostep(&self, steps: Steps) -> BackwardResult {
        cant_reach(
            self,
            steps,
            self.twostep_slots()
                .into_iter()
                .map(|((st, l_co), (_, r_co))| (st, (l_co, r_co)))
                .collect(),
            None,
            twostep_configs,
            true,
        )
    }
}

/**************************************/

type Configs = Vec<Config>;
type BlankStates = Set<State>;

type Entry = (Slot, (Color, Shift));
type Entries = Vec<Entry>;
type Entrypoints = Dict<State, (Entries, Entries)>;

/// Compact lookup tables for which 3-cell windows `(L, scan, R)` are
/// possible in some run from the blank tape.
///
/// - `right[st][scan][left]` is a bitmask of possible right colors.
/// - `left[st][scan][right]` is a bitmask of possible left colors.
/// - `any[st][scan]` records whether at least one neighbor pair is possible.
///
/// This makes all four known/unknown-neighbor lookup cases constant-time.
struct WinPossible<const S: usize, const C: usize> {
    right: [[[u64; C]; C]; S],
    left: [[[u64; C]; C]; S],
    any: [[bool; C]; S],

    // Two-bit masks of possible total nonblank-cell parities.  The exact
    // table retains `(state, left, scan, right)` correlation; the three
    // aggregate tables mirror `right`/`left`/`any` so queries with unknown
    // neighbors remain constant-time. Bit 0 is even, bit 1 is odd.
    parity: Vec<u8>,
    parity_right: [[[u8; C]; C]; S],
    parity_left: [[[u8; C]; C]; S],
    parity_any: [[u8; C]; S],

    // Four-bit masks of possible `(left nonblank parity, right nonblank
    // parity)` combinations.  Combination `lp | (rp << 1)` is represented by
    // bit `1 << combination`.  Keeping the two side parities jointly is
    // strictly stronger than total support parity: the latter is recovered as
    // `lp ^ rp ^ (scan != 0)`.
    side_parity: Vec<u8>,
    side_parity_right: [[[u8; C]; C]; S],
    side_parity_left: [[[u8; C]; C]; S],
    side_parity_any: [[u8; C]; S],

    // Nine-bit masks of possible `(left nonblank count mod 3, right nonblank
    // count mod 3)` combinations.  Combination `left + 3 * right` is bit
    // `1 << combination`.  This is kept in addition to side parity so the
    // mod-3 refinement cannot lose any parity pruning power.
    side_mod3: Vec<u16>,
    side_mod3_right: [[[u16; C]; C]; S],
    side_mod3_left: [[[u16; C]; C]; S],
    side_mod3_any: [[u16; C]; S],

    // Bitset of possible global per-color parity vectors, conditioned on the
    // exact local window.  Vector bit `k - 1` is the parity of the number of
    // cells of nonblank color `k`.  The outer u64 bitset therefore supports
    // up to 2^6 vectors, i.e. alphabets with at most 7 colors including 0.
    // Larger alphabets conservatively skip this refinement.
    color_parity: Vec<u64>,
    color_parity_right: [[[u64; C]; C]; S],
    color_parity_left: [[[u64; C]; C]; S],
    color_parity_any: [[u64; C]; S],
}

impl<const S: usize, const C: usize> WinPossible<S, C> {
    const fn parity_index(
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
    ) -> usize {
        (((st * C) + scan) * C + left) * C + right
    }

    fn exact_parity_mask(
        &self,
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
    ) -> u8 {
        self.parity[Self::parity_index(st, scan, left, right)]
    }

    fn exact_side_parity_mask(
        &self,
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
    ) -> u8 {
        self.side_parity[Self::parity_index(st, scan, left, right)]
    }

    fn exact_side_mod3_mask(
        &self,
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
    ) -> u16 {
        self.side_mod3[Self::parity_index(st, scan, left, right)]
    }

    fn exact_color_parity_mask(
        &self,
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
    ) -> u64 {
        self.color_parity[Self::parity_index(st, scan, left, right)]
    }

    const fn color_parity_enabled() -> bool {
        C <= 7
    }

    const fn all_color_parity_vectors() -> u64 {
        if !Self::color_parity_enabled() {
            return u64::MAX;
        }

        let states = 1_usize << C.saturating_sub(1);
        if states == 64 {
            u64::MAX
        } else {
            (1_u64 << states) - 1
        }
    }
}

const LEFT_SIDE: usize = 0;
const RIGHT_SIDE: usize = 1;

/// Whole-side color/pair summaries conditioned on an exact reachable local
/// window `(left, scan, right)` as well as the control state.
///
/// Pairs are oriented from the head toward the tape end.  A summary therefore
/// retains correlations that the state-and-scan-only version joined away:
/// two configurations with the same `(state, scan)` but different immediate
/// neighbors no longer automatically share all whole-side colors and pairs.
///
/// Storage is flattened onto the heap because the full `S * C^3` family of
/// summaries can otherwise become a large stack value for bigger alphabets.
#[derive(Clone, Copy)]
struct WindowSideSummary<const C: usize> {
    reachable: bool,
    colors: [u64; 2],
    pairs: [[u64; C]; 2],
}

impl<const C: usize> WindowSideSummary<C> {
    const fn empty() -> Self {
        Self {
            reachable: false,
            colors: [0; 2],
            pairs: [[0; C]; 2],
        }
    }
}

struct SidePossible<const S: usize, const C: usize> {
    // Dense exact-window index -> compact summary index. `usize::MAX` means
    // the window has never been reached. This avoids eagerly zero-filling a
    // full `S * C^3` array of large `WindowSideSummary<C>` values.
    lookup: Vec<usize>,
    windows: Vec<WindowSideSummary<C>>,
    empty: WindowSideSummary<C>,
}

impl<const S: usize, const C: usize> SidePossible<S, C> {
    const fn index(
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
    ) -> usize {
        (((st * C) + scan) * C + left) * C + right
    }

    fn new() -> Self {
        let len = S * C * C * C;
        Self {
            lookup: vec![usize::MAX; len],
            // Reserve address space for the worst case without initializing
            // any summaries. Reached windows are constructed on demand.
            windows: Vec::with_capacity(len),
            empty: WindowSideSummary::empty(),
        }
    }

    const fn slot_count(&self) -> usize {
        self.lookup.len()
    }

    fn window_by_index(&self, index: usize) -> &WindowSideSummary<C> {
        let compact = self.lookup[index];
        if compact == usize::MAX {
            &self.empty
        } else {
            &self.windows[compact]
        }
    }

    fn window(
        &self,
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
    ) -> &WindowSideSummary<C> {
        self.window_by_index(Self::index(st, scan, left, right))
    }

    /// Intersect whole-side color/pair summaries with the independently
    /// computed per-color tail-count domain.
    ///
    /// `ColorTailCountPossible` describes cells strictly beyond the immediate
    /// neighbors.  If, for one exact local window, a color can only have tail
    /// count zero on a side, that color cannot occur as the *far* endpoint of
    /// any oriented pair on that side.  If it is not the immediate neighbor
    /// either, it cannot occur anywhere on that side at all.
    ///
    /// This is a sound reduced-product refinement: both component domains are
    /// forward over-approximations of the same blank-start executions.  It is
    /// useful before ordered-prefix propagation because `pairs[side][near]` is
    /// exactly what exposes the next cell when the head moves into that side.
    #[expect(clippy::excessive_nesting)]
    fn refine_zero_tail_pairs(
        &mut self,
        counts: &ColorTailCountPossible<S, C>,
    ) -> bool {
        // Status is left_count + 3 * right_count with each count in 0..=2.
        // These masks select statuses where the corresponding side count is 0.
        const LEFT_ZERO: u16 = (1 << 0) | (1 << 3) | (1 << 6);
        const RIGHT_ZERO: u16 = (1 << 0) | (1 << 1) | (1 << 2);
        const ALL_STATUSES: u16 = (1 << 9) - 1;

        let mut changed = false;

        for st in 0..S {
            for scan in 0..C {
                for left in 0..C {
                    for right in 0..C {
                        let index = Self::index(st, scan, left, right);
                        let compact = self.lookup[index];
                        if compact == usize::MAX {
                            continue;
                        }

                        let summary = &mut self.windows[compact];

                        for color in 1..C {
                            let count_mask =
                                counts.exact[ColorTailCountPossible::<
                                    S,
                                    C,
                                >::exact_index(
                                    st, scan, left, right, color,
                                )];

                            // A zero mask means the count abstraction itself has
                            // no witness for this exact window/color.  Do not use
                            // absence of information as a pruning fact here.
                            if count_mask == 0 {
                                continue;
                            }

                            let bit = 1_u64 << color;
                            let left_tail_zero = count_mask
                                & (ALL_STATUSES ^ LEFT_ZERO)
                                == 0;
                            let right_tail_zero = count_mask
                                & (ALL_STATUSES ^ RIGHT_ZERO)
                                == 0;

                            if left_tail_zero {
                                // The far endpoint of every oriented pair lies
                                // strictly beyond the immediate neighbor.
                                for near in 0..C {
                                    let old =
                                        summary.pairs[LEFT_SIDE][near];
                                    summary.pairs[LEFT_SIDE][near] &=
                                        !bit;
                                    changed |= summary.pairs[LEFT_SIDE]
                                        [near]
                                        != old;
                                }

                                // If the exact immediate neighbor is not this
                                // color either, then the color is absent from the
                                // complete left side, including as a pair-near.
                                if left != color {
                                    let old = summary.colors[LEFT_SIDE];
                                    summary.colors[LEFT_SIDE] &= !bit;
                                    changed |= summary.colors
                                        [LEFT_SIDE]
                                        != old;

                                    let old =
                                        summary.pairs[LEFT_SIDE][color];
                                    summary.pairs[LEFT_SIDE][color] = 0;
                                    changed |= summary.pairs[LEFT_SIDE]
                                        [color]
                                        != old;
                                }
                            }

                            if right_tail_zero {
                                for near in 0..C {
                                    let old =
                                        summary.pairs[RIGHT_SIDE][near];
                                    summary.pairs[RIGHT_SIDE][near] &=
                                        !bit;
                                    changed |= summary.pairs
                                        [RIGHT_SIDE][near]
                                        != old;
                                }

                                if right != color {
                                    let old =
                                        summary.colors[RIGHT_SIDE];
                                    summary.colors[RIGHT_SIDE] &= !bit;
                                    changed |= summary.colors
                                        [RIGHT_SIDE]
                                        != old;

                                    let old = summary.pairs[RIGHT_SIDE]
                                        [color];
                                    summary.pairs[RIGHT_SIDE][color] =
                                        0;
                                    changed |= summary.pairs
                                        [RIGHT_SIDE][color]
                                        != old;
                                }
                            }
                        }
                    }
                }
            }
        }

        changed
    }

    fn insert_window_by_index(
        &mut self,
        index: usize,
        summary: WindowSideSummary<C>,
    ) {
        debug_assert_eq!(self.lookup[index], usize::MAX);
        let compact = self.windows.len();
        self.windows.push(summary);
        self.lookup[index] = compact;
    }
}

/// Ordered three-cell whole-side summaries, conditioned on the same exact
/// local window as `SidePossible`.
///
/// `mask(..., side, a, b)` is a bitmask of colors `c` for which the oriented
/// near-to-far triple `(a, b, c)` occurs somewhere on that side in at least one
/// forward execution represented by the exact window.  This is deliberately a
/// separate sparse-ish product from `WindowSideSummary`: for large alphabets
/// the C^2 table per window would dominate the cheap color/pair domain.  The
/// refinement is therefore enabled only for modest alphabets; when disabled,
/// backward queries conservatively accept everything.
const SIDE_TRIPLE_MAX_COLORS: usize = 8;

struct SideTriplePossible<const S: usize, const C: usize> {
    enabled: bool,
    masks: Vec<u64>,
}

impl<const S: usize, const C: usize> SideTriplePossible<S, C> {
    const fn window_index(
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
    ) -> usize {
        (((st * C) + scan) * C + left) * C + right
    }

    const fn index(
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
        side: usize,
        near: usize,
        middle: usize,
    ) -> usize {
        (((Self::window_index(st, scan, left, right) * 2 + side) * C
            + near)
            * C)
            + middle
    }

    fn new() -> Self {
        let enabled = C <= SIDE_TRIPLE_MAX_COLORS;
        let len = if enabled {
            S * C * C * C * 2 * C * C
        } else {
            0
        };
        Self {
            enabled,
            masks: vec![0; len],
        }
    }

    fn mask(
        &self,
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
        side: usize,
        near: usize,
        middle: usize,
    ) -> u64 {
        if !self.enabled {
            return u64::MAX;
        }
        self.masks
            [Self::index(st, scan, left, right, side, near, middle)]
    }
}

/// Cross-side co-occurrence relation for ordered whole-side triples.
///
/// For one exact local window, bit `(l, r)` means that left-side triple `l`
/// and right-side triple `r` can occur together in the same forward abstract
/// execution.  This is strictly stronger than checking the two projected
/// `SideTriplePossible` masks independently.  The domain is intentionally
/// limited to three-color machines: 27 triples per side give 729 pair bits,
/// i.e. only 12 u64 words per exact window.
const JOINT_SIDE_TRIPLE_MAX_COLORS: usize = 3;

struct JointSideTriplePossible<const S: usize, const C: usize> {
    enabled: bool,
    words_per_window: usize,
    bits: Vec<u64>,
}

impl<const S: usize, const C: usize> JointSideTriplePossible<S, C> {
    const fn window_index(
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
    ) -> usize {
        (((st * C) + scan) * C + left) * C + right
    }

    const fn triple_id(a: usize, b: usize, c: usize) -> usize {
        (a * C + b) * C + c
    }

    const fn triple_count() -> usize {
        C * C * C
    }

    fn new() -> Self {
        let enabled = C <= JOINT_SIDE_TRIPLE_MAX_COLORS;
        let triple_count = Self::triple_count();
        let pair_bits = triple_count * triple_count;
        let words_per_window =
            if enabled { pair_bits.div_ceil(64) } else { 0 };
        let windows = S * C * C * C;
        Self {
            enabled,
            words_per_window,
            bits: vec![0; windows * words_per_window],
        }
    }

    const fn word_base(
        &self,
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
    ) -> usize {
        Self::window_index(st, scan, left, right)
            * self.words_per_window
    }

    fn contains_ids(
        &self,
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
        left_id: usize,
        right_id: usize,
    ) -> bool {
        if !self.enabled {
            return true;
        }
        let triple_count = Self::triple_count();
        let pair = left_id * triple_count + right_id;
        let base = self.word_base(st, scan, left, right);
        self.bits[base + pair / 64] & (1_u64 << (pair % 64)) != 0
    }

    fn insert_ids(
        &mut self,
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
        left_id: usize,
        right_id: usize,
    ) -> bool {
        if !self.enabled {
            return false;
        }
        let triple_count = Self::triple_count();
        let pair = left_id * triple_count + right_id;
        let base = self.word_base(st, scan, left, right);
        let word = &mut self.bits[base + pair / 64];
        let bit = 1_u64 << (pair % 64);
        let old = *word;
        *word |= bit;
        *word != old
    }

    #[expect(clippy::too_many_arguments)]
    fn copy_window(
        &mut self,
        src_st: usize,
        src_scan: usize,
        src_left: usize,
        src_right: usize,
        dst_st: usize,
        dst_scan: usize,
        dst_left: usize,
        dst_right: usize,
    ) -> bool {
        if !self.enabled {
            return false;
        }
        let src = self.word_base(src_st, src_scan, src_left, src_right);
        let dst = self.word_base(dst_st, dst_scan, dst_left, dst_right);
        let mut changed = false;
        for offset in 0..self.words_per_window {
            let value = self.bits[src + offset];
            let old = self.bits[dst + offset];
            self.bits[dst + offset] |= value;
            changed |= self.bits[dst + offset] != old;
        }
        changed
    }

    /// Project one exact-window relation to the individual triples currently
    /// known possible on each side.  Because every concrete blank-start tape
    /// has a far-away 000 triple on both sides, the relation has a natural
    /// anchor and these projections safely cover every concrete side triple.
    fn projections(
        &self,
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
    ) -> (u64, u64) {
        if !self.enabled {
            return (u64::MAX, u64::MAX);
        }
        let triple_count = Self::triple_count();
        debug_assert!(triple_count <= 64);
        let mut left_mask = 0_u64;
        let mut right_mask = 0_u64;
        for left_id in 0..triple_count {
            for right_id in 0..triple_count {
                if self.contains_ids(
                    st, scan, left, right, left_id, right_id,
                ) {
                    left_mask |= 1_u64 << left_id;
                    right_mask |= 1_u64 << right_id;
                }
            }
        }
        (left_mask, right_mask)
    }
}

#[expect(clippy::multiple_inherent_impl)]
impl<const S: usize, const C: usize> WinPossible<S, C> {
    /// Replace the coarse exact-window reachability relation with the stronger
    /// fixed point already established by `SidePossible`, then rebuild every
    /// aggregate lookup from the surviving exact windows.
    ///
    /// Exact parity/residue masks are retained only for reachable windows.
    /// Their one-neighbor/unknown-neighbor aggregates must be rebuilt as well;
    /// otherwise a removed exact window could still witness a later query via
    /// `parity_right`, `side_mod3_any`, etc.
    fn refine_reachability(&mut self, sides: &SidePossible<S, C>) {
        self.right = [[[0; C]; C]; S];
        self.left = [[[0; C]; C]; S];
        self.any = [[false; C]; S];

        self.parity_right = [[[0; C]; C]; S];
        self.parity_left = [[[0; C]; C]; S];
        self.parity_any = [[0; C]; S];

        self.side_parity_right = [[[0; C]; C]; S];
        self.side_parity_left = [[[0; C]; C]; S];
        self.side_parity_any = [[0; C]; S];

        self.side_mod3_right = [[[0; C]; C]; S];
        self.side_mod3_left = [[[0; C]; C]; S];
        self.side_mod3_any = [[0; C]; S];

        self.color_parity_right = [[[0; C]; C]; S];
        self.color_parity_left = [[[0; C]; C]; S];
        self.color_parity_any = [[0; C]; S];

        for st in 0..S {
            for scan in 0..C {
                for left in 0..C {
                    for right in 0..C {
                        let index =
                            Self::parity_index(st, scan, left, right);

                        if !sides
                            .window(st, scan, left, right)
                            .reachable
                        {
                            self.parity[index] = 0;
                            self.side_parity[index] = 0;
                            self.side_mod3[index] = 0;
                            self.color_parity[index] = 0;
                            continue;
                        }

                        self.right[st][scan][left] |= 1_u64 << right;
                        self.left[st][scan][right] |= 1_u64 << left;
                        self.any[st][scan] = true;

                        let parity = self.parity[index];
                        self.parity_right[st][scan][left] |= parity;
                        self.parity_left[st][scan][right] |= parity;
                        self.parity_any[st][scan] |= parity;

                        let side_parity = self.side_parity[index];
                        self.side_parity_right[st][scan][left] |=
                            side_parity;
                        self.side_parity_left[st][scan][right] |=
                            side_parity;
                        self.side_parity_any[st][scan] |= side_parity;

                        let side_mod3 = self.side_mod3[index];
                        self.side_mod3_right[st][scan][left] |=
                            side_mod3;
                        self.side_mod3_left[st][scan][right] |=
                            side_mod3;
                        self.side_mod3_any[st][scan] |= side_mod3;

                        let color_parity = self.color_parity[index];
                        self.color_parity_right[st][scan][left] |=
                            color_parity;
                        self.color_parity_left[st][scan][right] |=
                            color_parity;
                        self.color_parity_any[st][scan] |= color_parity;
                    }
                }
            }
        }
    }

    /// Rebuild the cheap transposed/aggregate window relation from `right`.
    ///
    /// Intermediate ordered-prefix refinement only needs exact local-window
    /// reachability.  Parity/count aggregates are deliberately left untouched
    /// until the refinement fixed point is complete.
    fn rebuild_window_relation(&mut self) {
        self.left = [[[0; C]; C]; S];
        self.any = [[false; C]; S];

        for st in 0..S {
            for scan in 0..C {
                for left in 0..C {
                    let mut rights = self.right[st][scan][left];
                    if rights != 0 {
                        self.any[st][scan] = true;
                    }
                    while rights != 0 {
                        let right = rights.trailing_zeros() as usize;
                        rights &= rights - 1;
                        self.left[st][scan][right] |= 1_u64 << left;
                    }
                }
            }
        }
    }

    /// Intersect the exact local-window relation with the stronger same-cell
    /// crossing closure.  The crossing relation contains every frontier
    /// checkpoint and is closed under complete one-sided excursions, so every
    /// concrete visit to a tape cell is represented: its first visit is a
    /// frontier visit, and between consecutive visits the head remains wholly
    /// on one side of that cell.
    fn refine_crossing_reachability_relation(
        &mut self,
        crossing: &[[[u64; C]; C]; S],
    ) -> bool {
        let mut changed = false;

        for st in 0..S {
            for scan in 0..C {
                for left in 0..C {
                    let old = self.right[st][scan][left];
                    let keep = old & crossing[st][scan][left];
                    changed |= keep != old;
                    self.right[st][scan][left] = keep;
                }
            }
        }

        if changed {
            self.rebuild_window_relation();
        }
        changed
    }

    /// Shrink only the exact local-window relation to windows reachable in the
    /// supplied `SidePossible` fixed point.  This is the fast intermediate
    /// counterpart of `refine_reachability`: it does not rebuild parity,
    /// residue, or color-count aggregates.
    fn refine_side_reachability_relation(
        &mut self,
        sides: &SidePossible<S, C>,
    ) -> bool {
        let mut changed = false;

        for st in 0..S {
            for scan in 0..C {
                for left in 0..C {
                    let old = self.right[st][scan][left];
                    if old == 0 {
                        continue;
                    }

                    let mut keep = 0_u64;
                    let mut rights = old;
                    while rights != 0 {
                        let right = rights.trailing_zeros() as usize;
                        rights &= rights - 1;
                        if sides.window(st, scan, left, right).reachable
                        {
                            keep |= 1_u64 << right;
                        }
                    }

                    changed |= keep != old;
                    self.right[st][scan][left] = keep;
                }
            }
        }

        if changed {
            self.rebuild_window_relation();
        }
        changed
    }

    /// Remove exact local windows for which the tiny joint left/right prefix
    /// product has no witness. This feeds correlation discovered beyond both
    /// immediate neighbors back into the existing exact-window fixed point.
    fn refine_joint_short_reachability_relation(
        &mut self,
        joint: &JointShortPossible<S, C>,
    ) -> bool {
        let mut changed = false;

        for st in 0..S {
            for scan in 0..C {
                for left in 0..C {
                    let old = self.right[st][scan][left];
                    if old == 0 {
                        continue;
                    }

                    let mut keep = old;
                    let mut rights = old;
                    while rights != 0 {
                        let right = rights.trailing_zeros() as usize;
                        rights &= rights - 1;
                        if joint
                            .window(st, scan, left, right)
                            .is_empty()
                        {
                            keep &= !(1_u64 << right);
                        }
                    }

                    changed |= keep != old;
                    self.right[st][scan][left] = keep;
                }
            }
        }

        if changed {
            self.rebuild_window_relation();
        }
        changed
    }

    /// Remove exact local windows for which the cheap ordered run-prefix
    /// projection has no witness on at least one side.  This runs before the
    /// richer word-prefix projection so windows eliminated here never pay for
    /// the word fixed point.
    fn refine_run_prefix_reachability_relation(
        &mut self,
        prefixes: &SidePrefixPossible<S, C>,
    ) -> bool {
        let mut changed = false;

        for st in 0..S {
            for scan in 0..C {
                for left in 0..C {
                    let old = self.right[st][scan][left];
                    if old == 0 {
                        continue;
                    }

                    let mut keep = old;
                    let mut rights = old;
                    while rights != 0 {
                        let right = rights.trailing_zeros() as usize;
                        rights &= rights - 1;

                        let left_ok = !prefixes
                            .prefixes(st, scan, left, right, LEFT_SIDE)
                            .is_empty();
                        let right_ok = !prefixes
                            .prefixes(st, scan, left, right, RIGHT_SIDE)
                            .is_empty();

                        if !left_ok || !right_ok {
                            keep &= !(1_u64 << right);
                        }
                    }

                    changed |= keep != old;
                    self.right[st][scan][left] = keep;
                }
            }
        }

        if changed {
            self.rebuild_window_relation();
        }
        changed
    }

    /// Once the run-prefix relation is stable, remove exact local windows for
    /// which the richer ordered cell/word-prefix projection has no witness.
    fn refine_word_prefix_reachability_relation(
        &mut self,
        prefixes: &SidePrefixPossible<S, C>,
    ) -> bool {
        let mut changed = false;

        for st in 0..S {
            for scan in 0..C {
                for left in 0..C {
                    let old = self.right[st][scan][left];
                    if old == 0 {
                        continue;
                    }

                    let mut keep = old;
                    let mut rights = old;
                    while rights != 0 {
                        let right = rights.trailing_zeros() as usize;
                        rights &= rights - 1;

                        let left_ok = !prefixes
                            .word_prefixes(
                                st, scan, left, right, LEFT_SIDE,
                            )
                            .is_empty();
                        let right_ok = !prefixes
                            .word_prefixes(
                                st, scan, left, right, RIGHT_SIDE,
                            )
                            .is_empty();

                        if !left_ok || !right_ok {
                            keep &= !(1_u64 << right);
                        }
                    }

                    changed |= keep != old;
                    self.right[st][scan][left] = keep;
                }
            }
        }

        if changed {
            self.rebuild_window_relation();
        }
        changed
    }

    /// Remove exact local windows for which the parity-aware joint left/right
    /// run-prefix product has no witness. Unlike the independent prefix
    /// refinement above, this keeps the two long side summaries correlated.
    /// A capped `Unknown` alternative still counts as a witness, so overflow
    /// can only disable this refinement rather than make it unsound.
    fn refine_joint_side_prefix_reachability_relation(
        &mut self,
        joint: &JointSidePrefixPossible<S, C>,
    ) -> bool {
        let mut changed = false;

        for st in 0..S {
            for scan in 0..C {
                for left in 0..C {
                    let old = self.right[st][scan][left];
                    if old == 0 {
                        continue;
                    }

                    let mut keep = old;
                    let mut rights = old;
                    while rights != 0 {
                        let right = rights.trailing_zeros() as usize;
                        rights &= rights - 1;
                        if joint
                            .window(st, scan, left, right)
                            .is_empty()
                        {
                            keep &= !(1_u64 << right);
                        }
                    }

                    changed |= keep != old;
                    self.right[st][scan][left] = keep;
                }
            }
        }

        if changed {
            self.rebuild_window_relation();
        }
        changed
    }

    /// Remove exact local windows for which the joint ordered word-prefix
    /// product has no forward witness. This is stronger than checking the two
    /// `word_windows` projections independently because the left and right
    /// alternatives remain paired from the same execution.
    fn refine_joint_side_word_reachability_relation(
        &mut self,
        joint: &JointSideWordPrefixPossible<S, C>,
    ) -> bool {
        let mut changed = false;

        for st in 0..S {
            for scan in 0..C {
                for left in 0..C {
                    let old = self.right[st][scan][left];
                    if old == 0 {
                        continue;
                    }

                    let mut keep = old;
                    let mut rights = old;
                    while rights != 0 {
                        let right = rights.trailing_zeros() as usize;
                        rights &= rights - 1;
                        if joint
                            .window(st, scan, left, right)
                            .is_empty()
                        {
                            keep &= !(1_u64 << right);
                        }
                    }

                    changed |= keep != old;
                    self.right[st][scan][left] = keep;
                }
            }
        }

        if changed {
            self.rebuild_window_relation();
        }
        changed
    }
}

/// Near-to-far run prefix of the tape strictly beyond one immediate neighbor.
///
/// Two complete runs keep exact lengths 1/2/3 and, beyond that, retain
/// run-length parity as `even >= 4` or `odd >= 5`.  The extra spill run uses
/// the same count lattice instead of collapsing to 1/2+, so the third retained
/// run can use the full `ReqRun` min/max/parity test too.
///
/// Beyond the spill, retain the color of one additional run when it is known.
/// This fourth-run color has no count component; when it moves inward, its
/// possible count is split across the existing five count classes.  Under
/// joint-bucket pressure this color is the first precision discarded, falling
/// back exactly to the old blank-vs-dirty far-tail summary.
// Full-run count codes. 1/2/3 are exact; the two larger codes are infinite
// parity classes rather than ordinary lower bounds.
const SIDE_PREFIX_EVEN_MANY: u8 = 4; // even lengths >= 4
const SIDE_PREFIX_ODD_MANY: u8 = 5; // odd lengths >= 5
// Pressure-only widening for the spill run. Unlike exact count 2, this means
// any length >= 2. Ordinary independent prefixes never construct it; the
// joint domain uses it before falling all the way to shape-unknown.
const SIDE_PREFIX_SPILL_MANY: u8 = 6;

fn side_prefix_color_bit(color: Color) -> u64 {
    let color = usize::from(color);
    if color < 64 { 1_u64 << color } else { u64::MAX }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct SidePrefixRun {
    color: Color,
    count: u8,
}

impl SidePrefixRun {
    const EMPTY: Self = Self { color: 0, count: 0 };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum SidePrefixFar {
    Blank,
    // Exact color of the next run, but not its length. `farther_dirty` says
    // whether some nonblank cell is known strictly beyond that run.
    Run { color: Color, farther_dirty: bool },
    // Some nonblank cell exists farther out, but the next run color is lost.
    DirtyUnknown,
}

impl SidePrefixFar {
    const fn definitely_dirty(self) -> bool {
        match self {
            Self::Blank => false,
            Self::Run {
                color,
                farther_dirty,
            } => color != 0 || farther_dirty,
            Self::DirtyUnknown => true,
        }
    }

    const fn from_spill(spill: SidePrefixSpill) -> Self {
        match spill {
            SidePrefixSpill::Blank => Self::Blank,
            SidePrefixSpill::Run { color, far, .. } => Self::Run {
                color,
                farther_dirty: far.definitely_dirty(),
            },
            SidePrefixSpill::DirtyUnknown => Self::DirtyUnknown,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum SidePrefixSpill {
    Blank,
    Run {
        color: Color,
        // Normally the same 1/2/3/even-many/odd-many lattice as
        // `SidePrefixRun`. `SIDE_PREFIX_SPILL_MANY` is a pressure-only 2+
        // widening used by the capped joint domain.
        count: u8,
        // One more run of ordered structure beyond the spill.
        far: SidePrefixFar,
    },
    // The forgotten remainder is definitely dirty, but its next run is lost.
    DirtyUnknown,
}

impl SidePrefixSpill {
    const fn definitely_dirty(self) -> bool {
        match self {
            Self::Blank => false,
            Self::Run { color, far, .. } => {
                color != 0 || far.definitely_dirty()
            },
            Self::DirtyUnknown => true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct SidePrefix {
    runs: [SidePrefixRun; 2],
    len: u8,
    spill: SidePrefixSpill,
}

impl SidePrefix {
    const fn blank() -> Self {
        Self {
            runs: [SidePrefixRun::EMPTY; 2],
            len: 0,
            spill: SidePrefixSpill::Blank,
        }
    }

    const fn dirty_unknown() -> Self {
        Self {
            runs: [SidePrefixRun::EMPTY; 2],
            len: 0,
            spill: SidePrefixSpill::DirtyUnknown,
        }
    }

    fn definitely_dirty(self) -> bool {
        let mut index = 0;
        while index < usize::from(self.len) {
            if self.runs[index].color != 0 {
                return true;
            }
            index += 1;
        }
        self.spill.definitely_dirty()
    }

    const fn fourth_run_color(self) -> Option<Color> {
        match self.spill {
            SidePrefixSpill::Run {
                far: SidePrefixFar::Run { color, .. },
                ..
            } => Some(color),
            _ => None,
        }
    }

    fn far_colors_after_prepend(
        self,
        color: Color,
        next: Self,
        mut far_colors: u64,
    ) -> u64 {
        // Prepending a new run to two retained runs shifts the old fourth run
        // beyond the retained horizon. Preserve its color in the merged far
        // summary rather than losing it when the new spill/fourth pair forms.
        if self.len == 2
            && self.runs[0].color != color
            && let Some(fourth) = self.fourth_run_color()
        {
            far_colors |= side_prefix_color_bit(fourth);
        }

        // DirtyUnknown branches can split into the case where the prepended
        // nonblank accounts for all forgotten dirt. That successor has no
        // forgotten nonblank remainder, so only the infinite blank tail remains.
        if matches!(self.spill, SidePrefixSpill::DirtyUnknown)
            && matches!(next.spill, SidePrefixSpill::Blank)
        {
            return 1;
        }

        far_colors
    }

    const fn far_colors_after_pull(
        self,
        next: Self,
        far_colors: u64,
    ) -> u64 {
        // Pulling from DirtyUnknown may consume its last nonblank cell. The
        // explicit blank successor is the only case where the forgotten
        // remainder is known to have disappeared completely.
        if matches!(self.spill, SidePrefixSpill::DirtyUnknown)
            && matches!(next.spill, SidePrefixSpill::Blank)
        {
            1
        } else {
            far_colors
        }
    }

    fn unknown_pull_color_possible(
        self,
        far_colors: u64,
        color: Color,
    ) -> bool {
        if self.len != 0
            || !matches!(self.spill, SidePrefixSpill::DirtyUnknown)
        {
            return true;
        }

        far_colors & side_prefix_color_bit(color) != 0
    }

    /// Denotational subsumption used by the regression tests for the broad
    /// antichain state. No structural-prefix heuristic is used.
    fn subsumes(self, other: Self) -> bool {
        self == other
            || (matches!(self.spill, SidePrefixSpill::DirtyUnknown)
                && self.len == 0
                && other.definitely_dirty())
    }

    fn canonicalize(&mut self) {
        // A zero spill followed by an all-blank remainder is itself just blank
        // remainder.  Retain a zero spill only when it separates the retained
        // runs from known farther dirt.
        if matches!(
            self.spill,
            SidePrefixSpill::Run {
                color: 0,
                far: SidePrefixFar::Blank,
                ..
            }
        ) {
            self.spill = SidePrefixSpill::Blank;
        }

        // Finite zero runs immediately followed by an all-blank remainder are
        // likewise redundant.  This keeps the exact blank alternative unique.
        while matches!(self.spill, SidePrefixSpill::Blank)
            && self.len != 0
        {
            let far = usize::from(self.len) - 1;
            if self.runs[far].color != 0 {
                break;
            }
            self.runs[far] = SidePrefixRun::EMPTY;
            self.len -= 1;
        }
    }

    const fn spill_after_dropped(
        dropped: SidePrefixRun,
        old: SidePrefixSpill,
    ) -> SidePrefixSpill {
        SidePrefixSpill::Run {
            color: dropped.color,
            count: dropped.count,
            far: SidePrefixFar::from_spill(old),
        }
    }

    /// Once the spill is consumed, an exact fourth-run color becomes the new
    /// spill color but its length was deliberately not tracked. Split that
    /// unknown positive length across the existing finite count partition.
    fn for_each_suffix_after_spill(
        far: SidePrefixFar,
        mut emit: impl FnMut(SidePrefixSpill),
    ) {
        match far {
            SidePrefixFar::Blank => emit(SidePrefixSpill::Blank),
            SidePrefixFar::DirtyUnknown => {
                emit(SidePrefixSpill::DirtyUnknown);
            },
            SidePrefixFar::Run {
                color,
                farther_dirty,
            } => {
                let next_far = if farther_dirty {
                    SidePrefixFar::DirtyUnknown
                } else {
                    SidePrefixFar::Blank
                };
                for count in [
                    1,
                    2,
                    3,
                    SIDE_PREFIX_EVEN_MANY,
                    SIDE_PREFIX_ODD_MANY,
                ] {
                    emit(SidePrefixSpill::Run {
                        color,
                        count,
                        far: next_far,
                    });
                }
            },
        }
    }

    /// First pressure widening for the capped joint run product. Forget only
    /// the fourth-run color, retaining the old exact blank-vs-dirty fact.
    const fn widen_far_color(mut self) -> Self {
        if let SidePrefixSpill::Run { far, .. } = &mut self.spill
            && matches!(*far, SidePrefixFar::Run { .. })
        {
            *far = SidePrefixFar::DirtyUnknown;
        }
        self
    }

    /// Pressure widening used only by the capped joint run product. Preserve
    /// the spill color/far-tail fact but join every count >= 2 back to the
    /// original coarse 2+ state. The independent run domain remains precise.
    const fn widen_spill_count(mut self) -> Self {
        if let SidePrefixSpill::Run { count, .. } = &mut self.spill
            && *count != 1
        {
            *count = SIDE_PREFIX_SPILL_MANY;
        }
        self
    }

    /// Prepend one exact cell at the near end of the represented tail.
    /// Successors are emitted directly to avoid allocating a temporary `Vec`
    /// for every abstract edge in the fixed point.
    #[expect(clippy::excessive_nesting)]
    fn for_each_prepend(
        self,
        color: Color,
        mut emit: impl FnMut(Self),
    ) {
        if self.len == 0 {
            match self.spill {
                SidePrefixSpill::Blank => {
                    if color == 0 {
                        emit(self);
                        return;
                    }

                    emit(Self {
                        runs: [
                            SidePrefixRun { color, count: 1 },
                            SidePrefixRun::EMPTY,
                        ],
                        len: 1,
                        spill: SidePrefixSpill::Blank,
                    });
                    return;
                },
                SidePrefixSpill::Run {
                    color: spill_color,
                    count,
                    far,
                } => {
                    if color == spill_color {
                        // The exact prepended cell merges into the known spill
                        // run. Precise spill counts promote deterministically.
                        // A pressure-widened 2+ spill has the old three-way
                        // image: after adding one cell its length is any >= 3.
                        Self::for_each_suffix_after_spill(
                            far,
                            |suffix| {
                                if count == SIDE_PREFIX_SPILL_MANY {
                                    for next_count in [
                                        3,
                                        SIDE_PREFIX_EVEN_MANY,
                                        SIDE_PREFIX_ODD_MANY,
                                    ] {
                                        emit(Self {
                                            runs: [
                                                SidePrefixRun {
                                                    color,
                                                    count: next_count,
                                                },
                                                SidePrefixRun::EMPTY,
                                            ],
                                            len: 1,
                                            spill: suffix,
                                        });
                                    }
                                } else {
                                    let next_count = match count {
                                        1 => 2,
                                        2 => 3,
                                        3 => SIDE_PREFIX_EVEN_MANY,
                                        SIDE_PREFIX_EVEN_MANY => {
                                            SIDE_PREFIX_ODD_MANY
                                        },
                                        SIDE_PREFIX_ODD_MANY => {
                                            SIDE_PREFIX_EVEN_MANY
                                        },
                                        _ => unreachable!(),
                                    };
                                    emit(Self {
                                        runs: [
                                            SidePrefixRun {
                                                color,
                                                count: next_count,
                                            },
                                            SidePrefixRun::EMPTY,
                                        ],
                                        len: 1,
                                        spill: suffix,
                                    });
                                }
                            },
                        );
                        return;
                    }

                    // The prepended cell starts a new exact run.  Keep the old
                    // spill as the next known run instead of forgetting it.
                    emit(Self {
                        runs: [
                            SidePrefixRun { color, count: 1 },
                            SidePrefixRun::EMPTY,
                        ],
                        len: 1,
                        spill: self.spill,
                    });
                    return;
                },
                SidePrefixSpill::DirtyUnknown => {
                    // The old remainder is known dirty but its near run is
                    // forgotten. Conservatively enumerate every capped length
                    // of the new run. A nonzero new run may consume the last
                    // dirty cells, so both blank and dirty residuals are
                    // possible. A zero run cannot account for the old dirt.
                    for count in [
                        1,
                        2,
                        3,
                        SIDE_PREFIX_EVEN_MANY,
                        SIDE_PREFIX_ODD_MANY,
                    ] {
                        let mut dirty = Self {
                            runs: [
                                SidePrefixRun { color, count },
                                SidePrefixRun::EMPTY,
                            ],
                            len: 1,
                            spill: SidePrefixSpill::DirtyUnknown,
                        };
                        dirty.canonicalize();
                        emit(dirty);

                        if color != 0 {
                            let mut blank = dirty;
                            blank.spill = SidePrefixSpill::Blank;
                            blank.canonicalize();
                            emit(blank);
                        }
                    }
                    return;
                },
            }
        }

        let mut out = self;
        #[expect(clippy::match_same_arms)]
        if out.runs[0].color == color {
            out.runs[0].count = match out.runs[0].count {
                1 => 2,
                2 => 3,
                3 => SIDE_PREFIX_EVEN_MANY,
                SIDE_PREFIX_EVEN_MANY => SIDE_PREFIX_ODD_MANY,
                SIDE_PREFIX_ODD_MANY => SIDE_PREFIX_EVEN_MANY,
                _ => unreachable!(),
            };
            emit(out);
            return;
        }

        let new_near = SidePrefixRun { color, count: 1 };
        if out.len == 1 {
            out.runs[1] = out.runs[0];
            out.runs[0] = new_near;
            out.len = 2;
            out.canonicalize();
            emit(out);
            return;
        }

        debug_assert_eq!(out.len, 2);
        let dropped = out.runs[1];
        out.runs[1] = out.runs[0];
        out.runs[0] = new_near;

        // Keep one more run boundary instead of immediately degrading to a
        // blank/dirty bit. Only structure strictly beyond this spill is joined.
        out.spill = Self::spill_after_dropped(dropped, out.spill);
        out.canonicalize();
        emit(out);
    }

    /// Consume the nearest represented tail cell, emitting the exposed color
    /// and residual prefix directly without a temporary allocation.
    fn for_each_pull<const C: usize>(
        self,
        mut emit: impl FnMut(Color, Self),
    ) {
        if self.len == 0 {
            match self.spill {
                SidePrefixSpill::Blank => {
                    emit(0, self);
                },
                SidePrefixSpill::Run { color, count, far } => {
                    let residual = |next_count| {
                        let mut next = Self {
                            runs: [SidePrefixRun::EMPTY; 2],
                            len: 0,
                            spill: SidePrefixSpill::Run {
                                color,
                                count: next_count,
                                far,
                            },
                        };
                        next.canonicalize();
                        next
                    };

                    match count {
                        1 => Self::for_each_suffix_after_spill(
                            far,
                            |spill| {
                                let mut next = Self {
                                    runs: [SidePrefixRun::EMPTY; 2],
                                    len: 0,
                                    spill,
                                };
                                next.canonicalize();
                                emit(color, next);
                            },
                        ),
                        2 => emit(color, residual(1)),
                        3 => emit(color, residual(2)),
                        SIDE_PREFIX_EVEN_MANY => {
                            // even >= 4 minus one is either exact 3 (source 4)
                            // or odd >= 5.
                            emit(color, residual(3));
                            emit(color, residual(SIDE_PREFIX_ODD_MANY));
                        },
                        SIDE_PREFIX_ODD_MANY => {
                            emit(
                                color,
                                residual(SIDE_PREFIX_EVEN_MANY),
                            );
                        },
                        SIDE_PREFIX_SPILL_MANY => {
                            // 2+ minus one is either exactly one or still 2+.
                            emit(color, residual(1));
                            emit(
                                color,
                                residual(SIDE_PREFIX_SPILL_MANY),
                            );
                        },
                        _ => unreachable!(),
                    }
                },
                SidePrefixSpill::DirtyUnknown => {
                    // The forgotten remainder is dirty. Consuming a zero cannot
                    // remove that dirt. Consuming a nonzero may remove the last
                    // nonblank or may leave more dirt farther out.
                    for color in 0..C {
                        #[expect(clippy::cast_possible_truncation)]
                        let color = color as Color;
                        if color != 0 {
                            emit(color, Self::blank());
                        }
                        emit(color, Self::dirty_unknown());
                    }
                },
            }
            return;
        }

        let color = self.runs[0].color;
        let count = self.runs[0].count;

        let residual = |new_count: Option<u8>| {
            let mut next = self;
            if let Some(count) = new_count {
                next.runs[0].count = count;
            } else {
                next.runs[0] = next.runs[1];
                next.runs[1] = SidePrefixRun::EMPTY;
                next.len -= 1;
            }
            next.canonicalize();
            next
        };

        match count {
            1 => emit(color, residual(None)),
            2 | 3 => emit(color, residual(Some(count - 1))),
            // even >= 4 minus one is either exact 3 (when the source was 4)
            // or odd >= 5. Keep both cases explicitly.
            SIDE_PREFIX_EVEN_MANY => {
                emit(color, residual(Some(3)));
                emit(color, residual(Some(SIDE_PREFIX_ODD_MANY)));
            },
            // odd >= 5 minus one is always even >= 4.
            SIDE_PREFIX_ODD_MANY => {
                emit(color, residual(Some(SIDE_PREFIX_EVEN_MANY)));
            },
            _ => unreachable!(),
        }
    }
}

/// Ordered cell-prefix companion to `SidePrefix`. The ordinary run abstraction
/// remains useful for long homogeneous runs; this second domain retains short
/// periodic words such as `(1 2)+` instead of degrading them to a dirty spill
/// after only a few run boundaries.
///
/// `Periodic` is deliberately a widening. `min_cells` cells are guaranteed to
/// follow the stored primitive cycle from `phase`; any additional full cycles
/// may occur before `tail`. Widening an exact observed prefix to that family is
/// a sound over-approximation, and bounding/canonicalizing `min_cells` keeps the
/// forward fixed point finite.
const SIDE_WORD_LITERAL_CELLS: usize = 24;
const SIDE_WORD_MAX_PATTERN: usize = 8;
const SIDE_WORD_MIN_REPEATS: usize = 3;
// Bound the exact alternative antichain at each exact window/side. When it
// overflows, join the alternatives to their longest common guaranteed cell
// prefix instead of enumerating an exponential family of 24-cell literals.
// The joined unknown suffix is an over-approximation, so this can only weaken
// the extra word-prefix proof domain, never make it unsound.
const SIDE_WORD_MAX_ALTS_PER_WINDOW: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum SideWordTail {
    Blank,
    DirtyUnknown,
    Unknown,
}

impl SideWordTail {
    const fn definitely_dirty(self) -> bool {
        matches!(self, Self::DirtyUnknown)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum SideWordPrefix {
    Literal {
        cells: [Color; SIDE_WORD_LITERAL_CELLS],
        len: u8,
        tail: SideWordTail,
    },
    Periodic {
        word: [Color; SIDE_WORD_MAX_PATTERN],
        word_len: u8,
        phase: u8,
        min_cells: u16,
        tail: SideWordTail,
    },
}

impl SideWordPrefix {
    const fn blank() -> Self {
        Self::Literal {
            cells: [0; SIDE_WORD_LITERAL_CELLS],
            len: 0,
            tail: SideWordTail::Blank,
        }
    }

    const fn dirty_unknown() -> Self {
        Self::Literal {
            cells: [0; SIDE_WORD_LITERAL_CELLS],
            len: 0,
            tail: SideWordTail::DirtyUnknown,
        }
    }

    const fn unknown() -> Self {
        Self::Literal {
            cells: [0; SIDE_WORD_LITERAL_CELLS],
            len: 0,
            tail: SideWordTail::Unknown,
        }
    }

    const fn is_blank(self) -> bool {
        matches!(
            self,
            Self::Literal {
                len: 0,
                tail: SideWordTail::Blank,
                ..
            }
        )
    }

    const fn is_dirty_unknown(self) -> bool {
        matches!(
            self,
            Self::Literal {
                len: 0,
                tail: SideWordTail::DirtyUnknown,
                ..
            }
        )
    }

    const fn is_unconstrained(self) -> bool {
        matches!(
            self,
            Self::Literal {
                len: 0,
                tail: SideWordTail::Unknown,
                ..
            }
        )
    }

    fn definitely_dirty(self) -> bool {
        match self {
            Self::Literal { cells, len, tail } => {
                cells[..usize::from(len)]
                    .iter()
                    .any(|&color| color != 0)
                    || tail.definitely_dirty()
            },
            Self::Periodic {
                word,
                word_len,
                min_cells,
                tail,
                ..
            } => {
                min_cells != 0
                    && word[..usize::from(word_len)]
                        .iter()
                        .any(|&color| color != 0)
                    || tail.definitely_dirty()
            },
        }
    }

    fn guaranteed_len(self) -> usize {
        match self {
            Self::Literal { len, .. } => usize::from(len),
            Self::Periodic { min_cells, .. } => usize::from(min_cells),
        }
    }

    fn guaranteed_cell(self, index: usize) -> Option<Color> {
        match self {
            Self::Literal { cells, len, .. } => {
                (index < usize::from(len)).then_some(cells[index])
            },
            Self::Periodic {
                word,
                word_len,
                phase,
                min_cells,
                ..
            } => {
                if index >= usize::from(min_cells) {
                    return None;
                }
                let width = usize::from(word_len);
                Some(word[(usize::from(phase) + index) % width])
            },
        }
    }

    /// Compare the first `count` cells guaranteed by two prefixes.
    ///
    /// Hot antichain checks call this many times.  Specializing the four
    /// representation pairs avoids repeated enum dispatch through
    /// `guaranteed_cell` and avoids `% word_len` for every periodic cell.
    fn same_guaranteed_prefix(self, other: Self, count: usize) -> bool {
        debug_assert!(count <= self.guaranteed_len());
        debug_assert!(count <= other.guaranteed_len());

        match (self, other) {
            (
                Self::Literal { cells: a, .. },
                Self::Literal { cells: b, .. },
            ) => a[..count] == b[..count],
            (
                Self::Literal { cells, .. },
                Self::Periodic {
                    word,
                    word_len,
                    phase,
                    ..
                },
            ) => {
                let width = usize::from(word_len);
                let mut wi = usize::from(phase);
                debug_assert!(wi < width);

                for &cell in &cells[..count] {
                    if cell != word[wi] {
                        return false;
                    }
                    wi += 1;
                    if wi == width {
                        wi = 0;
                    }
                }
                true
            },
            (
                Self::Periodic {
                    word,
                    word_len,
                    phase,
                    ..
                },
                Self::Literal { cells, .. },
            ) => {
                let width = usize::from(word_len);
                let mut wi = usize::from(phase);
                debug_assert!(wi < width);

                for &cell in &cells[..count] {
                    if word[wi] != cell {
                        return false;
                    }
                    wi += 1;
                    if wi == width {
                        wi = 0;
                    }
                }
                true
            },
            (
                Self::Periodic {
                    word: a_word,
                    word_len: a_len,
                    phase: a_phase,
                    ..
                },
                Self::Periodic {
                    word: b_word,
                    word_len: b_len,
                    phase: b_phase,
                    ..
                },
            ) => {
                let a_width = usize::from(a_len);
                let b_width = usize::from(b_len);
                let mut ai = usize::from(a_phase);
                let mut bi = usize::from(b_phase);
                debug_assert!(ai < a_width);
                debug_assert!(bi < b_width);

                // Identical primitive cycle and phase is a common antichain
                // case; avoid walking the guaranteed cells at all.
                if a_width == b_width
                    && ai == bi
                    && a_word[..a_width] == b_word[..a_width]
                {
                    return true;
                }

                for _ in 0..count {
                    if a_word[ai] != b_word[bi] {
                        return false;
                    }
                    ai += 1;
                    if ai == a_width {
                        ai = 0;
                    }
                    bi += 1;
                    if bi == b_width {
                        bi = 0;
                    }
                }
                true
            },
        }
    }

    /// Equality of the represented prefix state, ignoring unused array cells.
    /// This replaces derived `self == other` in the hot subsumption path, where
    /// comparing all 24 literal cells is unnecessary for short literals.
    fn same_representation(self, other: Self) -> bool {
        match (self, other) {
            (
                Self::Literal {
                    cells: a,
                    len: a_len,
                    tail: a_tail,
                },
                Self::Literal {
                    cells: b,
                    len: b_len,
                    tail: b_tail,
                },
            ) => {
                a_len == b_len
                    && a_tail == b_tail
                    && a[..usize::from(a_len)]
                        == b[..usize::from(a_len)]
            },
            (
                Self::Periodic {
                    word: a_word,
                    word_len: a_len,
                    phase: a_phase,
                    min_cells: a_min,
                    tail: a_tail,
                },
                Self::Periodic {
                    word: b_word,
                    word_len: b_len,
                    phase: b_phase,
                    min_cells: b_min,
                    tail: b_tail,
                },
            ) => {
                a_len == b_len
                    && a_phase == b_phase
                    && a_min == b_min
                    && a_tail == b_tail
                    && a_word[..usize::from(a_len)]
                        == b_word[..usize::from(a_len)]
            },
            _ => false,
        }
    }

    fn periodic_min(width: usize, len: usize) -> u16 {
        let threshold = SIDE_WORD_MIN_REPEATS * width;
        let residue = len % width;
        let minimum = threshold + residue;
        debug_assert!(minimum <= len);
        u16::try_from(minimum).unwrap_or(u16::MAX)
    }

    fn canonical_at_least_min(width: usize, min_cells: usize) -> u16 {
        let threshold = SIDE_WORD_MIN_REPEATS * width;
        #[expect(clippy::int_plus_one)]
        if min_cells <= threshold + width - 1 {
            return u16::try_from(min_cells).unwrap_or(u16::MAX);
        }

        let residue = min_cells % width;
        u16::try_from(threshold + residue).unwrap_or(u16::MAX)
    }

    fn from_literal(
        cells: [Color; SIDE_WORD_LITERAL_CELLS],
        len: usize,
        tail: SideWordTail,
    ) -> Self {
        debug_assert!(len <= SIDE_WORD_LITERAL_CELLS);

        // Width one is intentionally left to the original run-prefix domain.
        // Search the smallest nontrivial period so the stored cycle is
        // primitive automatically. Comparing the shifted slices is equivalent
        // to `cells[index] == cells[index % width]`, but avoids a remainder
        // operation for every cell of every candidate width.
        for width in
            2..=SIDE_WORD_MAX_PATTERN.min(len / SIDE_WORD_MIN_REPEATS)
        {
            if cells[width..len] == cells[..len - width] {
                // Reject a homogeneous pseudo-word. A smaller width-one period
                // would only duplicate the existing run abstraction.
                if cells[..width].iter().all(|&color| color == cells[0])
                {
                    continue;
                }

                let mut word = [0; SIDE_WORD_MAX_PATTERN];
                word[..width].copy_from_slice(&cells[..width]);
                return Self::Periodic {
                    word,
                    word_len: u8::try_from(width)
                        .expect("side word width fits in u8"),
                    phase: 0,
                    min_cells: Self::periodic_min(width, len),
                    tail,
                };
            }
        }

        Self::Literal {
            cells,
            len: u8::try_from(len)
                .expect("side word literal length fits in u8"),
            tail,
        }
    }

    /// Fast path for prepending to a retained, non-full literal.
    ///
    /// The old literal has already failed every period width eligible at its
    /// old length. If prepending one cell makes the new sequence periodic with
    /// width `w`, removing that first cell leaves a suffix with the same period.
    /// Therefore the only width worth testing is one that becomes eligible at
    /// this exact length. With the three-repeat promotion threshold, that is
    /// `len / 3` exactly when `len` is divisible by three.
    fn from_literal_after_prepend(
        cells: [Color; SIDE_WORD_LITERAL_CELLS],
        len: usize,
        tail: SideWordTail,
    ) -> Self {
        debug_assert!(0 < len && len <= SIDE_WORD_LITERAL_CELLS);

        if len.is_multiple_of(SIDE_WORD_MIN_REPEATS) {
            let width = len / SIDE_WORD_MIN_REPEATS;
            if (2..=SIDE_WORD_MAX_PATTERN).contains(&width)
                && cells[width..len] == cells[..len - width]
                && !cells[..width]
                    .iter()
                    .all(|&color| color == cells[0])
            {
                let mut word = [0; SIDE_WORD_MAX_PATTERN];
                word[..width].copy_from_slice(&cells[..width]);
                return Self::Periodic {
                    word,
                    word_len: u8::try_from(width)
                        .expect("side word width fits in u8"),
                    phase: 0,
                    min_cells: u16::try_from(len)
                        .expect("side word literal length fits in u16"),
                    tail,
                };
            }
        }

        Self::Literal {
            cells,
            len: u8::try_from(len)
                .expect("side word literal length fits in u8"),
            tail,
        }
    }

    #[expect(clippy::match_same_arms)]
    const fn tail_after_dropped(
        color: Color,
        tail: SideWordTail,
    ) -> SideWordTail {
        match tail {
            SideWordTail::Unknown => SideWordTail::Unknown,
            SideWordTail::DirtyUnknown => SideWordTail::DirtyUnknown,
            SideWordTail::Blank if color == 0 => SideWordTail::Blank,
            SideWordTail::Blank => SideWordTail::DirtyUnknown,
        }
    }

    fn periodic_to_literal_with_prepend(
        word: [Color; SIDE_WORD_MAX_PATTERN],
        word_len: u8,
        phase: u8,
        min_cells: u16,
        color: Color,
    ) -> Self {
        let width = usize::from(word_len);
        let mut cells = [0; SIDE_WORD_LITERAL_CELLS];
        cells[0] = color;
        let take =
            (SIDE_WORD_LITERAL_CELLS - 1).min(usize::from(min_cells));
        for index in 0..take {
            cells[index + 1] =
                word[(usize::from(phase) + index) % width];
        }

        // A broken periodic boundary can have the variable repetition stop at
        // several farther positions. Retain its exact near cells but join the
        // remainder conservatively.
        Self::from_literal(cells, take + 1, SideWordTail::Unknown)
    }

    /// Prepend one exact cell. Unlike pulling, prepending never branches, so
    /// return the unique successor directly instead of routing it through a
    /// callback (and, at joint call sites, a temporary one-element `Vec`).
    fn prepend(self, color: Color) -> Self {
        match self {
            Self::Literal { cells, len, tail } => {
                let len = usize::from(len);
                if len == 0 && tail == SideWordTail::Blank && color == 0
                {
                    return self;
                }

                if len < SIDE_WORD_LITERAL_CELLS {
                    let mut next = [0; SIDE_WORD_LITERAL_CELLS];
                    next[0] = color;
                    next[1..=len].copy_from_slice(&cells[..len]);
                    return Self::from_literal_after_prepend(
                        next,
                        len + 1,
                        tail,
                    );
                }

                let dropped = cells[SIDE_WORD_LITERAL_CELLS - 1];
                let mut next = [0; SIDE_WORD_LITERAL_CELLS];
                next[0] = color;
                next[1..].copy_from_slice(
                    &cells[..SIDE_WORD_LITERAL_CELLS - 1],
                );
                let next_tail = Self::tail_after_dropped(dropped, tail);
                Self::from_literal(
                    next,
                    SIDE_WORD_LITERAL_CELLS,
                    next_tail,
                )
            },
            Self::Periodic {
                word,
                word_len,
                phase,
                min_cells,
                tail,
            } => {
                let width = usize::from(word_len);
                let previous = (usize::from(phase) + width - 1) % width;
                if color == word[previous] {
                    let next_min =
                        usize::from(min_cells).saturating_add(1);
                    Self::Periodic {
                        word,
                        word_len,
                        phase: u8::try_from(previous)
                            .expect("side word phase fits in u8"),
                        min_cells: Self::canonical_at_least_min(
                            width, next_min,
                        ),
                        tail,
                    }
                } else {
                    Self::periodic_to_literal_with_prepend(
                        word, word_len, phase, min_cells, color,
                    )
                }
            },
        }
    }

    fn emit_tail<const C: usize>(
        tail: SideWordTail,
        mut emit: impl FnMut(Color, Self),
    ) {
        match tail {
            SideWordTail::Blank => emit(0, Self::blank()),
            SideWordTail::DirtyUnknown => {
                for color in 0..C {
                    #[expect(clippy::cast_possible_truncation)]
                    let color = color as Color;
                    if color != 0 {
                        emit(color, Self::blank());
                    }
                    emit(color, Self::dirty_unknown());
                }
            },
            SideWordTail::Unknown => {
                for color in 0..C {
                    #[expect(clippy::cast_possible_truncation)]
                    emit(color as Color, Self::unknown());
                }
            },
        }
    }

    fn for_each_pull<const C: usize>(
        self,
        mut emit: impl FnMut(Color, Self),
    ) {
        match self {
            Self::Literal { cells, len, tail } => {
                let len = usize::from(len);
                if len == 0 {
                    Self::emit_tail::<C>(tail, emit);
                    return;
                }

                let color = cells[0];
                if len == 1 {
                    let residual = match tail {
                        SideWordTail::Blank => Self::blank(),
                        SideWordTail::DirtyUnknown => {
                            Self::dirty_unknown()
                        },
                        SideWordTail::Unknown => Self::unknown(),
                    };
                    emit(color, residual);
                    return;
                }

                let mut next = [0; SIDE_WORD_LITERAL_CELLS];
                next[..len - 1].copy_from_slice(&cells[1..len]);
                emit(color, Self::from_literal(next, len - 1, tail));
            },
            Self::Periodic {
                word,
                word_len,
                phase,
                min_cells,
                tail,
            } => {
                let width = usize::from(word_len);
                let color = word[usize::from(phase)];
                let next_phase = (usize::from(phase) + 1) % width;
                let minimum = usize::from(min_cells);

                if minimum > 1 {
                    emit(
                        color,
                        Self::Periodic {
                            word,
                            word_len,
                            phase: u8::try_from(next_phase)
                                .expect("side word phase fits in u8"),
                            min_cells: Self::canonical_at_least_min(
                                width,
                                minimum - 1,
                            ),
                            tail,
                        },
                    );
                    return;
                }

                // `AtLeast(1 + k*width)` splits after one pull into the case
                // k=0 (the periodic segment ended) and k>=1 (at least one
                // whole cycle remains). This is the word-level counterpart of
                // pulling from a `1..` monochromatic run.
                let residual = match tail {
                    SideWordTail::Blank => Self::blank(),
                    SideWordTail::DirtyUnknown => Self::dirty_unknown(),
                    SideWordTail::Unknown => Self::unknown(),
                };
                emit(color, residual);
                emit(
                    color,
                    Self::Periodic {
                        word,
                        word_len,
                        phase: u8::try_from(next_phase)
                            .expect("side word phase fits in u8"),
                        min_cells: u16::try_from(width)
                            .expect("side word width fits in u16"),
                        tail,
                    },
                );
            },
        }
    }

    /// Denotational subsumption used to keep each exact window/side as a
    /// small antichain. In particular, a retained exact prefix followed by an
    /// unknown tail covers every more-specific continuation with that prefix.
    fn subsumes(self, other: Self) -> bool {
        if self.is_unconstrained() || self.same_representation(other) {
            return true;
        }

        if self.is_dirty_unknown() {
            return other.definitely_dirty();
        }

        #[expect(clippy::match_like_matches_macro)]
        let unknown_after_guarantee = match self {
            Self::Literal {
                tail: SideWordTail::Unknown,
                ..
            }
            | Self::Periodic {
                tail: SideWordTail::Unknown,
                ..
            } => true,
            _ => false,
        };
        if !unknown_after_guarantee {
            return false;
        }

        let required = self.guaranteed_len();
        if other.guaranteed_len() < required {
            return false;
        }

        self.same_guaranteed_prefix(other, required)
    }

    fn prefix_compatible(self, other: Self) -> bool {
        if self.is_unconstrained() || other.is_unconstrained() {
            return true;
        }

        let common = self.guaranteed_len().min(other.guaranteed_len());
        if !self.same_guaranteed_prefix(other, common) {
            return false;
        }

        // This deliberately checks only jointly guaranteed cells. Optional
        // cycles and unknown tails are existential, so comparing farther would
        // require a full regular-language intersection. The guaranteed prefix
        // alone is already much stronger than the old two-run horizon and is
        // sufficient to reject incompatible repeated words safely.
        true
    }
}

fn side_word_requirement(span: &Span) -> SideWordPrefix {
    let mut cells = [0; SIDE_WORD_LITERAL_CELLS];
    let mut len = 0_usize;
    let mut skip = 1_usize;

    let append = |cells: &mut [Color; SIDE_WORD_LITERAL_CELLS],
                  len: &mut usize,
                  color: Color|
     -> bool {
        if *len == SIDE_WORD_LITERAL_CELLS {
            return false;
        }
        cells[*len] = color;
        *len += 1;
        true
    };

    for block in span.span.iter() {
        match block {
            Block::Run { color, count } => {
                let minimum = usize::from(count.minimum());
                let start = skip.min(minimum);
                skip -= start;
                for _ in start..minimum {
                    if !append(&mut cells, &mut len, *color) {
                        return SideWordPrefix::from_literal(
                            cells,
                            len,
                            SideWordTail::Unknown,
                        );
                    }
                }
                if count.is_indef() {
                    return SideWordPrefix::from_literal(
                        cells,
                        len,
                        SideWordTail::Unknown,
                    );
                }
            },
            Block::Word { word, count } => {
                let width = word.len();
                let minimum = count.minimum().saturating_mul(width);
                let start = skip.min(minimum);
                skip -= start;
                for offset in start..minimum {
                    if !append(
                        &mut cells,
                        &mut len,
                        word[offset % width],
                    ) {
                        return SideWordPrefix::from_literal(
                            cells,
                            len,
                            SideWordTail::Unknown,
                        );
                    }
                }
                if count.is_indef() {
                    return SideWordPrefix::from_literal(
                        cells,
                        len,
                        SideWordTail::Unknown,
                    );
                }
            },
        }
    }

    if skip != 0 {
        return match span.end {
            TapeEnd::Blanks => SideWordPrefix::blank(),
            TapeEnd::Unknown => SideWordPrefix::unknown(),
        };
    }

    let tail = match span.end {
        TapeEnd::Blanks => SideWordTail::Blank,
        TapeEnd::Unknown => SideWordTail::Unknown,
    };
    SideWordPrefix::from_literal(cells, len, tail)
}

// Shared by independent and same-witness joint run-prefix checks.
#[derive(Clone, Copy)]
struct ReqRun {
    color: Color,
    min: u16,
    max: Option<u16>,
    // Exact run-count parity when the backward description proves it.
    // Stride runs with even step retain this bit.
    parity: Option<u8>,
}

impl ReqRun {
    const EMPTY: Self = Self {
        color: 0,
        min: 0,
        max: Some(0),
        parity: Some(0),
    };
}

#[derive(Clone, Copy)]
struct Requirement {
    // The two full runs plus the spill run retain full count information.
    runs: [ReqRun; 3],

    // The forward domain keeps only the color of one additional run, so the
    // backward requirement needs no fourth count/parity descriptor either.
    fourth_color: Option<Color>,

    // Number of explicit residual runs, capped at four.
    run_count: u8,

    // Bit i says some explicit nonblank residual run occurs at position i or
    // farther, for i in 0..=4. Position 4 summarizes everything strictly
    // beyond the fourth retained run.
    suffix_nonblank: u8,

    // `suffix_colors[i]` is the union of explicit residual-run colors at
    // position i or farther. The joint forward domain compares this against
    // its merged forgotten-tail color mask; independent matching ignores it.
    suffix_colors: [u64; 5],
    end_unknown: bool,
}

impl Requirement {
    const EMPTY: Self = Self {
        runs: [ReqRun::EMPTY; 3],
        fourth_color: None,
        run_count: 0,
        suffix_nonblank: 0,
        suffix_colors: [0; 5],
        end_unknown: false,
    };

    const fn unconstrained(self) -> bool {
        self.run_count == 0 && self.end_unknown
    }
}

#[derive(Clone, Copy)]
struct RequirementSet {
    reqs: [Requirement; 2],
    len: u8,
    unconstrained: bool,
}

#[derive(Default)]
struct SideMatchRequirements {
    runs: Option<[RequirementSet; 2]>,
    words: Option<[SideWordPrefix; 2]>,
}

impl SideMatchRequirements {
    fn runs(&mut self, tape: &Tape) -> [RequirementSet; 2] {
        *self.runs.get_or_insert_with(|| {
            [
                side_run_requirements(&tape.lspan),
                side_run_requirements(&tape.rspan),
            ]
        })
    }

    fn words(&mut self, tape: &Tape) -> [SideWordPrefix; 2] {
        *self.words.get_or_insert_with(|| {
            [
                side_word_requirement(&tape.lspan),
                side_word_requirement(&tape.rspan),
            ]
        })
    }
}

#[derive(Clone, Copy)]
enum FirstResidual {
    Drop,
    Replace(BlockCount),
    ConsumeWord,
}

fn req_run(color: Color, count: BlockCount) -> ReqRun {
    match count {
        BlockCount::Exact(count) => ReqRun {
            color,
            min: u16::from(count),
            max: Some(u16::from(count)),
            parity: Some(count & 1),
        },
        BlockCount::AtLeast(count) => ReqRun {
            color,
            min: u16::from(count),
            max: None,
            parity: None,
        },
        BlockCount::Stride { min, step } => ReqRun {
            color,
            min: u16::from(min),
            max: None,
            parity: (step & 1 == 0).then_some(min & 1),
        },
    }
}

#[expect(clippy::cast_possible_truncation)]
fn side_run_requirements(span: &Span) -> RequirementSet {
    #[derive(Clone, Copy)]
    struct Builder {
        req: Requirement,
        runs: usize,
        last_color: Option<Color>,
        blocked: bool,
    }

    fn exact_bounds(value: usize) -> (u16, Option<u16>) {
        u16::try_from(value)
            .map_or((u16::MAX, None), |value| (value, Some(value)))
    }

    fn emit(
        builder: &mut Builder,
        color: Color,
        min: u16,
        max: Option<u16>,
        parity: Option<u8>,
    ) {
        debug_assert!(min != 0);

        if builder.last_color == Some(color) {
            let index = builder.runs - 1;
            if index < 3 {
                let run = &mut builder.req.runs[index];
                run.min = run.min.saturating_add(min);
                run.max = match (run.max, max) {
                    (Some(left), Some(right)) => {
                        left.checked_add(right)
                    },
                    _ => None,
                };
                run.parity = match (run.parity, parity) {
                    (Some(left), Some(right)) => Some(left ^ right),
                    _ => None,
                };
            }
            return;
        }

        let index = builder.runs;
        if index < 3 {
            builder.req.runs[index] = ReqRun {
                color,
                min,
                max,
                parity,
            };
        } else if index == 3 {
            builder.req.fourth_color = Some(color);
        }

        if color != 0 {
            builder.req.suffix_nonblank |=
                (1_u8 << (index.min(4) + 1)) - 1;
        }

        let bit = side_prefix_color_bit(color);
        for mask in
            builder.req.suffix_colors.iter_mut().take(index.min(4) + 1)
        {
            *mask |= bit;
        }

        builder.runs += 1;
        builder.last_color = Some(color);
    }

    fn emit_run(
        builder: &mut Builder,
        color: Color,
        count: BlockCount,
    ) {
        let req = req_run(color, count);
        emit(builder, req.color, req.min, req.max, req.parity);
    }

    const fn settled(builder: &Builder) -> bool {
        builder.runs > 4 && builder.req.suffix_nonblank & (1 << 4) != 0
    }

    fn emit_word(
        builder: &mut Builder,
        word: &[Color],
        count: WordCount,
        skip_first: bool,
    ) {
        debug_assert_ne!(word, []);
        let copies = count.minimum();
        debug_assert!(copies != 0);

        if word.iter().all(|&color| color == word[0]) {
            let total = word
                .len()
                .checked_mul(copies)
                .and_then(|cells| {
                    cells.checked_sub(usize::from(skip_first))
                })
                .unwrap_or(usize::MAX);
            if total != 0 {
                let (min, max) = exact_bounds(total);
                emit(
                    builder,
                    word[0],
                    min,
                    max,
                    Some((total & 1) as u8),
                );
            }
            if count.is_indef() {
                builder.req.end_unknown = true;
                builder.blocked = true;
            }
            return;
        }

        let mut copy = 0_usize;
        while copy < copies {
            let start = usize::from(skip_first && copy == 0);
            for &color in &word[start..] {
                emit(builder, color, 1, Some(1), Some(1));
            }
            copy += 1;

            // For a non-homogeneous periodic word, only the first four
            // logical run colors and whether anything nonblank lies beyond
            // them can affect this matcher. Stop once both are settled.
            if settled(builder) {
                break;
            }
        }

        if count.is_indef() {
            // Extra copies are optional and add more logical runs, so
            // farther explicit blocks no longer occupy a fixed prefix
            // position. Keep the guaranteed prefix and conservatively
            // forget everything beyond it.
            builder.req.end_unknown = true;
            builder.blocked = true;
        }
    }

    fn emit_full_block(builder: &mut Builder, block: &Block) {
        if builder.blocked {
            return;
        }

        match block {
            Block::Run { color, count } => {
                emit_run(builder, *color, *count);
            },
            Block::Word { word, count } => {
                emit_word(builder, word, *count, false);
            },
        }
    }

    let end_unknown = span.end == TapeEnd::Unknown;
    let Some(first) = span.span.first() else {
        let req = Requirement {
            end_unknown,
            ..Requirement::EMPTY
        };
        return RequirementSet {
            reqs: [req, Requirement::EMPTY],
            len: 1,
            unconstrained: req.unconstrained(),
        };
    };

    let (first_mode, second_mode) = match first {
        Block::Run { count, .. } => match count {
            BlockCount::Exact(1) => (FirstResidual::Drop, None),
            BlockCount::Exact(count) => (
                FirstResidual::Replace(BlockCount::Exact(count - 1)),
                None,
            ),
            BlockCount::AtLeast(1) => (
                FirstResidual::Drop,
                Some(FirstResidual::Replace(BlockCount::AtLeast(1))),
            ),
            BlockCount::AtLeast(count) => (
                FirstResidual::Replace(BlockCount::AtLeast(count - 1)),
                None,
            ),
            BlockCount::Stride { min: 1, step } => (
                FirstResidual::Drop,
                Some(FirstResidual::Replace(BlockCount::Stride {
                    min: *step,
                    step: *step,
                })),
            ),
            BlockCount::Stride { min, step } => (
                FirstResidual::Replace(BlockCount::Stride {
                    min: min - 1,
                    step: *step,
                }),
                None,
            ),
        },
        Block::Word { .. } => (FirstResidual::ConsumeWord, None),
    };

    let len = if second_mode.is_some() { 2 } else { 1 };
    let modes = [first_mode, second_mode.unwrap_or(first_mode)];
    let mut builders = [
        Builder {
            req: Requirement {
                end_unknown,
                ..Requirement::EMPTY
            },
            runs: 0,
            last_color: None,
            blocked: false,
        },
        Builder {
            req: Requirement {
                end_unknown,
                ..Requirement::EMPTY
            },
            runs: 0,
            last_color: None,
            blocked: false,
        },
    ];

    let block_count = span.span.len();
    for (source_index, block) in span.span.iter().enumerate() {
        // A whole explicit blank block at a known-blank far end is
        // redundant, matching absorb_trailing_blanks/the old builder.
        if !end_unknown
            && source_index + 1 == block_count
            && block.blank()
        {
            continue;
        }

        for alt in 0..len {
            if source_index == 0 {
                match modes[alt] {
                    FirstResidual::Drop => {},
                    FirstResidual::Replace(count) => {
                        let (color, _) = block
                            .run()
                            .expect("Replace applies only to runs");
                        emit_run(&mut builders[alt], color, count);
                    },
                    FirstResidual::ConsumeWord => {
                        let Block::Word { word, count } = block else {
                            unreachable!()
                        };
                        emit_word(
                            &mut builders[alt],
                            word,
                            *count,
                            true,
                        );
                    },
                }
            } else {
                emit_full_block(&mut builders[alt], block);
            }
        }

        if (0..len).all(|alt| settled(&builders[alt])) {
            break;
        }
    }

    let mut reqs = [Requirement::EMPTY; 2];
    for alt in 0..len {
        let builder = &mut builders[alt];
        builder.req.run_count = builder.runs.min(4) as u8;

        if builder.req.end_unknown && (1..=3).contains(&builder.runs) {
            let last = &mut builder.req.runs[builder.runs - 1];
            last.max = None;
            last.parity = None;
        }
        reqs[alt] = builder.req;
    }

    RequirementSet {
        reqs,
        len: len as u8,
        unconstrained: (0..len).any(|alt| reqs[alt].unconstrained()),
    }
}

fn precise_count_matches(count: u8, req: ReqRun) -> bool {
    match count {
        1..=3 => {
            let value = u16::from(count);
            value >= req.min
                && req.max.is_none_or(|max| value <= max)
                && req.parity.is_none_or(|parity| parity == count & 1)
        },
        SIDE_PREFIX_EVEN_MANY | SIDE_PREFIX_ODD_MANY => {
            let (minimum, odd) = if count == SIDE_PREFIX_EVEN_MANY {
                (4_u16, false)
            } else {
                (5_u16, true)
            };
            if req.parity.is_some_and(|parity| (parity != 0) != odd) {
                return false;
            }
            let mut first =
                if req.min > minimum { req.min } else { minimum };
            if (first & 1 != 0) != odd {
                first = first.saturating_add(1);
            }
            req.max.is_none_or(|max| first <= max)
        },
        _ => unreachable!(),
    }
}

fn widened_spill_count_matches(req: ReqRun) -> bool {
    // Intersect the widened spill language [2, +inf) with the backward
    // requirement's interval/parity.
    let mut first = req.min.max(2);
    if let Some(parity) = req.parity
        && (first & 1) != u16::from(parity)
    {
        first = first.saturating_add(1);
    }
    req.max.is_none_or(|max| first <= max)
}

#[expect(clippy::comparison_chain)]
fn side_run_colors_compatible(
    prefix: SidePrefix,
    req: Requirement,
) -> bool {
    let prefix_len = usize::from(prefix.len);
    let req_len = req.run_count as usize;
    let common = prefix_len.min(req_len);

    // Check every retained color before doing any interval/parity arithmetic.
    // Later-run color mismatches are common and can reject a candidate without
    // paying for precise count intersection on the earlier runs.
    for index in 0..common {
        if prefix.runs[index].color != req.runs[index].color {
            return false;
        }
    }

    let mut known_len = prefix_len;
    let SidePrefixSpill::Run { color, far, .. } = prefix.spill else {
        return true;
    };

    if req_len > known_len {
        if color != req.runs[known_len].color {
            return false;
        }
        known_len += 1;
    } else {
        return true;
    }

    let SidePrefixFar::Run { color, .. } = far else {
        return true;
    };
    if req_len <= known_len {
        return true;
    }

    let required_color = if known_len < 3 {
        Some(req.runs[known_len].color)
    } else if known_len == 3 {
        req.fourth_color
    } else {
        None
    };
    required_color == Some(color)
}

fn side_run_matches_with_far_colors(
    prefix: SidePrefix,
    req: Requirement,
    far_colors: Option<u64>,
) -> bool {
    if !side_run_colors_compatible(prefix, req) {
        return false;
    }

    let prefix_len = usize::from(prefix.len);
    let req_len = req.run_count as usize;
    let common = prefix_len.min(req_len);

    for index in 0..common {
        if !precise_count_matches(
            prefix.runs[index].count,
            req.runs[index],
        ) {
            return false;
        }
    }

    if prefix_len > req_len {
        // The backward description can supply additional run
        // structure only when its explicit prefix ends in `?`.
        return req.end_unknown;
    }

    let mut known_len = prefix_len;
    let tail_dirty;

    match prefix.spill {
        SidePrefixSpill::Blank => {
            tail_dirty = false;
        },
        SidePrefixSpill::DirtyUnknown => {
            tail_dirty = true;
        },
        SidePrefixSpill::Run { count, far, .. } => {
            // The color prepass above has already checked this run. Only the
            // expensive count-language intersection remains here.
            if req_len <= known_len {
                return req.end_unknown;
            }
            let required = req.runs[known_len];
            let count_matches = if count == SIDE_PREFIX_SPILL_MANY {
                widened_spill_count_matches(required)
            } else {
                precise_count_matches(count, required)
            };
            if !count_matches {
                return false;
            }
            known_len += 1;

            match far {
                SidePrefixFar::Blank => {
                    tail_dirty = false;
                },
                SidePrefixFar::DirtyUnknown => {
                    tail_dirty = true;
                },
                SidePrefixFar::Run { farther_dirty, .. } => {
                    if req_len <= known_len {
                        return req.end_unknown;
                    }

                    // Fourth-run color compatibility was handled by the cheap
                    // prepass; the forward abstraction deliberately has no
                    // fourth-run count to check.
                    known_len += 1;
                    tail_dirty = farther_dirty;
                },
            }
        },
    }

    if known_len > req_len {
        return req.end_unknown;
    }

    if tail_dirty {
        if let Some(possible_colors) = far_colors {
            let required_colors = req.suffix_colors[known_len.min(4)];
            if required_colors & !possible_colors != 0 {
                return false;
            }
        }

        req.end_unknown
            || req.suffix_nonblank & (1_u8 << known_len) != 0
    } else {
        req.suffix_nonblank & (1_u8 << known_len) == 0
    }
}

fn side_run_matches(prefix: SidePrefix, req: Requirement) -> bool {
    side_run_matches_with_far_colors(prefix, req, None)
}

/// Same exact-window conditioning as `SidePossible`, but keeps alternatives
/// instead of unioning their ordered near-tail run structure.
const SIDE_PREFIX_HAS_BLANK: u8 = 1;
const SIDE_PREFIX_HAS_DIRTY_UNKNOWN: u8 = 2;
const SIDE_PREFIX_UNCONSTRAINED: u8 =
    SIDE_PREFIX_HAS_BLANK | SIDE_PREFIX_HAS_DIRTY_UNKNOWN;

struct SidePrefixPossible<const S: usize, const C: usize> {
    // Flattened [window][side] -> reachable run-prefix alternatives.
    windows: Vec<Vec<SidePrefix>>,

    // Parallel ordered cell/word-prefix alternatives. This is a second sound
    // projection of the same forward executions, so backward configurations
    // must be compatible with both domains.
    word_windows: Vec<Vec<SideWordPrefix>>,

    // Cached membership of the broad alternatives for the original run
    // abstraction.
    flags: Vec<u8>,

    // Broad-state flags for the word abstraction. Bit 0 = exact blank,
    // bit 1 = dirty unknown, bit 2 = fully unknown/unconstrained.
    word_flags: Vec<u8>,
}

impl<const S: usize, const C: usize> SidePrefixPossible<S, C> {
    const fn window_index(
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
    ) -> usize {
        (((st * C) + scan) * C + left) * C + right
    }

    const fn index(
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
        side: usize,
    ) -> usize {
        Self::window_index(st, scan, left, right) * 2 + side
    }

    fn new_run_only() -> Self {
        let len = S * C * C * C * 2;
        Self {
            windows: (0..len).map(|_| Vec::new()).collect(),
            word_windows: Vec::new(),
            flags: vec![0; len],
            word_flags: Vec::new(),
        }
    }

    fn init_word_storage(&mut self) {
        debug_assert_eq!(self.word_windows.len(), 0);
        debug_assert_eq!(self.word_flags.len(), 0);
        let len = self.windows.len();
        self.word_windows = (0..len).map(|_| Vec::new()).collect();
        self.word_flags = vec![0; len];
    }

    fn prefixes(
        &self,
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
        side: usize,
    ) -> &[SidePrefix] {
        &self.windows[Self::index(st, scan, left, right, side)]
    }

    fn word_prefixes(
        &self,
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
        side: usize,
    ) -> &[SideWordPrefix] {
        &self.word_windows[Self::index(st, scan, left, right, side)]
    }

    fn word_side_unconstrained(
        &self,
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
        side: usize,
    ) -> bool {
        let flags =
            self.word_flags[Self::index(st, scan, left, right, side)];
        flags & 0b100 != 0 || flags & 0b011 == 0b011
    }

    fn word_has_unknown_index(&self, index: usize) -> bool {
        self.word_flags[index] & 0b100 != 0
    }

    fn side_unconstrained(
        &self,
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
        side: usize,
    ) -> bool {
        self.flags[Self::index(st, scan, left, right, side)]
            == SIDE_PREFIX_UNCONSTRAINED
    }

    fn has_dirty_unknown_index(&self, index: usize) -> bool {
        self.flags[index] & SIDE_PREFIX_HAS_DIRTY_UNKNOWN != 0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct SidePrefixNode {
    side: usize,
    st: usize,
    scan: usize,
    left: usize,
    right: usize,
    prefix: SidePrefix,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct SideWordPrefixNode {
    side: usize,
    st: usize,
    scan: usize,
    left: usize,
    right: usize,
    prefix: SideWordPrefix,
}

const JOINT_SHORT_DEPTH: usize = 3;

/// Tiny left/right reduced product retaining the first two cells strictly
/// beyond each immediate neighbor in the *same* forward execution.  Each side
/// also remembers whether everything after those cells is certainly blank.
/// Unknown forgotten tails are deliberately widened: the power comes from
/// correlating the two near prefixes, not from another long-horizon domain.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct JointShortSide {
    cells: [Color; JOINT_SHORT_DEPTH],
    len: u8,
    end_blank: bool,
}

impl JointShortSide {
    const fn blank() -> Self {
        Self {
            cells: [0; JOINT_SHORT_DEPTH],
            len: 0,
            end_blank: true,
        }
    }

    fn prepend(self, color: Color) -> Self {
        if self.len == 0 && self.end_blank && color == 0 {
            return self;
        }

        let mut out = self;
        let len = usize::from(out.len);
        if len < JOINT_SHORT_DEPTH {
            let mut index = len;
            while index != 0 {
                out.cells[index] = out.cells[index - 1];
                index -= 1;
            }
            out.cells[0] = color;
            out.len += 1;
            return out;
        }

        let dropped = out.cells[JOINT_SHORT_DEPTH - 1];
        let mut index = JOINT_SHORT_DEPTH - 1;
        while index != 0 {
            out.cells[index] = out.cells[index - 1];
            index -= 1;
        }
        out.cells[0] = color;
        out.end_blank &= dropped == 0;
        out
    }

    fn for_each_pull<const C: usize>(
        self,
        mut emit: impl FnMut(Color, Self),
    ) {
        let len = usize::from(self.len);
        if len != 0 {
            let color = self.cells[0];
            let mut out = self;
            let mut index = 1;
            while index < len {
                out.cells[index - 1] = out.cells[index];
                index += 1;
            }
            out.cells[len - 1] = 0;
            out.len -= 1;
            emit(color, out);
            return;
        }

        if self.end_blank {
            emit(0, self);
            return;
        }

        for color in 0..C {
            #[expect(clippy::cast_possible_truncation)]
            emit(color as Color, self);
        }
    }

    fn cell(self, index: usize) -> Option<Color> {
        if index < usize::from(self.len) {
            Some(self.cells[index])
        } else if self.end_blank {
            Some(0)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct JointShortPrefix {
    left: JointShortSide,
    right: JointShortSide,
}

impl JointShortPrefix {
    const fn blank() -> Self {
        Self {
            left: JointShortSide::blank(),
            right: JointShortSide::blank(),
        }
    }
}

struct JointShortPossible<const S: usize, const C: usize> {
    windows: Vec<Vec<JointShortPrefix>>,
}

impl<const S: usize, const C: usize> JointShortPossible<S, C> {
    const fn index(
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
    ) -> usize {
        (((st * C) + scan) * C + left) * C + right
    }

    fn new() -> Self {
        Self {
            windows: (0..S * C * C * C).map(|_| Vec::new()).collect(),
        }
    }

    fn window(
        &self,
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
    ) -> &[JointShortPrefix] {
        &self.windows[Self::index(st, scan, left, right)]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct JointShortNode {
    st: usize,
    scan: usize,
    left: usize,
    right: usize,
    prefix: JointShortPrefix,
}

/// Exact radius-2 same-cell crossing relation.  For one ordinary local
/// window `(left, scan, right)`, bit `l2 * C + r2` records a same-witness
/// checkpoint `l2 left [scan] right r2` reachable from the blank tape.
///
/// The domain is enabled only for C <= 8, where all C^2 second-neighbor
/// pairs fit in one u64.  Larger alphabets conservatively disable the extra
/// refinement and retain the existing radius-1 crossing relation.
struct Radius2Possible<const S: usize, const C: usize> {
    enabled: bool,
    pairs: Vec<u64>,
}

impl<const S: usize, const C: usize> Radius2Possible<S, C> {
    const fn index(
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
    ) -> usize {
        (((st * C + scan) * C + left) * C) + right
    }

    const fn disabled() -> Self {
        Self {
            enabled: false,
            pairs: Vec::new(),
        }
    }

    fn from_pairs(pairs: Vec<u64>) -> Self {
        debug_assert!(C <= 8);
        debug_assert_eq!(pairs.len(), S * C * C * C);
        Self {
            enabled: true,
            pairs,
        }
    }

    fn pair_mask(
        &self,
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
    ) -> u64 {
        if !self.enabled {
            return u64::MAX;
        }
        self.pairs[Self::index(st, scan, left, right)]
    }

    #[expect(clippy::disallowed_names)]
    const fn required_pair_mask(
        left2: Option<usize>,
        right2: Option<usize>,
    ) -> u64 {
        if C > 8 {
            return u64::MAX;
        }

        match (left2, right2) {
            (Some(left2), Some(right2)) => {
                1_u64 << (left2 * C + right2)
            },
            (Some(left2), None) => {
                let row = (1_u64 << C) - 1;
                row << (left2 * C)
            },
            (None, Some(right2)) => {
                let mut mask = 0_u64;
                let mut left2 = 0_usize;
                while left2 < C {
                    mask |= 1_u64 << (left2 * C + right2);
                    left2 += 1;
                }
                mask
            },
            (None, None) => u64::MAX,
        }
    }

    fn compatible(
        &self,
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
        left2: Option<usize>,
        right2: Option<usize>,
    ) -> bool {
        if !self.enabled {
            return true;
        }

        self.pair_mask(st, scan, left, right)
            & Self::required_pair_mask(left2, right2)
            != 0
    }

    fn joint_short_compatible(
        &self,
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
        prefix: JointShortPrefix,
    ) -> bool {
        self.compatible(
            st,
            scan,
            left,
            right,
            prefix.left.cell(0).map(usize::from),
            prefix.right.cell(0).map(usize::from),
        )
    }

    fn radius1_projection(&self) -> [[[u64; C]; C]; S] {
        let mut possible = [[[0_u64; C]; C]; S];
        if !self.enabled {
            return possible;
        }

        for st in 0..S {
            for scan in 0..C {
                for left in 0..C {
                    for right in 0..C {
                        if self.pair_mask(st, scan, left, right) != 0 {
                            possible[st][scan][left] |= 1_u64 << right;
                        }
                    }
                }
            }
        }
        possible
    }
}

// Joint version of the run-prefix domain.  Unlike `SidePrefixPossible`, the
// left and right prefixes below always come from the same forward execution.
// This preserves correlations such as a remote left anchor being required
// while the right side has a particular shape.
const JOINT_SIDE_PREFIX_MAX_ALTS_PER_WINDOW: usize = 64;
// DirtyUnknown can emit five count classes, each with blank/dirty residuals.
const SIDE_PREFIX_MAX_PREPEND_ALTS: usize = 10;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[expect(variant_size_differences)]
enum JointSidePrefix {
    Specific {
        left: SidePrefix,
        right: SidePrefix,
        // Bit `lp | (rp << 1)` records one possible pair of whole-side
        // nonblank parities for this same run-shape alternative.
        parity_mask: u8,
        // Per-side union of colors occurring in the forgotten remainder
        // beyond the retained ordered run horizon. These masks are metadata:
        // they do not participate in shape identity and merge by OR.
        far_colors: [u64; 2],
    },
    // Conservative shape top used after a per-window antichain overflow or
    // propagated from one. Unlike the old top, it still retains the possible
    // left/right whole-side parity pairs. Far-color information is discarded
    // because an unknown shape can move the retained/far boundary arbitrarily.
    Unknown {
        parity_mask: u8,
    },
}

impl JointSidePrefix {
    const fn blank(track_parity: bool) -> Self {
        Self::Specific {
            left: SidePrefix::blank(),
            right: SidePrefix::blank(),
            parity_mask: if track_parity { 1 } else { 0b1111 },
            // The infinite forgotten remainder is blank on both sides.
            far_colors: [1; 2],
        }
    }

    const fn parity_mask(self) -> u8 {
        match self {
            Self::Specific { parity_mask, .. }
            | Self::Unknown { parity_mask } => parity_mask,
        }
    }

    fn merge_same_shape(self, other: Self) -> Self {
        debug_assert!(self.same_shape(other));
        let parity_mask = self.parity_mask() | other.parity_mask();
        match (self, other) {
            (
                Self::Specific {
                    left,
                    right,
                    far_colors: a,
                    ..
                },
                Self::Specific { far_colors: b, .. },
            ) => Self::Specific {
                left,
                right,
                parity_mask,
                far_colors: [a[0] | b[0], a[1] | b[1]],
            },
            (Self::Unknown { .. }, Self::Unknown { .. }) => {
                Self::Unknown { parity_mask }
            },
            _ => unreachable!(),
        }
    }

    fn widen_far_colors(self) -> Self {
        match self {
            Self::Specific {
                left,
                right,
                parity_mask,
                mut far_colors,
            } => {
                // The first pressure fallback forgets the exact fourth-run
                // color. Move that color into the merged forgotten-tail mask
                // before replacing its ordered descriptor with DirtyUnknown.
                if let Some(color) = left.fourth_run_color() {
                    far_colors[LEFT_SIDE] |=
                        side_prefix_color_bit(color);
                }
                if let Some(color) = right.fourth_run_color() {
                    far_colors[RIGHT_SIDE] |=
                        side_prefix_color_bit(color);
                }
                Self::Specific {
                    left: left.widen_far_color(),
                    right: right.widen_far_color(),
                    parity_mask,
                    far_colors,
                }
            },
            Self::Unknown { .. } => self,
        }
    }

    const fn widen_spill_counts(self) -> Self {
        match self {
            Self::Specific {
                left,
                right,
                parity_mask,
                far_colors,
            } => Self::Specific {
                left: left.widen_spill_count(),
                right: right.widen_spill_count(),
                parity_mask,
                far_colors,
            },
            Self::Unknown { .. } => self,
        }
    }

    fn same_shape(self, other: Self) -> bool {
        match (self, other) {
            (Self::Unknown { .. }, Self::Unknown { .. }) => true,
            (
                Self::Specific {
                    left: a_l,
                    right: a_r,
                    ..
                },
                Self::Specific {
                    left: b_l,
                    right: b_r,
                    ..
                },
            ) => a_l == b_l && a_r == b_r,
            _ => false,
        }
    }

    fn subsumes(self, other: Self) -> bool {
        if self.parity_mask() & other.parity_mask()
            != other.parity_mask()
        {
            return false;
        }

        match (self, other) {
            (Self::Unknown { .. }, _) => true,
            (_, Self::Unknown { .. }) => false,
            (
                Self::Specific {
                    left: a_l,
                    right: a_r,
                    far_colors: a_far,
                    ..
                },
                Self::Specific {
                    left: b_l,
                    right: b_r,
                    far_colors: b_far,
                    ..
                },
            ) => {
                a_far[LEFT_SIDE] | b_far[LEFT_SIDE] == a_far[LEFT_SIDE]
                    && a_far[RIGHT_SIDE] | b_far[RIGHT_SIDE]
                        == a_far[RIGHT_SIDE]
                    && a_l.subsumes(b_l)
                    && a_r.subsumes(b_r)
            },
        }
    }
}

/// Insert one alternative into a joint run-prefix antichain. Same-shape
/// parity/far-color merging, existing-subsumes-new, and
/// new-subsumes-existing are handled in one mutable walk. If metadata merging
/// enlarges the incoming alternative, restart so it can remove entries visited
/// before the merge.
fn insert_joint_side_prefix_alt(
    alts: &mut Vec<JointSidePrefix>,
    mut prefix: JointSidePrefix,
) -> Option<JointSidePrefix> {
    let mut index = 0_usize;
    while index < alts.len() {
        let old = alts[index];
        if old.subsumes(prefix) {
            return None;
        }

        if old.same_shape(prefix) {
            prefix = old.merge_same_shape(prefix);
            alts.swap_remove(index);
            index = 0;
            continue;
        }

        if prefix.subsumes(old) {
            alts.swap_remove(index);
        } else {
            index += 1;
        }
    }

    alts.push(prefix);
    Some(prefix)
}

/// Apply a pressure widening and rebuild the antichain in its existing Vec.
/// The front of the same allocation is used as the compacted output, avoiding
/// a fresh allocation/copy every time a hot joint window crosses a pressure
/// threshold.
fn widen_joint_side_prefix_alts_in_place(
    alts: &mut Vec<JointSidePrefix>,
    widen: fn(JointSidePrefix) -> JointSidePrefix,
) {
    for prefix in alts.iter_mut() {
        *prefix = widen(*prefix);
    }

    let source_len = alts.len();
    let mut out_len = 0_usize;
    let mut read = 0_usize;
    while read < source_len {
        let mut prefix = alts[read];
        let mut index = 0_usize;
        let mut keep = true;

        while index < out_len {
            let old = alts[index];
            if old.subsumes(prefix) {
                keep = false;
                break;
            }

            if old.same_shape(prefix) {
                prefix = old.merge_same_shape(prefix);
                out_len -= 1;
                alts[index] = alts[out_len];
                index = 0;
                continue;
            }

            if prefix.subsumes(old) {
                out_len -= 1;
                alts[index] = alts[out_len];
            } else {
                index += 1;
            }
        }

        if keep {
            alts[out_len] = prefix;
            out_len += 1;
        }
        read += 1;
    }
    alts.truncate(out_len);
}

/// Advance a set of whole-side nonblank parity pairs through one exact local
/// transition.  Each concrete pair is transformed only by XOR with one fixed
/// two-bit delta, so permute the four mask bits directly instead of walking
/// the set bits on every forward edge.
fn advance_joint_side_parity(
    mask: u8,
    shift: Shift,
    print: usize,
    left: usize,
    right: usize,
) -> u8 {
    let delta = if shift {
        u8::from(print != 0) | (u8::from(right != 0) << 1)
    } else {
        u8::from(left != 0) | (u8::from(print != 0) << 1)
    };

    match delta {
        0 => mask,
        1 => ((mask & 0b0101) << 1) | ((mask & 0b1010) >> 1),
        2 => ((mask & 0b0011) << 2) | ((mask & 0b1100) >> 2),
        3 => {
            ((mask & 0b0001) << 3)
                | ((mask & 0b0010) << 1)
                | ((mask & 0b0100) >> 1)
                | ((mask & 0b1000) >> 3)
        },
        _ => unreachable!(),
    }
}

struct JointSidePrefixPossible<const S: usize, const C: usize> {
    windows: Vec<Vec<JointSidePrefix>>,
    // Once a bucket itself crosses the antichain cap, keep it permanently as
    // one shape-unknown alternative.  Its parity mask can still grow later.
    overflowed: Vec<bool>,
    // First pressure fallback: forget only the fourth-run color, recovering
    // the previous exact blank-vs-dirty far-tail abstraction.
    far_color_widened: Vec<bool>,
    // Second fallback: widen the third-run count back to the old 1/2+ lattice.
    spill_widened: Vec<bool>,
    // Blank-target frontiers start with two finite known-blank ends.  Their
    // run requirements already expose the parity facts that matter in this
    // domain, so avoid paying for same-witness parity propagation there.
    track_parity: bool,
}

impl<const S: usize, const C: usize> JointSidePrefixPossible<S, C> {
    const fn index(
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
    ) -> usize {
        (((st * C) + scan) * C + left) * C + right
    }

    fn new(track_parity: bool) -> Self {
        let len = S * C * C * C;
        Self {
            windows: (0..len).map(|_| Vec::new()).collect(),
            overflowed: vec![false; len],
            far_color_widened: vec![false; len],
            spill_widened: vec![false; len],
            track_parity,
        }
    }

    fn window(
        &self,
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
    ) -> &[JointSidePrefix] {
        &self.windows[Self::index(st, scan, left, right)]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct JointSidePrefixNode {
    st: usize,
    scan: usize,
    left: usize,
    right: usize,
    prefix: JointSidePrefix,
}

// Joint version of the richer ordered cell/word-prefix domain. Unlike the
// independent `SidePrefixPossible::word_windows`, each left/right pair below
// comes from one forward execution. This prevents the backward prover from
// combining a left periodic/literal prefix from one run with an incompatible
// right prefix from another run.
const JOINT_SIDE_WORD_MAX_ALTS_PER_WINDOW: usize = 64;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum JointSideWordPrefix {
    Specific {
        left: SideWordPrefix,
        right: SideWordPrefix,
    },
    // Conservative top used only when the paired antichain overflows.
    Unknown,
}

impl JointSideWordPrefix {
    const fn blank() -> Self {
        Self::Specific {
            left: SideWordPrefix::blank(),
            right: SideWordPrefix::blank(),
        }
    }

    fn subsumes(self, other: Self) -> bool {
        match (self, other) {
            (Self::Unknown, _) => true,
            (_, Self::Unknown) => false,
            (
                Self::Specific {
                    left: a_l,
                    right: a_r,
                },
                Self::Specific {
                    left: b_l,
                    right: b_r,
                },
            ) => a_l.subsumes(b_l) && a_r.subsumes(b_r),
        }
    }
}

struct JointSideWordPrefixPossible<const S: usize, const C: usize> {
    windows: Vec<Vec<JointSideWordPrefix>>,
}

impl<const S: usize, const C: usize> JointSideWordPrefixPossible<S, C> {
    const fn index(
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
    ) -> usize {
        (((st * C) + scan) * C + left) * C + right
    }

    fn new() -> Self {
        Self {
            windows: (0..S * C * C * C).map(|_| Vec::new()).collect(),
        }
    }

    fn window(
        &self,
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
    ) -> &[JointSideWordPrefix] {
        &self.windows[Self::index(st, scan, left, right)]
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct JointSideWordPrefixNode {
    st: usize,
    scan: usize,
    left: usize,
    right: usize,
    prefix: JointSideWordPrefix,
}

/// Bit `p` of `possible[state]` is set when the transition graph admits a
/// run from the blank initial configuration to `state` with
/// `p == (# nonblank tape cells mod 2)`.
///
/// This deliberately forgets the tape contents and therefore computes an
/// over-approximation. A missing bit is nevertheless a sound invariant and
/// can be used to prune backward configurations whose complete finite tape has
/// the wrong parity.
struct NonblankParity<const S: usize> {
    possible: [u8; S],
}

/// Joint whole-side status carried by the forward abstraction.
///
/// Bit 0 of a concrete `flags` value means the whole left side is blank and
/// bit 1 means the whole right side is blank. A clear bit means that side is
/// definitely dirty (contains at least one nonblank), not merely unknown.
const LEFT_BLANK_FLAG: u8 = 1;
const RIGHT_BLANK_FLAG: u8 = 2;
const BOTH_BLANK_FLAGS: u8 = LEFT_BLANK_FLAG | RIGHT_BLANK_FLAG;

/// Same-run blank/dirty possibilities, both aggregated by `(state, scan)` and
/// conditioned on an exact reachable local window `(left, scan, right)`.
///
/// Each stored byte is a set of the four concrete side-status combinations:
/// bit `1 << flags` is set when that exact status pair is possible.
struct JointBlankPossible<const S: usize, const C: usize> {
    any: [[u8; C]; S],
    windows: Vec<u8>,
}

impl<const S: usize, const C: usize> JointBlankPossible<S, C> {
    const fn index(
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
    ) -> usize {
        (((st * C) + scan) * C + left) * C + right
    }

    fn new() -> Self {
        Self {
            any: [[0; C]; S],
            windows: vec![0; S * C * C * C],
        }
    }

    fn window_mask(
        &self,
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
    ) -> u8 {
        self.windows[Self::index(st, scan, left, right)]
    }
}

/// Forward over-approximations used whenever a backward configuration proves
/// something about a whole side.
///
/// The excursion-derived halfblank tables remain the strongest checks for an
/// exactly blank single side. `joint` additionally retains all four same-run
/// blank/dirty combinations and correlates them with the exact local window.
struct BlankSidePossible<const S: usize, const C: usize> {
    // For a left-blank checkpoint `0+ [scan] near`, bit `near` is set in
    // `left_half[state][scan]`.  `right_half` is symmetric for
    // `near [scan] 0+`.  Retaining the exact inward-neighbor color keeps the
    // clean-excursion proof correlated with the local window instead of
    // collapsing it to a state/scan boolean.
    left_half: [[u64; C]; S],
    right_half: [[u64; C]; S],
    joint: JointBlankPossible<S, C>,
}

/// Independent per-color capped tail-count possibilities, conditioned on the
/// exact local window. For each nonblank color `k`, the left and right tails
/// (strictly beyond the immediate neighbors) are each summarized as:
///
/// - 0: no `k`,
/// - 1: exactly one `k`,
/// - 2: at least two `k`.
///
/// A concrete status is `left_count + 3 * right_count`, so each stored `u16`
/// is a set of the nine possible `(left_count, right_count)` combinations.
/// This strictly refines the old single-color presence abstraction while
/// retaining the same exact-window conditioning.
struct ColorTailCountPossible<const S: usize, const C: usize> {
    exact: Vec<u16>,
    by_left: Vec<u16>,
    by_right: Vec<u16>,
    any: Vec<u16>,
}

impl<const S: usize, const C: usize> ColorTailCountPossible<S, C> {
    const fn exact_index(
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
        color: usize,
    ) -> usize {
        ((((st * C) + scan) * C + left) * C + right) * C + color
    }

    const fn side_index(
        st: usize,
        scan: usize,
        neighbor: usize,
        color: usize,
    ) -> usize {
        (((st * C) + scan) * C + neighbor) * C + color
    }

    const fn any_index(st: usize, scan: usize, color: usize) -> usize {
        ((st * C) + scan) * C + color
    }

    fn new() -> Self {
        Self {
            exact: vec![0; S * C * C * C * C],
            by_left: vec![0; S * C * C * C],
            by_right: vec![0; S * C * C * C],
            any: vec![0; S * C * C],
        }
    }

    fn add(
        &mut self,
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
        color: usize,
        status: u8,
    ) {
        debug_assert!(status < 9);
        let bit = 1_u16 << status;
        self.exact[Self::exact_index(st, scan, left, right, color)] |=
            bit;
        self.by_left[Self::side_index(st, scan, left, color)] |= bit;
        self.by_right[Self::side_index(st, scan, right, color)] |= bit;
        self.any[Self::any_index(st, scan, color)] |= bit;
    }

    fn mask(
        &self,
        st: usize,
        scan: usize,
        left: Option<usize>,
        right: Option<usize>,
        color: usize,
    ) -> u16 {
        match (left, right) {
            (Some(left), Some(right)) => {
                self.exact
                    [Self::exact_index(st, scan, left, right, color)]
            },
            (Some(left), None) => {
                self.by_left[Self::side_index(st, scan, left, color)]
            },
            (None, Some(right)) => {
                self.by_right[Self::side_index(st, scan, right, color)]
            },
            (None, None) => self.any[Self::any_index(st, scan, color)],
        }
    }
}

/// Same-run pairwise tail-presence possibilities, conditioned on the exact
/// local window. For each unordered nonblank color pair `(a, b)`, a concrete
/// status has four bits:
///
/// - bit 0: `a` occurs in the left tail,
/// - bit 1: `a` occurs in the right tail,
/// - bit 2: `b` occurs in the left tail,
/// - bit 3: `b` occurs in the right tail.
///
/// Each stored `u16` is a set of those 16 concrete statuses. This retains the
/// correlation between two colors that the independent per-color abstraction
/// deliberately joins away.
struct PairTailPresencePossible<const S: usize, const C: usize> {
    exact: Vec<u16>,
    by_left: Vec<u16>,
    by_right: Vec<u16>,
    any: Vec<u16>,
}

impl<const S: usize, const C: usize> PairTailPresencePossible<S, C> {
    const fn pair_count() -> usize {
        C.saturating_sub(1) * C.saturating_sub(2) / 2
    }

    /// Dense index for `1 <= a < b < C`, in lexicographic pair order.
    const fn pair_index(a: usize, b: usize) -> usize {
        debug_assert!(0 < a && a < b && b < C);
        let before = (a - 1) * (2 * C - a - 2) / 2;
        before + (b - a - 1)
    }

    const fn exact_index(
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
        pair: usize,
    ) -> usize {
        let window = (((st * C) + scan) * C + left) * C + right;
        window * Self::pair_count() + pair
    }

    const fn side_index(
        st: usize,
        scan: usize,
        neighbor: usize,
        pair: usize,
    ) -> usize {
        let side = ((st * C) + scan) * C + neighbor;
        side * Self::pair_count() + pair
    }

    const fn any_index(st: usize, scan: usize, pair: usize) -> usize {
        ((st * C) + scan) * Self::pair_count() + pair
    }

    fn new() -> Self {
        let pairs = Self::pair_count();
        Self {
            exact: vec![0; S * C * C * C * pairs],
            by_left: vec![0; S * C * C * pairs],
            by_right: vec![0; S * C * C * pairs],
            any: vec![0; S * C * pairs],
        }
    }

    fn add(
        &mut self,
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
        a: usize,
        b: usize,
        status: u8,
    ) {
        let pair = Self::pair_index(a, b);
        let bit = 1_u16 << status;
        self.exact[Self::exact_index(st, scan, left, right, pair)] |=
            bit;
        self.by_left[Self::side_index(st, scan, left, pair)] |= bit;
        self.by_right[Self::side_index(st, scan, right, pair)] |= bit;
        self.any[Self::any_index(st, scan, pair)] |= bit;
    }

    fn mask(
        &self,
        st: usize,
        scan: usize,
        left: Option<usize>,
        right: Option<usize>,
        a: usize,
        b: usize,
    ) -> u16 {
        let pair = Self::pair_index(a, b);
        match (left, right) {
            (Some(left), Some(right)) => {
                self.exact
                    [Self::exact_index(st, scan, left, right, pair)]
            },
            (Some(left), None) => {
                self.by_left[Self::side_index(st, scan, left, pair)]
            },
            (None, Some(right)) => {
                self.by_right[Self::side_index(st, scan, right, pair)]
            },
            (None, None) => self.any[Self::any_index(st, scan, pair)],
        }
    }
}

fn cant_reach<const s: usize, const c: usize, T: Ord, F>(
    prog: &Prog<s, c>,
    steps: Steps,
    mut slots: Set<(State, T)>,
    entrypoints: Option<Entrypoints>,
    get_configs: F,
    use_exact_seen: bool,
) -> BackwardResult
where
    F: Fn(&Set<(State, T)>) -> Configs,
{
    if slots.is_empty() {
        return Refuted(0);
    }

    let entrypoints =
        entrypoints.unwrap_or_else(|| prog.get_entrypoints());

    slots.retain(|(state, _)| entrypoints.contains_key(state));

    if slots.is_empty() {
        return Refuted(0);
    }

    // The common path is still one ordinary BKW pass: no cycle-edge history
    // is built unless a real u8 count overflow is encountered.
    let first = cant_reach_once::<s, c, T, F, false>(
        prog,
        steps,
        &slots,
        &entrypoints,
        &get_configs,
        use_exact_seen,
    );

    match first {
        CountLimit => {},
        other => return other,
    }

    // Certificate-only fallback. Count overflow is only the trigger for the
    // expensive cycle pass. The retry keeps the original single-color run
    // counts exact and records recurring predecessor edges. Repeated-word
    // widening is the separate sound abstraction used in both passes. When one particular edge would
    // overflow after a stable increasing recurrence, only that edge is cut;
    // sibling exits and unrelated frontier branches remain live. Thus cycle
    // handling cannot invent a new predecessor entrance.
    //
    // This pass has its own budget.  Cutting one overflowing lineage can leave
    // finite sibling/exit cones that need more predecessor layers than the
    // caller's cheap-pass budget.  Restoring the larger overflow-only budget
    // cannot affect programs that did not first hit CountLimit.
    let cycle_steps = steps.max(4_096);

    match cant_reach_once::<s, c, T, F, true>(
        prog,
        cycle_steps,
        &slots,
        &entrypoints,
        &get_configs,
        use_exact_seen,
    ) {
        Refuted(step) => Refuted(step),
        _ => CountLimit,
    }
}

fn cant_reach_once<
    const s: usize,
    const c: usize,
    T: Ord,
    F,
    const CYCLE_ANALYSIS: bool,
>(
    prog: &Prog<s, c>,
    steps: Steps,
    slots: &Set<(State, T)>,
    entrypoints: &Entrypoints,
    get_configs: &F,
    use_exact_seen: bool,
) -> BackwardResult
where
    F: Fn(&Set<(State, T)>) -> Configs,
{
    // Shift-side analysis:
    // For some colors, the transition table itself proves they can
    // never appear on one side of the head in any run from the blank
    // tape. (Example: if a color is never written on an L-move, it
    // cannot persist to the right of the head.) We use this as a
    // *sound* pruning filter to avoid spurious backward
    // configurations.
    let (forbid_left, forbid_right) = prog.shift_side_forbidden();

    // If shift-side analysis proves that *no non-blank* symbol can ever
    // appear on a given side of the head in any run from the blank
    // tape, then that entire side is forced to be blank.
    //
    // This remains sound even if the program *can* print blank: the
    // invariant is about which symbols can occur on each side, not
    // about whether a cell has been visited.
    let left_forced_blank = (1..c).all(|k| forbid_left[k]);
    let right_forced_blank = (1..c).all(|k| forbid_right[k]);

    // One-sided blank-write analysis (strictly stronger than the global
    // "never writes blank" special case).
    //
    // For a cell to contain blank (0) *within the visited region* on the
    // left of the head, the last time that cell was visited the head must
    // have moved Right after writing 0 there. Therefore, if the program
    // never writes 0 on a Right move, any 0 appearing to the *left* of the
    // head must be unvisited, and thus all cells farther left must also be
    // unvisited blanks. Symmetrically for the right side and Left moves.
    let (writes_blank_on_r, writes_blank_on_l) =
        prog.blank_writes_by_shift();
    let left_fresh_zero = !writes_blank_on_r;
    let right_fresh_zero = !writes_blank_on_l;

    // Sound state/nonblank-count parity invariant. This is especially useful
    // after the fresh-zero rules turn a formerly unknown tape end into `0+`,
    // making the total nonblank parity exact.
    let nonblank_parity = prog.nonblank_parity_from_blank();

    let mut configs = get_configs(slots);

    // Apply the cheap static side filters before constructing the window
    // fixed point. This preserves the old short-circuit order while avoiding
    // window construction when every target is already impossible.
    configs.retain_mut(|Config { tape, .. }| {
        tape.obeys_shift_side(&forbid_left, &forbid_right)
            && tape.tighten_forced_blank_ends(
                left_forced_blank,
                right_forced_blank,
            )
            && tape.enforce_fresh_zero_side_invariants(
                left_fresh_zero,
                right_fresh_zero,
            )
    });

    configs.retain(|Config { state, tape }| {
        nonblank_parity_possible(*state, tape, &nonblank_parity)
    });

    if configs.is_empty() {
        return Refuted(0);
    }

    // Optional *sound* adjacency reachability filter.
    //
    // We over-approximate the set of 3-cell windows (L,scan,R) that
    // can appear around the head in each state when starting from the
    // blank tape. If a generated predecessor configuration demands an
    // immediate neighbor color that is impossible in this
    // over-approximation, we can safely prune it.
    let mut win_possible =
        prog.win_possible_from_blank(&forbid_left, &forbid_right);

    // Reduce the exact local-window relation jointly with whole-side
    // reachability.  Each time `SidePossible` removes a window, the exact-front
    // excursion grammar may lose recursive continuations and remove more; each
    // crossing shrink can in turn sharpen `SidePossible`.  Keep the cheap
    // window-only tables during these rounds, then rebuild all parity/residue
    // aggregates once at the shared fixed point.
    let (
        mut side_possible,
        crossing_left_any,
        crossing_right_any,
        mut radius2_possible,
    ) = refine_windows_by_crossings_and_sides(prog, &mut win_possible);
    win_possible.refine_reachability(&side_possible);

    // Preserve the original lazy ordering: build/check every cheaper forward
    // summary before paying for ordered-prefix propagation.  Easy targets that
    // these summaries already refute never enter the expensive prefix/window
    // feedback fixed point.
    let mut color_tail_count =
        color_tail_count_from_blank(prog, &win_possible);
    side_possible.refine_zero_tail_pairs(&color_tail_count);

    let mut blank_side_possible =
        blank_side_possible_from_blank_with_any(
            prog,
            &win_possible,
            &crossing_left_any,
            &crossing_right_any,
        );
    let mut pair_tail_presence =
        pair_tail_presence_from_blank(prog, &win_possible);

    // Halt targets begin with two unknown neighbors, so the
    // `(state, scanned color)` pair must still occur in at least one reachable
    // window after the cheaper side filters above have canonicalized the tape.
    configs.retain(|Config { state, tape }| {
        window_nonblank_parity_possible(*state, tape, &win_possible)
            && window_side_nonblank_parity_possible(
                *state,
                tape,
                &win_possible,
            )
            && window_side_nonblank_mod3_possible(
                *state,
                tape,
                &win_possible,
            )
            && window_color_parity_possible(*state, tape, &win_possible)
            && window_possible(*state, tape, &win_possible)
            && window_radius2_possible(*state, tape, &radius2_possible)
            && tape.obeys_state_side(*state, &side_possible)
            && tape
                .obeys_blank_side_possible(*state, &blank_side_possible)
            && tape.obeys_tail_presence(
                *state,
                &color_tail_count,
                &pair_tail_presence,
            )
    });

    if configs.is_empty() {
        return Refuted(0);
    }

    // Ordered-prefix propagation can prove additional exact windows
    // unreachable.  Intermediate rounds now shrink only the compact exact
    // window relation; parity/count aggregates are intentionally stale during
    // these rounds because SidePossible/SidePrefixPossible consult only
    // `right/left/any`.  If the relation actually shrinks, rebuild all
    // aggregates and cheap summaries exactly once at the final fixed point.
    let mut prefix_refined_windows = false;
    // Same-witness whole-side parity is enabled only when the initial target
    // frontier contains an unknown tape end.  Fully finite blank targets use
    // the original shape-only joint domain, where this refinement has not added
    // decisions but does add forward fixed-point work.
    let track_joint_run_parity = configs.iter().any(|config| {
        config.tape.lspan.end == TapeEnd::Unknown
            || config.tape.rspan.end == TapeEnd::Unknown
    });
    let mut crossing_dirty = false;
    let (
        side_prefix_possible,
        joint_short_possible,
        joint_side_prefix_fixed,
        joint_side_word_prefix_possible,
    ) = loop {
        if crossing_dirty {
            let (sides, _, _, next_radius2) =
                refine_windows_by_crossings_and_sides(
                    prog,
                    &mut win_possible,
                );
            side_possible = sides;
            radius2_possible = next_radius2;
            color_tail_count =
                color_tail_count_from_blank(prog, &win_possible);
            side_possible.refine_zero_tail_pairs(&color_tail_count);
        }

        let (mut prefixes, prefix_trans, prefix_exposed) = prog
            .side_run_prefix_possible_from_blank(
                &win_possible,
                &side_possible,
            );

        // First feed back only the cheap run-prefix projection. If that already
        // removes a window, restart immediately and avoid constructing the much
        // richer ordered word-prefix fixed point for this round.
        if win_possible
            .refine_run_prefix_reachability_relation(&prefixes)
        {
            prefix_refined_windows = true;
            crossing_dirty = true;
            continue;
        }

        Prog::<s, c>::populate_side_word_prefix_possible_from_blank(
            &win_possible,
            &prefix_trans,
            &prefix_exposed,
            &mut prefixes,
        );

        if win_possible
            .refine_word_prefix_reachability_relation(&prefixes)
        {
            prefix_refined_windows = true;
            crossing_dirty = true;
            continue;
        }

        // Both independent prefix projections are now at the current window
        // fixed point. Reject targets before paying for stronger joint domains.
        configs.retain(|Config { state, tape }| {
            tape.obeys_side_prefix_possible(*state, &prefixes)
        });
        if configs.is_empty() {
            return Refuted(0);
        }

        let joint = prog.joint_short_possible_from_blank(
            &win_possible,
            &radius2_possible,
        );
        if win_possible.refine_joint_short_reachability_relation(&joint)
        {
            prefix_refined_windows = true;
            crossing_dirty = true;
            continue;
        }

        configs.retain(|Config { state, tape }| {
            tape.obeys_joint_short_possible(*state, &joint)
        });
        if configs.is_empty() {
            return Refuted(0);
        }

        // Feed the stronger parity-aware reduced product back into the local
        // window relation too.  This is deliberately after the cheaper two
        // projections so most impossible windows are gone before paying for
        // the larger joint worklist.
        let joint_side = prog.joint_side_prefix_possible_from_blank(
            &win_possible,
            track_joint_run_parity,
        );
        if win_possible
            .refine_joint_side_prefix_reachability_relation(&joint_side)
        {
            prefix_refined_windows = true;
            crossing_dirty = true;
            continue;
        }

        configs.retain(|Config { state, tape }| {
            tape.obeys_joint_side_prefix_possible(
                *state,
                Some(&joint_side),
            )
        });
        if configs.is_empty() {
            return Refuted(0);
        }

        let joint_word = prog
            .joint_side_word_prefix_possible_from_blank(&win_possible);
        if win_possible
            .refine_joint_side_word_reachability_relation(&joint_word)
        {
            prefix_refined_windows = true;
            crossing_dirty = true;
            continue;
        }

        configs.retain(|Config { state, tape }| {
            tape.obeys_joint_side_word_prefix_possible(
                *state,
                &joint_word,
            )
        });
        if configs.is_empty() {
            return Refuted(0);
        }

        break (prefixes, joint, joint_side, joint_word);
    };

    // Both sides can query the joint run relation, including targets and
    // later predecessors with no exactly blank side.
    let joint_side_prefix_possible = Some(joint_side_prefix_fixed);

    if prefix_refined_windows {
        // Finalize every exact/aggregate table once.  `side_possible` was
        // computed on the already-prefix-refined relation, so this cannot
        // resurrect a removed exact window.
        win_possible.refine_reachability(&side_possible);

        blank_side_possible =
            blank_side_possible_from_blank(prog, &win_possible);
        color_tail_count =
            color_tail_count_from_blank(prog, &win_possible);
        side_possible.refine_zero_tail_pairs(&color_tail_count);
        pair_tail_presence =
            pair_tail_presence_from_blank(prog, &win_possible);

        // The smaller final window graph can sharpen all of the cheap filters
        // too, so recheck only the targets that survived their first pass.
        configs.retain(|Config { state, tape }| {
            window_nonblank_parity_possible(*state, tape, &win_possible)
                && window_side_nonblank_parity_possible(
                    *state,
                    tape,
                    &win_possible,
                )
                && window_side_nonblank_mod3_possible(
                    *state,
                    tape,
                    &win_possible,
                )
                && window_color_parity_possible(
                    *state,
                    tape,
                    &win_possible,
                )
                && window_possible(*state, tape, &win_possible)
                && window_radius2_possible(
                    *state,
                    tape,
                    &radius2_possible,
                )
                && tape.obeys_state_side(*state, &side_possible)
                && tape.obeys_blank_side_possible(
                    *state,
                    &blank_side_possible,
                )
                && tape.obeys_tail_presence(
                    *state,
                    &color_tail_count,
                    &pair_tail_presence,
                )
        });

        if configs.is_empty() {
            return Refuted(0);
        }
    }

    let side_triple_possible = prog
        .side_triple_possible_from_blank(&win_possible, &side_possible);

    // Check the cheaper independent triple domain before constructing the
    // cross-side product. If it already rejects every target, the joint
    // triple fixed point is pure overhead.
    configs.retain(|Config { state, tape }| {
        tape.obeys_state_triples(*state, &side_triple_possible)
    });

    if configs.is_empty() {
        return Refuted(0);
    }

    let joint_side_triple_possible = prog
        .joint_side_triple_possible_from_blank(
            &win_possible,
            &side_possible,
        );

    // Prefix/joint domains were already checked as soon as each reached the
    // final window fixed point above. The final cheap-summary rebuild only
    // removes configs and does not mutate their tapes, so repeating those hot
    // matchers here would be redundant. Only the newly constructed joint
    // triple domain remains to check.
    configs.retain(|Config { state, tape }| {
        tape.obeys_joint_state_triples(
            *state,
            &joint_side_triple_possible,
        )
    });

    if configs.is_empty() {
        return Refuted(0);
    }

    let mut blanks = get_blanks(&configs);

    // In cycle mode, exact repeats are ordinary graph cycles and can be
    // discarded immediately. Growing-count cycles are handled more narrowly:
    // the retry records exact predecessor *edges* and only cuts an edge when
    // that very edge would overflow a run after a long, stable recurrence.
    // This overflow analysis never sees the ordinary-pass monochromatic
    // stride widening; the retry starts from fresh exact targets. Compound-word
    // widening is handled independently below.
    let mut cycle_seen: Option<Dict<(State, u64), Vec<Tape>>> =
        CYCLE_ANALYSIS.then(Dict::new);
    let mut overflow_cycle_history = OverflowCycleHistory::default();
    let mut run_spine_widening_history =
        RunSpineWideningHistory::default();
    let mut word_widening_history = WordWideningHistory::default();
    let mut widening_active = false;

    // Widened run/word states can recur even with unknown tape ends. Unlike the
    // old hash-only blank-end shortcut, resolve collisions by exact equality.
    let mut widened_seen: Dict<(State, u64), Vec<Tape>> = Dict::new();

    // Optional exact historical repeat filter, enabled only for `twostep`.
    // Exact Tape equality resolves hash collisions without relying on hash
    // uniqueness.  Absolute head position is intentionally not tracked: the
    // infinite tape is translation-invariant.
    let mut exact_seen: Option<Dict<(State, u64), Vec<Tape>>> =
        use_exact_seen.then(Dict::new);

    let mut seen: Set<(State, u64)> = Set::new();

    for step in 1..=steps {
        if let Some(cycle_seen) = &mut cycle_seen {
            configs.retain(|Config { state, tape }| {
                let key = (*state, tape.hash());
                let bucket = cycle_seen.entry(key).or_default();

                if bucket.contains(tape) {
                    return false;
                }

                bucket.push(tape.clone());
                true
            });
        } else {
            configs.retain(|Config { state, tape }| {
                if widening_active
                    && (tape.has_indef_word() || tape.has_stride_run())
                {
                    let key = (*state, tape.hash());
                    let bucket = widened_seen.entry(key).or_default();
                    if bucket.contains(tape) {
                        return false;
                    }
                    bucket.push(tape.clone());
                    return true;
                }

                let blank_ends = tape.lspan.end == TapeEnd::Blanks
                    && tape.rspan.end == TapeEnd::Blanks;

                !blank_ends || seen.insert((*state, tape.hash()))
            });
        }

        #[cfg(debug_assertions)]
        {
            for config in &configs {
                println!("{step} | {config}");
            }
            println!();
        };

        let valid_steps =
            match get_valid_steps(&mut configs, entrypoints) {
                Err(err) => return err,
                Ok(valid_steps) => valid_steps,
            };

        match valid_steps.len() {
            0 => return Refuted(step),
            n if MAX_STACK_DEPTH < n => return DepthLimit,
            _ => {},
        }

        let mut stepped = match step_configs::<s, c, CYCLE_ANALYSIS>(
            valid_steps,
            step,
            &mut overflow_cycle_history,
            &mut blanks,
            &win_possible,
            &radius2_possible,
            &side_possible,
            &side_triple_possible,
            &joint_side_triple_possible,
            &side_prefix_possible,
            &joint_short_possible,
            joint_side_prefix_possible.as_ref(),
            &joint_side_word_prefix_possible,
            &blank_side_possible,
            &color_tail_count,
            &pair_tail_presence,
            &forbid_left,
            &forbid_right,
            left_fresh_zero,
            right_fresh_zero,
            left_forced_blank,
            right_forced_blank,
            &nonblank_parity,
        ) {
            Err(err) => return err,
            Ok(stepped) => stepped,
        };

        for config in &mut stepped {
            // Preserve the pre-existing word-widening observation stream before
            // changing any run count representation on this configuration.
            let word_widened =
                word_widening_history.widen(config, step);
            let run_widened = !CYCLE_ANALYSIS
                && run_spine_widening_history.widen(config, step);
            if run_widened || word_widened {
                widening_active = true;
                #[cfg(debug_assertions)]
                if run_widened {
                    println!("run-widen | {config}");
                } else {
                    println!("word-widen | {config}");
                }
            }
        }

        if let Some(exact_seen) = &mut exact_seen {
            let mut kept = Configs::with_capacity(stepped.len());
            for config in stepped {
                let key = (config.state, config.tape.hash());
                let bucket = exact_seen.entry(key).or_default();

                if bucket.contains(&config.tape) {
                    continue;
                }

                bucket.push(config.tape.clone());
                kept.push(config);
            }
            configs = kept;
        } else {
            configs = stepped;
        }
    }

    StepLimit
}

type ValidatedSteps = Vec<(Vec<Instr>, Config)>;

fn get_valid_steps(
    configs: &mut Configs,
    entrypoints: &Entrypoints,
) -> Result<ValidatedSteps, BackwardResult> {
    let mut checked = ValidatedSteps::with_capacity(configs.len());

    for config in configs.drain(..) {
        let Config { state, tape } = &config;

        let Some((same, diff)) = entrypoints.get(state) else {
            assert_eq!(*state, 0);
            continue;
        };

        let mut steps = Vec::with_capacity(same.len() + diff.len());

        for &((next_state, color), (print, shift)) in diff {
            if !tape.is_valid_step(shift, print) {
                continue;
            }

            steps.push((color, shift, next_state));
        }

        for &((_, color), (print, shift)) in same {
            if !tape.is_valid_step(shift, print) {
                continue;
            }

            if !tape.is_spinout(shift, color) {
                steps.push((color, shift, *state));
                continue;
            }

            if let Some(indef) = get_indef(shift, &config, diff, same)?
            {
                checked.push(indef);
            }
        }

        if steps.is_empty() {
            continue;
        }

        checked.push((steps, config));
    }

    Ok(checked)
}

fn get_indef(
    push: Shift,
    config: &Config,
    diff: &Entries,
    same: &Entries,
) -> Result<Option<(Vec<Instr>, Config)>, BackwardResult> {
    let mut tape = config.tape.clone();
    tape.push_indef(push)?;

    // Extending an already-known blank tail with an indefinite run of 0s is
    // canonicalized away by `push_indef`. In that case this branch is exactly
    // the ordinary non-spinout branch: same tape and, because the spinout edge
    // itself is excluded below, the same eligible predecessor instructions.
    // Returning it again only duplicates the whole subsequent frontier.
    if tape == config.tape {
        return Ok(None);
    }

    // Avoid cloning `diff` and constructing a temporary combined entry list.
    // Preserve the original order: different-state entries first, followed by
    // eligible same-state entries.
    let same_entries =
        same.iter().copied().filter(|&((_, color), (_, shift))| {
            shift != push || color != config.tape.scan
        });
    let mut steps = Vec::with_capacity(diff.len() + same.len());

    for ((state, color), (print, shift)) in
        diff.iter().copied().chain(same_entries)
    {
        if tape.is_valid_step(shift, print) {
            steps.push((color, shift, state));
        }
    }

    if steps.is_empty() {
        return Ok(None);
    }

    let next_config = Config::new(config.state, tape);

    #[cfg(debug_assertions)]
    println!("~ | {next_config}");

    Ok(Some((steps, next_config)))
}

fn window_possible<const s: usize, const c: usize>(
    state: State,
    tape: &Tape,
    win_possible: &WinPossible<s, c>,
) -> bool {
    let st = state as usize;
    let sc = tape.scan as usize;

    let l = tape.left_neighbor_color().map(|x| x as usize);
    let r = tape.right_neighbor_color().map(|x| x as usize);

    match (l, r) {
        (Some(lc), Some(rc)) => {
            (win_possible.right[st][sc][lc] & (1_u64 << rc)) != 0
        },
        (Some(lc), None) => win_possible.right[st][sc][lc] != 0,
        (None, Some(rc)) => win_possible.left[st][sc][rc] != 0,
        (None, None) => win_possible.any[st][sc],
    }
}

fn window_radius2_possible<const S: usize, const C: usize>(
    state: State,
    tape: &Tape,
    possible: &Radius2Possible<S, C>,
) -> bool {
    if !possible.enabled {
        return true;
    }

    let left2 = tape.left_second_neighbor_color().map(usize::from);
    let right2 = tape.right_second_neighbor_color().map(usize::from);

    // With neither second neighbor fixed, radius-2 adds nothing beyond the
    // ordinary local-window test that is already run immediately before this.
    if left2.is_none() && right2.is_none() {
        return true;
    }

    let st = usize::from(state);
    let scan = usize::from(tape.scan);
    let left = tape.left_neighbor_color().map(usize::from);
    let right = tape.right_neighbor_color().map(usize::from);

    let left_start = left.unwrap_or(0);
    let left_end = left.map_or(C, |left| left + 1);
    let right_start = right.unwrap_or(0);
    let right_end = right.map_or(C, |right| right + 1);

    for left in left_start..left_end {
        for right in right_start..right_end {
            if possible.compatible(st, scan, left, right, left2, right2)
            {
                return true;
            }
        }
    }

    false
}

fn window_neighbor_mask<const S: usize, const C: usize>(
    state: usize,
    scan: usize,
    shift: Shift,
    possible: &WinPossible<S, C>,
) -> u64 {
    if shift {
        possible.right[state][scan]
            .iter()
            .copied()
            .fold(0, |mask, colors| mask | colors)
    } else {
        possible.left[state][scan]
            .iter()
            .copied()
            .fold(0, |mask, colors| mask | colors)
    }
}

fn nonblank_parity_possible<const s: usize>(
    state: State,
    tape: &Tape,
    parity: &NonblankParity<s>,
) -> bool {
    let st = state as usize;

    (parity.possible[st] & tape.nonblank_parity_mask()) != 0
}

fn window_nonblank_parity_possible<const S: usize, const C: usize>(
    state: State,
    tape: &Tape,
    possible: &WinPossible<S, C>,
) -> bool {
    let required = tape.nonblank_parity_mask();

    // Unknown ends or indefinite nonblank runs permit either parity, so this
    // invariant cannot prune them. Avoid even the small window lookup in the
    // common halt-target case.
    if required == 0b11 {
        return true;
    }

    let st = state as usize;
    let sc = tape.scan as usize;
    let left = tape.left_neighbor_color().map(usize::from);
    let right = tape.right_neighbor_color().map(usize::from);

    let parity_mask = match (left, right) {
        (Some(left), Some(right)) => {
            possible.exact_parity_mask(st, sc, left, right)
        },
        (Some(left), None) => possible.parity_right[st][sc][left],
        (None, Some(right)) => possible.parity_left[st][sc][right],
        (None, None) => possible.parity_any[st][sc],
    };

    (parity_mask & required) != 0
}

fn window_side_nonblank_parity_possible<
    const S: usize,
    const C: usize,
>(
    state: State,
    tape: &Tape,
    possible: &WinPossible<S, C>,
) -> bool {
    let (left_required, right_required) =
        tape.side_nonblank_parity_masks();

    // If neither side has a fixed parity, this abstraction cannot prune.
    if left_required == 0b11 && right_required == 0b11 {
        return true;
    }

    let mut required_pairs = 0_u8;
    for left_parity in 0..2 {
        if left_required & (1_u8 << left_parity) == 0 {
            continue;
        }
        for right_parity in 0..2 {
            if right_required & (1_u8 << right_parity) == 0 {
                continue;
            }
            let pair = left_parity | (right_parity << 1);
            required_pairs |= 1_u8 << pair;
        }
    }

    let st = state as usize;
    let sc = tape.scan as usize;
    let left = tape.left_neighbor_color().map(usize::from);
    let right = tape.right_neighbor_color().map(usize::from);

    let possible_pairs = match (left, right) {
        (Some(left), Some(right)) => {
            possible.exact_side_parity_mask(st, sc, left, right)
        },
        (Some(left), None) => possible.side_parity_right[st][sc][left],
        (None, Some(right)) => possible.side_parity_left[st][sc][right],
        (None, None) => possible.side_parity_any[st][sc],
    };

    possible_pairs & required_pairs != 0
}

fn window_side_nonblank_mod3_possible<
    const S: usize,
    const C: usize,
>(
    state: State,
    tape: &Tape,
    possible: &WinPossible<S, C>,
) -> bool {
    let (left_required, right_required) =
        tape.side_nonblank_mod3_masks();

    // Unknown ends or indefinite nonblank runs allow every residue.
    if left_required == 0b111 && right_required == 0b111 {
        return true;
    }

    let mut required_pairs = 0_u16;
    for left_residue in 0..3 {
        if left_required & (1_u8 << left_residue) == 0 {
            continue;
        }
        for right_residue in 0..3 {
            if right_required & (1_u8 << right_residue) == 0 {
                continue;
            }
            let pair = left_residue + 3 * right_residue;
            required_pairs |= 1_u16 << pair;
        }
    }

    let st = state as usize;
    let sc = tape.scan as usize;
    let left = tape.left_neighbor_color().map(usize::from);
    let right = tape.right_neighbor_color().map(usize::from);

    let possible_pairs = match (left, right) {
        (Some(left), Some(right)) => {
            possible.exact_side_mod3_mask(st, sc, left, right)
        },
        (Some(left), None) => possible.side_mod3_right[st][sc][left],
        (None, Some(right)) => possible.side_mod3_left[st][sc][right],
        (None, None) => possible.side_mod3_any[st][sc],
    };

    possible_pairs & required_pairs != 0
}

fn window_color_parity_possible<const S: usize, const C: usize>(
    state: State,
    tape: &Tape,
    possible: &WinPossible<S, C>,
) -> bool {
    if !WinPossible::<S, C>::color_parity_enabled() {
        return true;
    }

    let required = tape.color_parity_mask::<C>();
    if required == WinPossible::<S, C>::all_color_parity_vectors() {
        return true;
    }

    let st = state as usize;
    let sc = tape.scan as usize;
    let left = tape.left_neighbor_color().map(usize::from);
    let right = tape.right_neighbor_color().map(usize::from);

    let possible_vectors = match (left, right) {
        (Some(left), Some(right)) => {
            possible.exact_color_parity_mask(st, sc, left, right)
        },
        (Some(left), None) => possible.color_parity_right[st][sc][left],
        (None, Some(right)) => {
            possible.color_parity_left[st][sc][right]
        },
        (None, None) => possible.color_parity_any[st][sc],
    };

    possible_vectors & required != 0
}

#[expect(clippy::fn_params_excessive_bools, clippy::too_many_arguments)]
fn step_instrs<
    const s: usize,
    const c: usize,
    const CYCLE_ANALYSIS: bool,
>(
    instrs: impl IntoIterator<Item = Instr>,
    config: &Config,
    step: Steps,
    overflow_cycle_history: &mut OverflowCycleHistory,
    blanks: &mut BlankStates,
    win_possible: &WinPossible<s, c>,
    radius2_possible: &Radius2Possible<s, c>,
    side_possible: &SidePossible<s, c>,
    side_triple_possible: &SideTriplePossible<s, c>,
    joint_side_triple_possible: &JointSideTriplePossible<s, c>,
    side_prefix_possible: &SidePrefixPossible<s, c>,
    joint_short_possible: &JointShortPossible<s, c>,
    joint_side_prefix_possible: Option<&JointSidePrefixPossible<s, c>>,
    joint_side_word_prefix_possible: &JointSideWordPrefixPossible<s, c>,
    blank_side_possible: &BlankSidePossible<s, c>,
    color_tail_count: &ColorTailCountPossible<s, c>,
    pair_tail_presence: &PairTailPresencePossible<s, c>,
    forbid_left: &[bool; c],
    forbid_right: &[bool; c],
    left_fresh_zero: bool,
    right_fresh_zero: bool,
    left_forced_blank: bool,
    right_forced_blank: bool,
    nonblank_parity: &NonblankParity<s>,
    stepped: &mut Configs,
) -> Result<(), BackwardResult> {
    for (color, shift, state) in instrs {
        let instr = (color, shift, state);
        let growth = CYCLE_ANALYSIS
            .then(|| growth_edge_observation(config, instr))
            .flatten();

        if let Some((key, count)) = growth {
            if count == Count::MAX {
                if overflow_cycle_history.certifies(&key, step, count) {
                    #[cfg(debug_assertions)]
                    println!("cycle-cut | {config} via {instr:?}");
                    continue;
                }

                return Err(CountLimit);
            }

            // Record the exact attempted edge, not merely children surviving
            // later static filters. This is important for count-one splits of
            // an indefinite run: such an auxiliary branch can hit the same
            // overflowing push before its child would have been pruned.
            overflow_cycle_history.observe(key, step, count);
        }

        let mut tape = config.tape.clone();
        tape.backstep(shift, color)?;

        if tape.blank() {
            if state == 0 {
                return Err(Init);
            }

            if !blanks.insert(state) {
                continue;
            }
        }

        // Retain the original full-span static checks. This helper only avoids
        // the temporary `branch_indef` frontier and its instruction vectors.
        if !tape.obeys_shift_side(forbid_left, forbid_right) {
            continue;
        }

        if !tape.tighten_forced_blank_ends(
            left_forced_blank,
            right_forced_blank,
        ) {
            continue;
        }

        if (left_fresh_zero || right_fresh_zero)
            && !tape.enforce_fresh_zero_side_invariants(
                left_fresh_zero,
                right_fresh_zero,
            )
        {
            continue;
        }

        if !nonblank_parity_possible(state, &tape, nonblank_parity)
            || !window_nonblank_parity_possible(
                state,
                &tape,
                win_possible,
            )
            || !window_side_nonblank_parity_possible(
                state,
                &tape,
                win_possible,
            )
            || !window_side_nonblank_mod3_possible(
                state,
                &tape,
                win_possible,
            )
            || !window_color_parity_possible(state, &tape, win_possible)
        {
            continue;
        }

        if !window_possible(state, &tape, win_possible)
            || !window_radius2_possible(state, &tape, radius2_possible)
            || !tape.obeys_state_side(state, side_possible)
            || !tape.obeys_state_triples(state, side_triple_possible)
            || !tape.obeys_joint_state_triples(
                state,
                joint_side_triple_possible,
            )
            || !tape
                .obeys_blank_side_possible(state, blank_side_possible)
            || !tape.obeys_tail_presence(
                state,
                color_tail_count,
                pair_tail_presence,
            )
        {
            continue;
        }

        // The tiny joint prefix needs no compiled backward run/word
        // requirements, so let it reject first.  The three richer prefix
        // matchers below then share one lazily-built requirement bundle.
        if !tape.obeys_joint_short_possible(state, joint_short_possible)
        {
            continue;
        }

        let mut side_requirements = SideMatchRequirements::default();
        if !tape.obeys_side_prefix_possible_cached(
            state,
            side_prefix_possible,
            &mut side_requirements,
        ) || !tape.obeys_joint_side_prefix_possible_cached(
            state,
            joint_side_prefix_possible,
            &mut side_requirements,
        ) || !tape.obeys_joint_side_word_prefix_possible_cached(
            state,
            joint_side_word_prefix_possible,
            &mut side_requirements,
        ) {
            continue;
        }

        stepped.push(Config::new(state, tape));
    }

    Ok(())
}

#[expect(clippy::fn_params_excessive_bools, clippy::too_many_arguments)]
fn step_configs<
    const s: usize,
    const c: usize,
    const CYCLE_ANALYSIS: bool,
>(
    configs: ValidatedSteps,
    step: Steps,
    overflow_cycle_history: &mut OverflowCycleHistory,
    blanks: &mut BlankStates,
    win_possible: &WinPossible<s, c>,
    radius2_possible: &Radius2Possible<s, c>,
    side_possible: &SidePossible<s, c>,
    side_triple_possible: &SideTriplePossible<s, c>,
    joint_side_triple_possible: &JointSideTriplePossible<s, c>,
    side_prefix_possible: &SidePrefixPossible<s, c>,
    joint_short_possible: &JointShortPossible<s, c>,
    joint_side_prefix_possible: Option<&JointSidePrefixPossible<s, c>>,
    joint_side_word_prefix_possible: &JointSideWordPrefixPossible<s, c>,
    blank_side_possible: &BlankSidePossible<s, c>,
    color_tail_count: &ColorTailCountPossible<s, c>,
    pair_tail_presence: &PairTailPresencePossible<s, c>,
    forbid_left: &[bool; c],
    forbid_right: &[bool; c],
    left_fresh_zero: bool,
    right_fresh_zero: bool,
    left_forced_blank: bool,
    right_forced_blank: bool,
    nonblank_parity: &NonblankParity<s>,
) -> Result<Configs, BackwardResult> {
    let mut stepped = Configs::new();

    for (instrs, config) in configs {
        // Fuse `branch_indef` into stepping. The old processing order is
        // preserved: left count-one branch, right count-one branch, original.
        let split_left = config.tape.pull_needs_count_one_split(true);
        let split_right = config.tape.pull_needs_count_one_split(false);

        if split_left && instrs.iter().any(|&(_, shift, _)| shift) {
            let mut count_1 = config.clone();
            count_1.tape.lspan.set_head_to_one();

            step_instrs::<s, c, CYCLE_ANALYSIS>(
                instrs.iter().copied().filter(|&(_, shift, _)| shift),
                &count_1,
                step,
                overflow_cycle_history,
                blanks,
                win_possible,
                radius2_possible,
                side_possible,
                side_triple_possible,
                joint_side_triple_possible,
                side_prefix_possible,
                joint_short_possible,
                joint_side_prefix_possible,
                joint_side_word_prefix_possible,
                blank_side_possible,
                color_tail_count,
                pair_tail_presence,
                forbid_left,
                forbid_right,
                left_fresh_zero,
                right_fresh_zero,
                left_forced_blank,
                right_forced_blank,
                nonblank_parity,
                &mut stepped,
            )?;
        }

        if split_right && instrs.iter().any(|&(_, shift, _)| !shift) {
            let mut count_1 = config.clone();
            count_1.tape.rspan.set_head_to_one();

            step_instrs::<s, c, CYCLE_ANALYSIS>(
                instrs.iter().copied().filter(|&(_, shift, _)| !shift),
                &count_1,
                step,
                overflow_cycle_history,
                blanks,
                win_possible,
                radius2_possible,
                side_possible,
                side_triple_possible,
                joint_side_triple_possible,
                side_prefix_possible,
                joint_short_possible,
                joint_side_prefix_possible,
                joint_side_word_prefix_possible,
                blank_side_possible,
                color_tail_count,
                pair_tail_presence,
                forbid_left,
                forbid_right,
                left_fresh_zero,
                right_fresh_zero,
                left_forced_blank,
                right_forced_blank,
                nonblank_parity,
                &mut stepped,
            )?;
        }

        step_instrs::<s, c, CYCLE_ANALYSIS>(
            instrs,
            &config,
            step,
            overflow_cycle_history,
            blanks,
            win_possible,
            radius2_possible,
            side_possible,
            side_triple_possible,
            joint_side_triple_possible,
            side_prefix_possible,
            joint_short_possible,
            joint_side_prefix_possible,
            joint_side_word_prefix_possible,
            blank_side_possible,
            color_tail_count,
            pair_tail_presence,
            forbid_left,
            forbid_right,
            left_fresh_zero,
            right_fresh_zero,
            left_forced_blank,
            right_forced_blank,
            nonblank_parity,
            &mut stepped,
        )?;
    }

    Ok(stepped)
}

/**************************************/

fn halt_configs(halt_slots: &Set<Slot>) -> Configs {
    halt_slots
        .iter()
        .map(|&(state, color)| Config::init_halt(state, color))
        .collect()
}

fn erase_configs(erase_slots: &Set<Slot>) -> Configs {
    erase_slots
        .iter()
        .map(|&(state, color)| Config::init_blank(state, color))
        .collect()
}

fn zr_configs(zr_shifts: &Set<(State, Shift)>) -> Configs {
    zr_shifts
        .iter()
        .map(|&(state, shift)| Config::init_spinout(state, shift))
        .collect()
}

fn twostep_configs(twosteps: &Set<(State, (Color, Color))>) -> Configs {
    twosteps
        .iter()
        .map(|&(st, (l_co, r_co))| Config::init_twostep(st, l_co, r_co))
        .collect()
}

fn get_blanks(configs: &Configs) -> BlankStates {
    configs
        .iter()
        .filter_map(|cfg| cfg.tape.blank().then_some(cfg.state))
        .collect()
}

/**************************************/

#[expect(clippy::multiple_inherent_impl)]
impl<const s: usize, const c: usize> Prog<s, c> {
    fn get_entrypoints(&self) -> Entrypoints {
        let mut entrypoints = Entrypoints::new();

        for (slot @ (read, _), &(color, shift, state)) in self.iter() {
            let (same, diff) = entrypoints.entry(state).or_default();

            (if read == state { same } else { diff })
                .push((slot, (color, shift)));
        }

        entrypoints
    }

    /// Returns (writes_blank_on_r, writes_blank_on_l):
    /// - writes_blank_on_r is true if any transition writes 0 and moves Right.
    /// - writes_blank_on_l is true if any transition writes 0 and moves Left.
    ///
    /// This enables one-sided "fresh blank" invariants: if blank is never
    /// written on R-moves, then any 0 to the left of the head must be
    /// unvisited; similarly for the right side with L-moves.
    fn blank_writes_by_shift(&self) -> (bool, bool) {
        let mut on_r = false;
        let mut on_l = false;

        for (_, &(print, shift, _)) in self.iter() {
            if print != 0 {
                continue;
            }
            if shift {
                on_r = true;
            } else {
                on_l = true;
            }

            if on_r && on_l {
                break;
            }
        }

        (on_r, on_l)
    }

    /// Compute a sound over-approximation of the parity of the number of
    /// nonblank tape cells in each state, starting from the blank tape.
    ///
    /// A transition changes this parity exactly when one of `read` and
    /// `print` is blank and the other is nonblank. We retain only the state
    /// and this one parity bit, so every concrete run maps to a path in this
    /// finite abstract graph. Consequently, any absent parity bit is a valid
    /// invariant for backward pruning.
    fn nonblank_parity_from_blank(&self) -> NonblankParity<s> {
        let mut possible = [0_u8; s];
        possible[0] = 0b01; // initial state, entirely blank tape

        loop {
            let mut changed = false;

            for ((state, read), &(print, _, next_state)) in self.iter()
            {
                let state = state as usize;
                let next_state = next_state as usize;

                let source = possible[state];
                if source == 0 {
                    continue;
                }

                let flips = (read == 0) != (print == 0);
                let reached = if flips {
                    ((source & 0b01) << 1) | ((source & 0b10) >> 1)
                } else {
                    source
                };

                let old = possible[next_state];
                possible[next_state] |= reached;
                changed |= possible[next_state] != old;
            }

            if !changed {
                break;
            }
        }

        NonblankParity { possible }
    }

    /// Compute a *sound* shift-side restriction for each color.
    ///
    /// For a non-blank color `k != 0`:
    /// - If the machine never writes `k` on an L-move, then `k` can never
    ///   appear to the **right** of the head in any run from the
    ///   blank tape.
    /// - If the machine never writes `k` on an R-move, then `k` can
    ///   never appear to the **left** of the head in any run from the
    ///   blank tape.
    ///
    /// This is the classic invariant used in "shift-side" analysis:
    /// to get a symbol to the opposite side of the head you must
    /// *cross* it, and crossing requires leaving it behind via a move
    /// in that direction. If that direction never writes the symbol,
    /// the symbol cannot survive the crossing.
    fn shift_side_forbidden(&self) -> ([bool; c], [bool; c]) {
        // right_writes[k] == true if *any* transition writes k and moves R
        // left_writes[k]  == true if *any* transition writes k and moves L
        let mut left_writes = [false; c];
        let mut right_writes = [false; c];

        for (_, &(print, shift, _)) in self.iter() {
            (if shift {
                &mut right_writes
            } else {
                &mut left_writes
            })[print as usize] = true;
        }

        let mut forbid_left = [false; c];
        let mut forbid_right = [false; c];

        // Never forbid blanks (0) on either side.
        for k in 1..c {
            // If k is never written on an R-move, it cannot appear on the left.
            forbid_left[k] = !right_writes[k];
            // If k is never written on an L-move, it cannot appear on the right.
            forbid_right[k] = !left_writes[k];
        }

        (forbid_left, forbid_right)
    }

    /// Compute a sound over-approximation of which *immediate neighbor
    /// colors* can appear next to the head in each (state, scanned
    /// color), starting from the blank tape.
    ///
    /// We explore the abstract state space (q, L, S, R) where L and R
    /// are the colors immediately to the left/right of the head, and S
    /// is the scanned color. When the head moves off the 3-cell
    /// window, a known-blank outside tail exposes an exact zero;
    /// otherwise we conservatively treat the newly exposed cell as
    /// *unknown* (any color 0..c-1). This makes the analysis an
    /// over-approximation, and therefore safe for pruning: if a
    /// neighbor color is *not* possible here, it is not possible in any
    /// concrete run from blank.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::excessive_nesting,
        clippy::cognitive_complexity
    )]
    fn win_possible_from_blank(
        &self,
        forbid_left: &[bool; c],
        forbid_right: &[bool; c],
    ) -> WinPossible<s, c> {
        // Abstract state: (st, lb, l, sc, r, rb).
        // lb/rb = whether the whole tail immediately outside the 3-cell
        // window on that side is known blank.  The cells need not be
        // unvisited: only their current colors matter to this abstraction.
        fn idx<const C: usize, const S: usize>(
            st: usize,
            lb: usize,
            l: usize,
            sc: usize,
            r: usize,
            rb: usize,
        ) -> usize {
            // st * 2 * C^3 * 2 + ...
            let mut x = st;
            x = x * 2 + lb;
            x = x * C + l;
            x = x * C + sc;
            x = x * C + r;
            x = x * 2 + rb;
            x
        }

        let total = s * 2 * c * c * c * 2;

        // Each forward worklist stores the complete set of abstract residue
        // states known at one local-window state.  `processed` records which
        // bits have already been propagated.  If several predecessors add new
        // residue bits before a queued window is revisited, they are handled in
        // one batch rather than as separate queue entries.
        macro_rules! enqueue_mask {
            ($visited:ident, $queued:ident, $queue:ident, $n:expr, $mask:expr) => {{
                let n = $n;
                let mask = $mask;
                let id = idx::<c, s>(n.0, n.1, n.2, n.3, n.4, n.5);
                let added = mask & !$visited[id];
                if added != 0 {
                    $visited[id] |= added;
                    if !$queued[id] {
                        $queued[id] = true;
                        $queue.push_back(n);
                    }
                }
            }};
        }

        fn total_parity_mask(side_mask: u8, scan_nonblank: bool) -> u8 {
            let mut bits = side_mask;
            let mut out = 0_u8;
            let scan = u8::from(scan_nonblank);
            while bits != 0 {
                let code = bits.trailing_zeros() as u8;
                bits &= bits - 1;
                let left = code & 1;
                let right = (code >> 1) & 1;
                out |= 1_u8 << (left ^ right ^ scan);
            }
            out
        }

        const fn xor_side_parity_mask(side_mask: u8, xor: u8) -> u8 {
            let mut bits = side_mask;
            let mut out = 0_u8;
            while bits != 0 {
                let code = bits.trailing_zeros() as u8;
                bits &= bits - 1;
                out |= 1_u8 << (code ^ xor);
            }
            out
        }

        const fn shift_mod3_mask(
            mask: u16,
            left_add: u8,
            right_add: u8,
        ) -> u16 {
            let mut bits = mask;
            let mut out = 0_u16;
            while bits != 0 {
                let code = bits.trailing_zeros() as u8;
                bits &= bits - 1;
                let left = code % 3;
                let right = code / 3;
                let next_left = (left + left_add) % 3;
                let next_right = (right + right_add) % 3;
                let next = next_left + 3 * next_right;
                out |= 1_u16 << next;
            }
            out
        }

        const fn xor_color_parity_mask(mask: u64, xor: u8) -> u64 {
            let mut bits = mask;
            let mut out = 0_u64;
            while bits != 0 {
                let vector = bits.trailing_zeros() as u8;
                bits &= bits - 1;
                out |= 1_u64 << (vector ^ xor);
            }
            out
        }

        assert!(c <= 64, "window bitmasks support at most 64 colors");

        let mut possible = WinPossible {
            right: [[[0; c]; c]; s],
            left: [[[0; c]; c]; s],
            any: [[false; c]; s],
            parity: vec![0; s * c * c * c],
            parity_right: [[[0; c]; c]; s],
            parity_left: [[[0; c]; c]; s],
            parity_any: [[0; c]; s],
            side_parity: vec![0; s * c * c * c],
            side_parity_right: [[[0; c]; c]; s],
            side_parity_left: [[[0; c]; c]; s],
            side_parity_any: [[0; c]; s],
            side_mod3: vec![0; s * c * c * c],
            side_mod3_right: [[[0; c]; c]; s],
            side_mod3_left: [[[0; c]; c]; s],
            side_mod3_any: [[0; c]; s],
            color_parity: vec![0; s * c * c * c],
            color_parity_right: [[[0; c]; c]; s],
            color_parity_left: [[[0; c]; c]; s],
            color_parity_any: [[0; c]; s],
        };

        // Four-bit mask per abstract window state. Each bit is one exact
        // `(left-side parity, right-side parity)` combination. Propagate all
        // newly discovered combinations for a window together.
        let mut visited = vec![0_u8; total];
        let mut processed = vec![0_u8; total];
        let mut queued = vec![false; total];
        let mut q = std::collections::VecDeque::new();
        let initial = idx::<c, s>(0, 1, 0, 0, 0, 1);
        visited[initial] = 0b0001;
        queued[initial] = true;
        q.push_back((0, 1, 0, 0, 0, 1));

        while let Some((st, lb, l, sc, r, rb)) = q.pop_front() {
            let id = idx::<c, s>(st, lb, l, sc, r, rb);
            queued[id] = false;
            let fresh = visited[id] & !processed[id];
            if fresh == 0 {
                continue;
            }
            processed[id] |= fresh;

            possible.right[st][sc][l] |= 1_u64 << r;
            possible.left[st][sc][r] |= 1_u64 << l;
            possible.any[st][sc] = true;

            let parity_index =
                WinPossible::<s, c>::parity_index(st, sc, l, r);
            let parity = total_parity_mask(fresh, sc != 0);
            possible.parity[parity_index] |= parity;
            possible.parity_right[st][sc][l] |= parity;
            possible.parity_left[st][sc][r] |= parity;
            possible.parity_any[st][sc] |= parity;

            possible.side_parity[parity_index] |= fresh;
            possible.side_parity_right[st][sc][l] |= fresh;
            possible.side_parity_left[st][sc][r] |= fresh;
            possible.side_parity_any[st][sc] |= fresh;

            let Some(&(print, shift, next_state)) =
                self.get(&(st as State, sc as Color))
            else {
                continue;
            };

            let p = print as usize;
            let ns = next_state as usize;

            if shift {
                let new_lb = usize::from(lb == 1 && l == 0);
                let xor = u8::from(p != 0) | (u8::from(r != 0) << 1);
                let next_mask = xor_side_parity_mask(fresh, xor);

                if rb == 1 {
                    enqueue_mask!(
                        visited,
                        queued,
                        q,
                        (ns, new_lb, p, r, 0, 1),
                        next_mask
                    );
                } else {
                    for new_r in 0..c {
                        if forbid_right[new_r] {
                            continue;
                        }
                        enqueue_mask!(
                            visited,
                            queued,
                            q,
                            (ns, new_lb, p, r, new_r, 0),
                            next_mask
                        );
                    }
                }
            } else {
                let new_rb = usize::from(rb == 1 && r == 0);
                let xor = u8::from(l != 0) | (u8::from(p != 0) << 1);
                let next_mask = xor_side_parity_mask(fresh, xor);

                if lb == 1 {
                    enqueue_mask!(
                        visited,
                        queued,
                        q,
                        (ns, 1, 0, l, p, new_rb),
                        next_mask
                    );
                } else {
                    for new_l in 0..c {
                        if forbid_left[new_l] {
                            continue;
                        }
                        enqueue_mask!(
                            visited,
                            queued,
                            q,
                            (ns, 0, new_l, l, p, new_rb),
                            next_mask
                        );
                    }
                }
            }
        }

        // Joint left/right nonblank counts modulo 3. As above, one queue item
        // represents all newly reached residues for its abstract local window.
        let mut mod3_visited = vec![0_u16; total];
        let mut mod3_processed = vec![0_u16; total];
        let mut mod3_queued = vec![false; total];
        let mut mod3_q = std::collections::VecDeque::new();
        mod3_visited[initial] = 1;
        mod3_queued[initial] = true;
        mod3_q.push_back((0, 1, 0, 0, 0, 1));

        while let Some((st, lb, l, sc, r, rb)) = mod3_q.pop_front() {
            let id = idx::<c, s>(st, lb, l, sc, r, rb);
            mod3_queued[id] = false;
            let fresh = mod3_visited[id] & !mod3_processed[id];
            if fresh == 0 {
                continue;
            }
            mod3_processed[id] |= fresh;

            let residue_index =
                WinPossible::<s, c>::parity_index(st, sc, l, r);
            possible.side_mod3[residue_index] |= fresh;
            possible.side_mod3_right[st][sc][l] |= fresh;
            possible.side_mod3_left[st][sc][r] |= fresh;
            possible.side_mod3_any[st][sc] |= fresh;

            let Some(&(print, shift, next_state)) =
                self.get(&(st as State, sc as Color))
            else {
                continue;
            };

            let p = print as usize;
            let ns = next_state as usize;

            if shift {
                let new_lb = usize::from(lb == 1 && l == 0);
                let left_add = u8::from(p != 0);
                let right_add = 2 * u8::from(r != 0); // -1 mod 3
                let next_mask =
                    shift_mod3_mask(fresh, left_add, right_add);

                if rb == 1 {
                    enqueue_mask!(
                        mod3_visited,
                        mod3_queued,
                        mod3_q,
                        (ns, new_lb, p, r, 0, 1),
                        next_mask
                    );
                } else {
                    for new_r in 0..c {
                        if forbid_right[new_r] {
                            continue;
                        }
                        enqueue_mask!(
                            mod3_visited,
                            mod3_queued,
                            mod3_q,
                            (ns, new_lb, p, r, new_r, 0),
                            next_mask
                        );
                    }
                }
            } else {
                let new_rb = usize::from(rb == 1 && r == 0);
                let left_add = 2 * u8::from(l != 0); // -1 mod 3
                let right_add = u8::from(p != 0);
                let next_mask =
                    shift_mod3_mask(fresh, left_add, right_add);

                if lb == 1 {
                    enqueue_mask!(
                        mod3_visited,
                        mod3_queued,
                        mod3_q,
                        (ns, 1, 0, l, p, new_rb),
                        next_mask
                    );
                } else {
                    for new_l in 0..c {
                        if forbid_left[new_l] {
                            continue;
                        }
                        enqueue_mask!(
                            mod3_visited,
                            mod3_queued,
                            mod3_q,
                            (ns, 0, new_l, l, p, new_rb),
                            next_mask
                        );
                    }
                }
            }
        }

        // Global per-color parity. A transition XORs the vector by a fixed
        // color mask, so the complete set of newly reached vectors can likewise
        // be permuted and propagated as one u64 bitset.
        if WinPossible::<s, c>::color_parity_enabled() {
            let mut color_visited = vec![0_u64; total];
            let mut color_processed = vec![0_u64; total];
            let mut color_queued = vec![false; total];
            let mut color_q = std::collections::VecDeque::new();
            color_visited[initial] = 1;
            color_queued[initial] = true;
            color_q.push_back((0, 1, 0, 0, 0, 1));

            while let Some((st, lb, l, sc, r, rb)) = color_q.pop_front()
            {
                let id = idx::<c, s>(st, lb, l, sc, r, rb);
                color_queued[id] = false;
                let fresh = color_visited[id] & !color_processed[id];
                if fresh == 0 {
                    continue;
                }
                color_processed[id] |= fresh;

                let vector_index =
                    WinPossible::<s, c>::parity_index(st, sc, l, r);
                possible.color_parity[vector_index] |= fresh;
                possible.color_parity_right[st][sc][l] |= fresh;
                possible.color_parity_left[st][sc][r] |= fresh;
                possible.color_parity_any[st][sc] |= fresh;

                let Some(&(print, shift, next_state)) =
                    self.get(&(st as State, sc as Color))
                else {
                    continue;
                };

                let p = print as usize;
                let ns = next_state as usize;
                let mut xor = 0_u8;
                if sc != 0 {
                    xor ^= 1_u8 << (sc - 1);
                }
                if p != 0 {
                    xor ^= 1_u8 << (p - 1);
                }
                let next_mask = xor_color_parity_mask(fresh, xor);

                if shift {
                    let new_lb = usize::from(lb == 1 && l == 0);
                    if rb == 1 {
                        enqueue_mask!(
                            color_visited,
                            color_queued,
                            color_q,
                            (ns, new_lb, p, r, 0, 1),
                            next_mask
                        );
                    } else {
                        for new_r in 0..c {
                            if forbid_right[new_r] {
                                continue;
                            }
                            enqueue_mask!(
                                color_visited,
                                color_queued,
                                color_q,
                                (ns, new_lb, p, r, new_r, 0),
                                next_mask
                            );
                        }
                    }
                } else {
                    let new_rb = usize::from(rb == 1 && r == 0);
                    if lb == 1 {
                        enqueue_mask!(
                            color_visited,
                            color_queued,
                            color_q,
                            (ns, 1, 0, l, p, new_rb),
                            next_mask
                        );
                    } else {
                        for new_l in 0..c {
                            if forbid_left[new_l] {
                                continue;
                            }
                            enqueue_mask!(
                                color_visited,
                                color_queued,
                                color_q,
                                (ns, 0, new_l, l, p, new_rb),
                                next_mask
                            );
                        }
                    }
                }
            }
        }

        possible
    }

    /// Compute a sound over-approximation of whole-side colors and adjacent
    /// pairs for each exact local window `(left, scan, right)`.
    ///
    /// The fixed point starts at the true blank window `(A, 0, 0, 0)`.  For a
    /// reachable source window, the summary itself over-approximates the color
    /// immediately beyond each known neighbor: if the right neighbor is `r`,
    /// `pairs[RIGHT][r]` contains every color that may follow it.  On an
    /// R-move we intersect that mask with the already-sound target
    /// `WinPossible` mask and propagate separately to each resulting exact
    /// target window `(print, r, new_right)`.  L-moves are symmetric.
    ///
    /// This retains local-window/whole-side correlation without increasing the
    /// forward window radius.  Copying the complete source side summaries is
    /// conservative (the moved-over neighbor may remain in the summary), while
    /// the newly pushed boundary pair is exact for the source window.
    fn side_possible_from_blank(
        &self,
        win_possible: &WinPossible<s, c>,
    ) -> SidePossible<s, c> {
        assert!(c <= 64, "side bitmasks support at most 64 colors");

        let mut possible = SidePossible::new();

        {
            let mut initial = WindowSideSummary::empty();
            initial.reachable = true;
            for side in [LEFT_SIDE, RIGHT_SIDE] {
                initial.colors[side] = 1;
                initial.pairs[side][0] = 1;
            }
            let index = SidePossible::<s, c>::index(0, 0, 0, 0);
            possible.insert_window_by_index(index, initial);
        }

        fn merge<const S: usize, const C: usize>(
            possible: &mut SidePossible<S, C>,
            target_index: usize,
            source: WindowSideSummary<C>,
            push_side: usize,
            print: usize,
            old_neighbor: usize,
            target_left: usize,
            target_right: usize,
        ) -> bool {
            let compact = possible.lookup[target_index];

            // First reach: construct the target directly from the source
            // instead of initializing a zero summary and OR-ing every field
            // into it.
            if compact == usize::MAX {
                let mut target = source;
                target.reachable = true;
                target.colors[LEFT_SIDE] |= 1_u64 << target_left;
                target.colors[RIGHT_SIDE] |= 1_u64 << target_right;
                target.colors[push_side] |= 1_u64 << print;
                target.pairs[push_side][print] |= 1_u64 << old_neighbor;
                possible.insert_window_by_index(target_index, target);
                return true;
            }

            let target = &mut possible.windows[compact];
            let mut changed = false;

            debug_assert!(target.reachable);

            for side in [LEFT_SIDE, RIGHT_SIDE] {
                let old_colors = target.colors[side];
                target.colors[side] |= source.colors[side];
                changed |= target.colors[side] != old_colors;

                for near in 0..C {
                    let old_pairs = target.pairs[side][near];
                    target.pairs[side][near] |=
                        source.pairs[side][near];
                    changed |= target.pairs[side][near] != old_pairs;
                }
            }

            // Both exact target neighbors must occur on their respective
            // sides. Usually these bits are already inherited, but setting
            // them explicitly keeps the representation self-contained.
            let old_left_colors = target.colors[LEFT_SIDE];
            target.colors[LEFT_SIDE] |= 1_u64 << target_left;
            changed |= target.colors[LEFT_SIDE] != old_left_colors;

            let old_right_colors = target.colors[RIGHT_SIDE];
            target.colors[RIGHT_SIDE] |= 1_u64 << target_right;
            changed |= target.colors[RIGHT_SIDE] != old_right_colors;

            let old_colors = target.colors[push_side];
            target.colors[push_side] |= 1_u64 << print;
            changed |= target.colors[push_side] != old_colors;

            let old_pairs = target.pairs[push_side][print];
            target.pairs[push_side][print] |= 1_u64 << old_neighbor;
            changed |= target.pairs[push_side][print] != old_pairs;

            changed
        }

        let mut trans = [[None; c]; s];
        for ((state, read), &(print, shift, next_state)) in self.iter()
        {
            let st = state as usize;
            let sc = read as usize;
            let pr = print as usize;
            let ns = next_state as usize;
            trans[st][sc] = Some((pr, shift, ns));
        }

        // Worklist fixed point: only revisit an exact local window when its
        // whole-side summary actually gains information. The previous
        // implementation rescanned every transition and every C^2 source
        // window after any merge anywhere in the lattice.
        let initial_index = SidePossible::<s, c>::index(0, 0, 0, 0);
        let mut queued = vec![false; possible.slot_count()];
        let mut q = VecDeque::new();
        queued[initial_index] = true;
        q.push_back((0_usize, 0_usize, 0_usize, 0_usize));

        while let Some((st, sc, left, right)) = q.pop_front() {
            let index =
                SidePossible::<s, c>::index(st, sc, left, right);
            queued[index] = false;

            let source = *possible.window_by_index(index);
            debug_assert!(source.reachable);

            let Some((pr, shift, ns)) = trans[st][sc] else {
                continue;
            };

            if shift {
                // Move R:
                //   (left, scan, right) -> (print, right, new_right)
                // The old right side knows which colors can follow its exact
                // nearest color `right`; intersect that with the target
                // 3-cell window relation.
                let mut new_rights = source.pairs[RIGHT_SIDE][right]
                    & win_possible.right[ns][right][pr];

                while new_rights != 0 {
                    let new_right =
                        new_rights.trailing_zeros() as usize;
                    new_rights &= new_rights - 1;

                    let target_index = SidePossible::<s, c>::index(
                        ns, right, pr, new_right,
                    );
                    let changed = merge(
                        &mut possible,
                        target_index,
                        source,
                        LEFT_SIDE,
                        pr,
                        left,
                        pr,
                        new_right,
                    );

                    if changed && !queued[target_index] {
                        queued[target_index] = true;
                        q.push_back((ns, right, pr, new_right));
                    }
                }
            } else {
                // Move L:
                //   (left, scan, right) -> (new_left, left, print)
                let mut new_lefts = source.pairs[LEFT_SIDE][left]
                    & win_possible.left[ns][left][pr];

                while new_lefts != 0 {
                    let new_left = new_lefts.trailing_zeros() as usize;
                    new_lefts &= new_lefts - 1;

                    let target_index = SidePossible::<s, c>::index(
                        ns, left, new_left, pr,
                    );
                    let changed = merge(
                        &mut possible,
                        target_index,
                        source,
                        RIGHT_SIDE,
                        pr,
                        right,
                        new_left,
                        pr,
                    );

                    if changed && !queued[target_index] {
                        queued[target_index] = true;
                        q.push_back((ns, left, new_left, pr));
                    }
                }
            }
        }

        possible
    }

    /// Compute ordered whole-side triples `(a,b,c)` for every exact reachable
    /// local window.  This is the length-3 companion to `SidePossible::pairs`.
    ///
    /// Existing source triples are copied conservatively across a head move.
    /// On the side the head moves away from, the newly written cell creates an
    /// exact new boundary prefix `print, old_neighbor`; every color already
    /// allowed to follow `old_neighbor` by the source pair summary therefore
    /// supplies a sound third cell.  Pulling from the opposite side needs no
    /// special deletion: retaining triples that involved the consumed nearest
    /// cell is an over-approximation and cannot make the backward proof unsound.
    fn side_triple_possible_from_blank(
        &self,
        windows: &WinPossible<s, c>,
        sides: &SidePossible<s, c>,
    ) -> SideTriplePossible<s, c> {
        let mut possible = SideTriplePossible::new();
        if !possible.enabled {
            return possible;
        }

        let mut trans = [[None; c]; s];
        for ((state, read), &(print, shift, next_state)) in self.iter()
        {
            trans[state as usize][read as usize] =
                Some((print as usize, shift, next_state as usize));
        }

        // On the true blank tape, both oriented sides contain 000 everywhere.
        for side in [LEFT_SIDE, RIGHT_SIDE] {
            let index = SideTriplePossible::<s, c>::index(
                0, 0, 0, 0, side, 0, 0,
            );
            possible.masks[index] |= 1;
        }

        let slot_count = s * c * c * c;
        let mut queued = vec![false; slot_count];
        let mut q = VecDeque::new();
        let initial =
            SideTriplePossible::<s, c>::window_index(0, 0, 0, 0);
        queued[initial] = true;
        q.push_back((0_usize, 0_usize, 0_usize, 0_usize));

        while let Some((st, scan, left, right)) = q.pop_front() {
            let source_window =
                SideTriplePossible::<s, c>::window_index(
                    st, scan, left, right,
                );
            queued[source_window] = false;

            let source_side = sides.window(st, scan, left, right);
            if !source_side.reachable {
                continue;
            }

            let Some((print, shift, next_state)) = trans[st][scan]
            else {
                continue;
            };

            let mut merge_target =
                |target_left: usize,
                 target_scan: usize,
                 target_right: usize,
                 push_side: usize,
                 old_neighbor: usize,
                 possible: &mut SideTriplePossible<s, c>| {
                    let target_window =
                        SideTriplePossible::<s, c>::window_index(
                            next_state,
                            target_scan,
                            target_left,
                            target_right,
                        );
                    let mut changed = false;

                    // Every triple wholly inside an old side remains a sound
                    // possible triple after the move.  Some copied triples on
                    // the pulled side may have involved its consumed nearest
                    // cell; keeping them is conservative.
                    for side in [LEFT_SIDE, RIGHT_SIDE] {
                        for near in 0..c {
                            for middle in 0..c {
                                let source_index =
                                    SideTriplePossible::<s, c>::index(
                                        st,
                                        scan,
                                        left,
                                        right,
                                        side,
                                        near,
                                        middle,
                                    );
                                let target_index =
                                    SideTriplePossible::<s, c>::index(
                                        next_state,
                                        target_scan,
                                        target_left,
                                        target_right,
                                        side,
                                        near,
                                        middle,
                                    );
                                let source_mask =
                                    possible.masks[source_index];
                                let old = possible.masks[target_index];
                                possible.masks[target_index] |= source_mask;
                                changed |= possible.masks[target_index] != old;
                            }
                        }
                    }

                    // The pushed side begins `print, old_neighbor, ...`.
                    // `SidePossible` already over-approximates every possible
                    // third color following that exact old neighbor.
                    let boundary =
                        source_side.pairs[push_side][old_neighbor];
                    let target_index = SideTriplePossible::<s, c>::index(
                        next_state,
                        target_scan,
                        target_left,
                        target_right,
                        push_side,
                        print,
                        old_neighbor,
                    );
                    let old = possible.masks[target_index];
                    possible.masks[target_index] |= boundary;
                    changed |= possible.masks[target_index] != old;

                    if changed && !queued[target_window] {
                        queued[target_window] = true;
                        q.push_back((
                            next_state,
                            target_scan,
                            target_left,
                            target_right,
                        ));
                    }
                };

            if shift {
                let mut new_rights = source_side.pairs[RIGHT_SIDE]
                    [right]
                    & windows.right[next_state][right][print];
                while new_rights != 0 {
                    let new_right =
                        new_rights.trailing_zeros() as usize;
                    new_rights &= new_rights - 1;
                    merge_target(
                        print,
                        right,
                        new_right,
                        LEFT_SIDE,
                        left,
                        &mut possible,
                    );
                }
            } else {
                let mut new_lefts = source_side.pairs[LEFT_SIDE][left]
                    & windows.left[next_state][left][print];
                while new_lefts != 0 {
                    let new_left = new_lefts.trailing_zeros() as usize;
                    new_lefts &= new_lefts - 1;
                    merge_target(
                        new_left,
                        left,
                        print,
                        RIGHT_SIDE,
                        right,
                        &mut possible,
                    );
                }
            }
        }

        possible
    }

    /// Compute cross-side co-occurrence of whole-side triples for every exact
    /// reachable local window. Existing joint pairs are copied across each
    /// head move. When a new boundary triple is created on the pushed side, it
    /// is paired only with triples projected from the opposite side of the same
    /// source joint relation, rather than with the global Cartesian product of
    /// the independent side summaries.
    fn joint_side_triple_possible_from_blank(
        &self,
        windows: &WinPossible<s, c>,
        sides: &SidePossible<s, c>,
    ) -> JointSideTriplePossible<s, c> {
        let mut possible = JointSideTriplePossible::new();
        if !possible.enabled {
            return possible;
        }

        let mut trans = [[None; c]; s];
        for ((state, read), &(print, shift, next_state)) in self.iter()
        {
            trans[state as usize][read as usize] =
                Some((print as usize, shift, next_state as usize));
        }

        let blank = JointSideTriplePossible::<s, c>::triple_id(0, 0, 0);
        possible.insert_ids(0, 0, 0, 0, blank, blank);

        let slot_count = s * c * c * c;
        let mut queued = vec![false; slot_count];
        let mut q = VecDeque::new();
        let initial =
            JointSideTriplePossible::<s, c>::window_index(0, 0, 0, 0);
        queued[initial] = true;
        q.push_back((0_usize, 0_usize, 0_usize, 0_usize));

        while let Some((st, scan, left, right)) = q.pop_front() {
            let source_window =
                JointSideTriplePossible::<s, c>::window_index(
                    st, scan, left, right,
                );
            queued[source_window] = false;

            let source_side = sides.window(st, scan, left, right);
            if !source_side.reachable {
                continue;
            }

            let Some((print, shift, next_state)) = trans[st][scan]
            else {
                continue;
            };

            // Capture these before mutating target windows. If a transition
            // loops back to the same exact window, newly added target pairs
            // must be discovered by a later worklist iteration, not folded
            // recursively into this edge.
            let (source_left_triples, source_right_triples) =
                possible.projections(st, scan, left, right);

            let mut merge_target =
                |target_left: usize,
                 target_scan: usize,
                 target_right: usize,
                 push_side: usize,
                 old_neighbor: usize,
                 possible: &mut JointSideTriplePossible<s, c>| {
                    let target_window =
                        JointSideTriplePossible::<s, c>::window_index(
                            next_state,
                            target_scan,
                            target_left,
                            target_right,
                        );
                    let mut changed = possible.copy_window(
                        st,
                        scan,
                        left,
                        right,
                        next_state,
                        target_scan,
                        target_left,
                        target_right,
                    );

                    let mut boundary =
                        source_side.pairs[push_side][old_neighbor];
                    while boundary != 0 {
                        let far = boundary.trailing_zeros() as usize;
                        boundary &= boundary - 1;
                        let new_id =
                            JointSideTriplePossible::<s, c>::triple_id(
                                print,
                                old_neighbor,
                                far,
                            );

                        if push_side == LEFT_SIDE {
                            let mut opposite = source_right_triples;
                            while opposite != 0 {
                                let right_id =
                                    opposite.trailing_zeros() as usize;
                                opposite &= opposite - 1;
                                changed |= possible.insert_ids(
                                    next_state,
                                    target_scan,
                                    target_left,
                                    target_right,
                                    new_id,
                                    right_id,
                                );
                            }
                        } else {
                            let mut opposite = source_left_triples;
                            while opposite != 0 {
                                let left_id =
                                    opposite.trailing_zeros() as usize;
                                opposite &= opposite - 1;
                                changed |= possible.insert_ids(
                                    next_state,
                                    target_scan,
                                    target_left,
                                    target_right,
                                    left_id,
                                    new_id,
                                );
                            }
                        }
                    }

                    if changed && !queued[target_window] {
                        queued[target_window] = true;
                        q.push_back((
                            next_state,
                            target_scan,
                            target_left,
                            target_right,
                        ));
                    }
                };

            if shift {
                let mut new_rights = source_side.pairs[RIGHT_SIDE]
                    [right]
                    & windows.right[next_state][right][print];
                while new_rights != 0 {
                    let new_right =
                        new_rights.trailing_zeros() as usize;
                    new_rights &= new_rights - 1;
                    merge_target(
                        print,
                        right,
                        new_right,
                        LEFT_SIDE,
                        left,
                        &mut possible,
                    );
                }
            } else {
                let mut new_lefts = source_side.pairs[LEFT_SIDE][left]
                    & windows.left[next_state][left][print];
                while new_lefts != 0 {
                    let new_left = new_lefts.trailing_zeros() as usize;
                    new_lefts &= new_lefts - 1;
                    merge_target(
                        new_left,
                        left,
                        print,
                        RIGHT_SIDE,
                        right,
                        &mut possible,
                    );
                }
            }
        }

        possible
    }

    /// Reachable two-run-plus-spill prefixes strictly beyond each immediate
    /// neighbor.
    ///
    /// The two sides are projected independently to keep the lattice small,
    /// but every alternative remains conditioned on the same exact
    /// `(state, left, scan, right)` window. Moving away from the tracked side
    /// prepends the old immediate neighbor; moving into it consumes one cell
    /// from the prefix. Once two complete runs have been retained, the next
    /// run is kept as a cheap color + 1/2+ spill before farther structure
    /// finally degrades to exact blank/dirty status.
    #[expect(clippy::cast_possible_truncation)]
    fn side_run_prefix_possible_from_blank(
        &self,
        windows: &WinPossible<s, c>,
        sides: &SidePossible<s, c>,
    ) -> (
        SidePrefixPossible<s, c>,
        [[Option<(usize, Shift, usize)>; c]; s],
        Vec<u64>,
    ) {
        let mut trans = [[None; c]; s];
        for ((state, read), &(print, shift, next_state)) in self.iter()
        {
            trans[state as usize][read as usize] =
                Some((print as usize, shift, next_state as usize));
        }

        // The newly exposed opposite-side neighbor depends only on the exact
        // source window, not on which prefix alternative is being propagated.
        // Cache that intersection once per window instead of repeating the
        // SidePossible/WinPossible lookup for every worklist node.
        let mut exposed = vec![0_u64; s * c * c * c];
        for st in 0..s {
            for scan in 0..c {
                let Some((print, shift, tr)) = trans[st][scan] else {
                    continue;
                };
                for left in 0..c {
                    for right in 0..c {
                        if windows.right[st][scan][left]
                            & (1_u64 << right)
                            == 0
                        {
                            continue;
                        }

                        let source =
                            sides.window(st, scan, left, right);
                        let index =
                            SidePrefixPossible::<s, c>::window_index(
                                st, scan, left, right,
                            );
                        exposed[index] = if shift {
                            source.pairs[RIGHT_SIDE][right]
                                & windows.right[tr][right][print]
                        } else {
                            source.pairs[LEFT_SIDE][left]
                                & windows.left[tr][left][print]
                        };
                    }
                }
            }
        }

        let mut possible = SidePrefixPossible::new_run_only();
        let mut q = VecDeque::new();
        let mut seen_specific: Set<(usize, SidePrefix)> = Set::new();

        let mut push =
            |side: usize,
             st: usize,
             scan: usize,
             left: usize,
             right: usize,
             prefix: SidePrefix,
             possible: &mut SidePrefixPossible<s, c>,
             q: &mut VecDeque<SidePrefixNode>| {
                if windows.right[st][scan][left] & (1_u64 << right) == 0
                {
                    return;
                }

                // Keep the independent projection at the previous precision
                // level. Fourth-run color is reserved for the same-witness
                // joint product, where it can actually add correlation.
                let prefix = prefix.widen_far_color();

                let index = SidePrefixPossible::<s, c>::index(
                    st, scan, left, right, side,
                );
                let flags = possible.flags[index];

                // Once both broad alternatives are present, this exact side is
                // universal and no later prefix can add information.
                if flags == SIDE_PREFIX_UNCONSTRAINED {
                    return;
                }

                let blank = prefix == SidePrefix::blank();
                let dirty_unknown =
                    prefix == SidePrefix::dirty_unknown();

                if blank {
                    if flags & SIDE_PREFIX_HAS_BLANK != 0 {
                        return;
                    }
                    possible.flags[index] |= SIDE_PREFIX_HAS_BLANK;
                } else if dirty_unknown {
                    if flags & SIDE_PREFIX_HAS_DIRTY_UNKNOWN != 0 {
                        return;
                    }

                    // `dirty_unknown()` subsumes every specific dirty prefix. The
                    // only incomparable alternative is exact blank, so discard all
                    // specifics in one pass and remember the broad state in a flag.
                    possible.flags[index] |=
                        SIDE_PREFIX_HAS_DIRTY_UNKNOWN;
                    possible.windows[index]
                        .retain(|&old| old == SidePrefix::blank());
                } else {
                    // Any specific nonblank prefix is already covered by the broad
                    // dirty alternative. Otherwise only exact deduplication is
                    // needed; there are no other subsumption relations.
                    if flags & SIDE_PREFIX_HAS_DIRTY_UNKNOWN != 0
                        || !seen_specific.insert((index, prefix))
                    {
                        return;
                    }
                }

                possible.windows[index].push(prefix);

                q.push_back(SidePrefixNode {
                    side,
                    st,
                    scan,
                    left,
                    right,
                    prefix,
                });
            };

        for side in [LEFT_SIDE, RIGHT_SIDE] {
            push(
                side,
                0,
                0,
                0,
                0,
                SidePrefix::blank(),
                &mut possible,
                &mut q,
            );
        }

        while let Some(node) = q.pop_front() {
            let SidePrefixNode {
                side,
                st,
                scan,
                left,
                right,
                prefix,
            } = node;

            // The only event that can remove a queued specific prefix is
            // arrival of `dirty_unknown()`. Test that cached flag in O(1)
            // instead of rescanning the antichain for every popped node.
            let index = SidePrefixPossible::<s, c>::index(
                st, scan, left, right, side,
            );
            if prefix != SidePrefix::blank()
                && prefix != SidePrefix::dirty_unknown()
                && possible.has_dirty_unknown_index(index)
            {
                continue;
            }

            let Some((print, shift, tr)) = trans[st][scan] else {
                continue;
            };

            match (side, shift) {
                // Track the left side while moving Right: the printed old head
                // becomes the new immediate neighbor and the old left neighbor
                // is prepended to the retained tail.
                (LEFT_SIDE, true) => {
                    let window =
                        SidePrefixPossible::<s, c>::window_index(
                            st, scan, left, right,
                        );
                    let new_rights = exposed[window];
                    prefix.for_each_prepend(
                        left as Color,
                        |next_prefix| {
                            let mut colors = new_rights;
                            while colors != 0 {
                                let new_right =
                                    colors.trailing_zeros() as usize;
                                colors &= colors - 1;
                                push(
                                    side,
                                    tr,
                                    right,
                                    print,
                                    new_right,
                                    next_prefix,
                                    &mut possible,
                                    &mut q,
                                );
                            }
                        },
                    );
                },

                // Track the left side while moving Left: old `left` becomes
                // scanned and one cell from its farther tail becomes the new
                // immediate left neighbor.
                (LEFT_SIDE, false) => {
                    prefix.for_each_pull::<c>(
                        |new_left, next_prefix| {
                            push(
                                side,
                                tr,
                                left,
                                usize::from(new_left),
                                print,
                                next_prefix,
                                &mut possible,
                                &mut q,
                            );
                        },
                    );
                },

                // Right-side symmetric cases.
                (RIGHT_SIDE, false) => {
                    let window =
                        SidePrefixPossible::<s, c>::window_index(
                            st, scan, left, right,
                        );
                    let new_lefts = exposed[window];
                    prefix.for_each_prepend(
                        right as Color,
                        |next_prefix| {
                            let mut colors = new_lefts;
                            while colors != 0 {
                                let new_left =
                                    colors.trailing_zeros() as usize;
                                colors &= colors - 1;
                                push(
                                    side,
                                    tr,
                                    left,
                                    new_left,
                                    print,
                                    next_prefix,
                                    &mut possible,
                                    &mut q,
                                );
                            }
                        },
                    );
                },
                (RIGHT_SIDE, true) => {
                    prefix.for_each_pull::<c>(
                        |new_right, next_prefix| {
                            push(
                                side,
                                tr,
                                right,
                                print,
                                usize::from(new_right),
                                next_prefix,
                                &mut possible,
                                &mut q,
                            );
                        },
                    );
                },
                _ => unreachable!(),
            }
        }

        (possible, trans, exposed)
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::excessive_nesting
    )]
    fn populate_side_word_prefix_possible_from_blank(
        windows: &WinPossible<s, c>,
        trans: &[[Option<(usize, Shift, usize)>; c]; s],
        exposed: &[u64],
        possible: &mut SidePrefixPossible<s, c>,
    ) {
        possible.init_word_storage();
        // A second worklist over the same exact windows retains ordered cell
        // prefixes and promotes repeated non-homogeneous words instead of
        // forgetting them after the run-prefix spill horizon. Keeping this
        // separate from the original run lattice preserves its cheap strong
        // homogeneous-run facts while adding periodic-order facts as a
        // conjunct.
        let mut word_q = VecDeque::new();

        let push_word =
            |side: usize,
             st: usize,
             scan: usize,
             left: usize,
             right: usize,
             prefix: SideWordPrefix,
             possible: &mut SidePrefixPossible<s, c>,
             q: &mut VecDeque<SideWordPrefixNode>| {
                if windows.right[st][scan][left] & (1_u64 << right) == 0
                {
                    return;
                }

                let index = SidePrefixPossible::<s, c>::index(
                    st, scan, left, right, side,
                );
                let flags = possible.word_flags[index];

                // A fully unknown side is the top element of this word
                // projection; once reached, no more-specific alternative can
                // strengthen that exact window/side.
                if flags & 0b100 != 0 || flags & 0b011 == 0b011 {
                    return;
                }

                if prefix.is_unconstrained() {
                    possible.word_flags[index] |= 0b100;
                    possible.word_windows[index].clear();
                } else if prefix.is_blank() {
                    if flags & 0b001 != 0 {
                        return;
                    }
                    possible.word_flags[index] |= 0b001;
                } else if prefix.is_dirty_unknown() {
                    if flags & 0b010 != 0 {
                        return;
                    }
                    possible.word_flags[index] |= 0b010;
                    possible.word_windows[index].retain(|old| {
                        old.is_blank() || !old.definitely_dirty()
                    });
                } else if flags & 0b010 != 0
                    && prefix.definitely_dirty()
                {
                    return;
                } else {
                    // The word lattice deliberately has no unbounded global
                    // `seen`: a 24-cell exact literal domain can otherwise
                    // explode combinatorially. Dedup in the small local
                    // antichain instead.
                    if possible.word_windows[index]
                        .iter()
                        .copied()
                        .any(|old| old.subsumes(prefix))
                    {
                        return;
                    }

                    possible.word_windows[index]
                        .retain(|old| !prefix.subsumes(*old));

                    if possible.word_windows[index].len()
                        >= SIDE_WORD_MAX_ALTS_PER_WINDOW
                    {
                        // If a broad blank/dirty alternative is already
                        // present, joining it with another specific prefix
                        // yields full unknown immediately. Otherwise retain
                        // the longest cell prefix guaranteed by every exact
                        // alternative. `from_literal` can promote that common
                        // prefix straight back to a periodic word.
                        let joined = if flags != 0 {
                            SideWordPrefix::unknown()
                        } else {
                            let mut cells =
                                [0; SIDE_WORD_LITERAL_CELLS];
                            let mut common = prefix
                                .guaranteed_len()
                                .min(SIDE_WORD_LITERAL_CELLS);

                            for old in &possible.word_windows[index] {
                                common =
                                    common.min(old.guaranteed_len());
                            }

                            let mut keep = 0_usize;
                            while keep < common {
                                let Some(color) =
                                    prefix.guaranteed_cell(keep)
                                else {
                                    break;
                                };
                                if possible.word_windows[index]
                                    .iter()
                                    .any(|old| {
                                        old.guaranteed_cell(keep)
                                            != Some(color)
                                    })
                                {
                                    break;
                                }
                                cells[keep] = color;
                                keep += 1;
                            }

                            if keep == 0 {
                                SideWordPrefix::unknown()
                            } else {
                                SideWordPrefix::from_literal(
                                    cells,
                                    keep,
                                    SideWordTail::Unknown,
                                )
                            }
                        };

                        possible.word_windows[index].clear();
                        if joined.is_unconstrained() {
                            possible.word_flags[index] |= 0b100;
                        }
                        possible.word_windows[index].push(joined);
                        q.push_back(SideWordPrefixNode {
                            side,
                            st,
                            scan,
                            left,
                            right,
                            prefix: joined,
                        });
                        return;
                    }
                }

                possible.word_windows[index].push(prefix);
                q.push_back(SideWordPrefixNode {
                    side,
                    st,
                    scan,
                    left,
                    right,
                    prefix,
                });
            };

        for side in [LEFT_SIDE, RIGHT_SIDE] {
            push_word(
                side,
                0,
                0,
                0,
                0,
                SideWordPrefix::blank(),
                possible,
                &mut word_q,
            );
        }

        while let Some(node) = word_q.pop_front() {
            let SideWordPrefixNode {
                side,
                st,
                scan,
                left,
                right,
                prefix,
            } = node;

            let index = SidePrefixPossible::<s, c>::index(
                st, scan, left, right, side,
            );

            // A local join can remove many already-queued exact literals.
            // Do not keep propagating those stale nodes after their window has
            // been replaced by a broader common-prefix representative.
            if !possible.word_windows[index].contains(&prefix) {
                continue;
            }

            if !prefix.is_unconstrained()
                && possible.word_has_unknown_index(index)
            {
                continue;
            }
            if !prefix.is_blank()
                && !prefix.is_dirty_unknown()
                && prefix.definitely_dirty()
                && possible.word_flags[index] & 0b010 != 0
            {
                continue;
            }

            let Some((print, shift, tr)) = trans[st][scan] else {
                continue;
            };

            match (side, shift) {
                (LEFT_SIDE, true) => {
                    let window =
                        SidePrefixPossible::<s, c>::window_index(
                            st, scan, left, right,
                        );
                    let new_rights = exposed[window];
                    let next_prefix = prefix.prepend(left as Color);
                    let mut colors = new_rights;
                    while colors != 0 {
                        let new_right =
                            colors.trailing_zeros() as usize;
                        colors &= colors - 1;
                        push_word(
                            side,
                            tr,
                            right,
                            print,
                            new_right,
                            next_prefix,
                            possible,
                            &mut word_q,
                        );
                    }
                },
                (LEFT_SIDE, false) => {
                    prefix.for_each_pull::<c>(
                        |new_left, next_prefix| {
                            push_word(
                                side,
                                tr,
                                left,
                                usize::from(new_left),
                                print,
                                next_prefix,
                                possible,
                                &mut word_q,
                            );
                        },
                    );
                },
                (RIGHT_SIDE, false) => {
                    let window =
                        SidePrefixPossible::<s, c>::window_index(
                            st, scan, left, right,
                        );
                    let new_lefts = exposed[window];
                    let next_prefix = prefix.prepend(right as Color);
                    let mut colors = new_lefts;
                    while colors != 0 {
                        let new_left = colors.trailing_zeros() as usize;
                        colors &= colors - 1;
                        push_word(
                            side,
                            tr,
                            left,
                            new_left,
                            print,
                            next_prefix,
                            possible,
                            &mut word_q,
                        );
                    }
                },
                (RIGHT_SIDE, true) => {
                    prefix.for_each_pull::<c>(
                        |new_right, next_prefix| {
                            push_word(
                                side,
                                tr,
                                right,
                                print,
                                usize::from(new_right),
                                next_prefix,
                                possible,
                                &mut word_q,
                            );
                        },
                    );
                },
                _ => unreachable!(),
            }
        }
    }
}

#[expect(clippy::multiple_inherent_impl)]
impl<const S: usize, const C: usize> Prog<S, C> {
    #[expect(clippy::cast_possible_truncation)]
    fn joint_short_possible_from_blank(
        &self,
        windows: &WinPossible<S, C>,
        radius2: &Radius2Possible<S, C>,
    ) -> JointShortPossible<S, C> {
        let mut trans = [[None; C]; S];
        for ((state, read), &(print, shift, next_state)) in self.iter()
        {
            trans[state as usize][read as usize] =
                Some((print as usize, shift, next_state as usize));
        }

        let mut possible = JointShortPossible::new();
        let mut q = VecDeque::new();
        let mut seen: Set<JointShortNode> = Set::new();

        let push =
            |node: JointShortNode,
             possible: &mut JointShortPossible<S, C>,
             seen: &mut Set<JointShortNode>,
             q: &mut VecDeque<JointShortNode>| {
                if windows.right[node.st][node.scan][node.left]
                    & (1_u64 << node.right)
                    == 0
                    || !radius2.joint_short_compatible(
                        node.st,
                        node.scan,
                        node.left,
                        node.right,
                        node.prefix,
                    )
                {
                    return;
                }
                if !seen.insert(node) {
                    return;
                }

                let index = JointShortPossible::<S, C>::index(
                    node.st, node.scan, node.left, node.right,
                );
                possible.windows[index].push(node.prefix);
                q.push_back(node);
            };

        push(
            JointShortNode {
                st: 0,
                scan: 0,
                left: 0,
                right: 0,
                prefix: JointShortPrefix::blank(),
            },
            &mut possible,
            &mut seen,
            &mut q,
        );

        while let Some(node) = q.pop_front() {
            let Some((print, shift, tr)) = trans[node.st][node.scan]
            else {
                continue;
            };

            if shift {
                let new_left_tail =
                    node.prefix.left.prepend(node.left as Color);
                node.prefix.right.for_each_pull::<C>(
                    |new_right, new_right_tail| {
                        push(
                            JointShortNode {
                                st: tr,
                                scan: node.right,
                                left: print,
                                right: usize::from(new_right),
                                prefix: JointShortPrefix {
                                    left: new_left_tail,
                                    right: new_right_tail,
                                },
                            },
                            &mut possible,
                            &mut seen,
                            &mut q,
                        );
                    },
                );
            } else {
                let new_right_tail =
                    node.prefix.right.prepend(node.right as Color);
                node.prefix.left.for_each_pull::<C>(
                    |new_left, new_left_tail| {
                        push(
                            JointShortNode {
                                st: tr,
                                scan: node.left,
                                left: usize::from(new_left),
                                right: print,
                                prefix: JointShortPrefix {
                                    left: new_left_tail,
                                    right: new_right_tail,
                                },
                            },
                            &mut possible,
                            &mut seen,
                            &mut q,
                        );
                    },
                );
            }
        }

        possible
    }

    #[expect(clippy::cast_possible_truncation)]
    fn joint_side_prefix_possible_from_blank(
        &self,
        windows: &WinPossible<S, C>,
        track_parity: bool,
    ) -> JointSidePrefixPossible<S, C> {
        let mut trans = [[None; C]; S];
        for ((state, read), &(print, shift, next_state)) in self.iter()
        {
            trans[state as usize][read as usize] =
                Some((print as usize, shift, next_state as usize));
        }

        let mut possible = JointSidePrefixPossible::new(track_parity);
        let mut q = VecDeque::new();

        let push =
            |mut node: JointSidePrefixNode,
             possible: &mut JointSidePrefixPossible<S, C>,
             q: &mut VecDeque<JointSidePrefixNode>| {
                if windows.right[node.st][node.scan][node.left]
                    & (1_u64 << node.right)
                    == 0
                {
                    return;
                }

                let index = JointSidePrefixPossible::<S, C>::index(
                    node.st, node.scan, node.left, node.right,
                );

                if possible.far_color_widened[index] {
                    node.prefix = node.prefix.widen_far_colors();
                }
                if possible.spill_widened[index] {
                    node.prefix = node.prefix.widen_spill_counts();
                }

                // A bucket that crossed the shape-antichain cap stays widened
                // forever, but its surviving whole-side parity relation can
                // still grow as more forward paths reach the same window.
                if possible.overflowed[index] {
                    let alts = &mut possible.windows[index];
                    debug_assert_eq!(alts.len(), 1);
                    let old_mask = alts[0].parity_mask();
                    let new_mask = old_mask | node.prefix.parity_mask();
                    if new_mask == old_mask {
                        return;
                    }

                    let widened = JointSidePrefix::Unknown {
                        parity_mask: new_mask,
                    };
                    alts[0] = widened;
                    q.push_back(JointSidePrefixNode {
                        prefix: widened,
                        ..node
                    });
                    return;
                }

                let alts = &mut possible.windows[index];
                let Some(inserted) =
                    insert_joint_side_prefix_alt(alts, node.prefix)
                else {
                    return;
                };
                node.prefix = inserted;

                if alts.len() <= JOINT_SIDE_PREFIX_MAX_ALTS_PER_WINDOW {
                    q.push_back(node);
                    return;
                }

                if !possible.far_color_widened[index] {
                    // Ordered fourth-run color is deliberately the cheapest
                    // precision layer. Drop its order first while retaining the
                    // color in the merged far-tail mask.
                    widen_joint_side_prefix_alts_in_place(
                        alts,
                        JointSidePrefix::widen_far_colors,
                    );
                    possible.far_color_widened[index] = true;

                    if alts.len()
                        <= JOINT_SIDE_PREFIX_MAX_ALTS_PER_WINDOW
                    {
                        for &prefix in alts.iter() {
                            q.push_back(JointSidePrefixNode {
                                prefix,
                                ..node
                            });
                        }
                        return;
                    }
                }

                if !possible.spill_widened[index] {
                    // If fourth-color widening was insufficient, recover the
                    // old 1/2+ spill-count quotient before giving up all shape.
                    widen_joint_side_prefix_alts_in_place(
                        alts,
                        JointSidePrefix::widen_spill_counts,
                    );
                    possible.spill_widened[index] = true;

                    if alts.len()
                        <= JOINT_SIDE_PREFIX_MAX_ALTS_PER_WINDOW
                    {
                        for &prefix in alts.iter() {
                            q.push_back(JointSidePrefixNode {
                                prefix,
                                ..node
                            });
                        }
                        return;
                    }
                }

                let parity_mask = if possible.track_parity {
                    alts.iter().copied().fold(0_u8, |mask, alt| {
                        mask | alt.parity_mask()
                    })
                } else {
                    0b1111
                };

                let widened = JointSidePrefix::Unknown { parity_mask };
                alts.clear();
                alts.push(widened);
                possible.overflowed[index] = true;
                q.push_back(JointSidePrefixNode {
                    prefix: widened,
                    ..node
                });
            };

        push(
            JointSidePrefixNode {
                st: 0,
                scan: 0,
                left: 0,
                right: 0,
                prefix: JointSidePrefix::blank(track_parity),
            },
            &mut possible,
            &mut q,
        );

        while let Some(node) = q.pop_front() {
            let index = JointSidePrefixPossible::<S, C>::index(
                node.st, node.scan, node.left, node.right,
            );
            if !possible.windows[index].contains(&node.prefix) {
                continue;
            }

            let Some((print, shift, tr)) = trans[node.st][node.scan]
            else {
                continue;
            };

            let next_parity = if possible.track_parity {
                advance_joint_side_parity(
                    node.prefix.parity_mask(),
                    shift,
                    print,
                    node.left,
                    node.right,
                )
            } else {
                0b1111
            };

            match node.prefix {
                JointSidePrefix::Unknown { .. } => {
                    if shift {
                        let mut rights =
                            windows.right[tr][node.right][print];
                        while rights != 0 {
                            let new_right =
                                rights.trailing_zeros() as usize;
                            rights &= rights - 1;
                            push(
                                JointSidePrefixNode {
                                    st: tr,
                                    scan: node.right,
                                    left: print,
                                    right: new_right,
                                    prefix: JointSidePrefix::Unknown {
                                        parity_mask: next_parity,
                                    },
                                },
                                &mut possible,
                                &mut q,
                            );
                        }
                    } else {
                        let mut lefts =
                            windows.left[tr][node.left][print];
                        while lefts != 0 {
                            let new_left =
                                lefts.trailing_zeros() as usize;
                            lefts &= lefts - 1;
                            push(
                                JointSidePrefixNode {
                                    st: tr,
                                    scan: node.left,
                                    left: new_left,
                                    right: print,
                                    prefix: JointSidePrefix::Unknown {
                                        parity_mask: next_parity,
                                    },
                                },
                                &mut possible,
                                &mut q,
                            );
                        }
                    }
                },
                JointSidePrefix::Specific {
                    left,
                    right,
                    far_colors,
                    ..
                } => {
                    if shift {
                        // Prepending has a tiny fixed fanout (at most ten for a
                        // DirtyUnknown tail). Keep both shape and its updated
                        // far-color metadata on the stack instead of allocating.
                        let mut left_next = [SidePrefix::blank();
                            SIDE_PREFIX_MAX_PREPEND_ALTS];
                        let mut left_far_next =
                            [0_u64; SIDE_PREFIX_MAX_PREPEND_ALTS];
                        let mut left_len = 0_usize;
                        left.for_each_prepend(
                            node.left as Color,
                            |prefix| {
                                debug_assert!(
                                    left_len
                                        < SIDE_PREFIX_MAX_PREPEND_ALTS
                                );
                                left_next[left_len] = prefix;
                                left_far_next[left_len] = left
                                    .far_colors_after_prepend(
                                        node.left as Color,
                                        prefix,
                                        far_colors[LEFT_SIDE],
                                    );
                                left_len += 1;
                            },
                        );
                        right.for_each_pull::<C>(|new_right, right_next| {
                            if !right.unknown_pull_color_possible(
                                far_colors[RIGHT_SIDE],
                                new_right,
                            ) {
                                return;
                            }
                            if windows.right[tr][node.right][print]
                                & (1_u64 << usize::from(new_right))
                                == 0
                            {
                                return;
                            }
                            let right_far = right.far_colors_after_pull(
                                right_next,
                                far_colors[RIGHT_SIDE],
                            );
                            for (&left_next, &left_far) in left_next[..left_len]
                                .iter()
                                .zip(&left_far_next[..left_len])
                            {
                                push(
                                    JointSidePrefixNode {
                                        st: tr,
                                        scan: node.right,
                                        left: print,
                                        right: usize::from(new_right),
                                        prefix: JointSidePrefix::Specific {
                                            left: left_next,
                                            right: right_next,
                                            parity_mask: next_parity,
                                            far_colors: [left_far, right_far],
                                        },
                                    },
                                    &mut possible,
                                    &mut q,
                                );
                            }
                        });
                    } else {
                        let mut right_next = [SidePrefix::blank();
                            SIDE_PREFIX_MAX_PREPEND_ALTS];
                        let mut right_far_next =
                            [0_u64; SIDE_PREFIX_MAX_PREPEND_ALTS];
                        let mut right_len = 0_usize;
                        right.for_each_prepend(
                            node.right as Color,
                            |prefix| {
                                debug_assert!(
                                    right_len
                                        < SIDE_PREFIX_MAX_PREPEND_ALTS
                                );
                                right_next[right_len] = prefix;
                                right_far_next[right_len] = right
                                    .far_colors_after_prepend(
                                        node.right as Color,
                                        prefix,
                                        far_colors[RIGHT_SIDE],
                                    );
                                right_len += 1;
                            },
                        );
                        left.for_each_pull::<C>(|new_left, left_next| {
                            if !left.unknown_pull_color_possible(
                                far_colors[LEFT_SIDE],
                                new_left,
                            ) {
                                return;
                            }
                            if windows.left[tr][node.left][print]
                                & (1_u64 << usize::from(new_left))
                                == 0
                            {
                                return;
                            }
                            let left_far = left.far_colors_after_pull(
                                left_next,
                                far_colors[LEFT_SIDE],
                            );
                            for (&right_next, &right_far) in right_next[..right_len]
                                .iter()
                                .zip(&right_far_next[..right_len])
                            {
                                push(
                                    JointSidePrefixNode {
                                        st: tr,
                                        scan: node.left,
                                        left: usize::from(new_left),
                                        right: print,
                                        prefix: JointSidePrefix::Specific {
                                            left: left_next,
                                            right: right_next,
                                            parity_mask: next_parity,
                                            far_colors: [left_far, right_far],
                                        },
                                    },
                                    &mut possible,
                                    &mut q,
                                );
                            }
                        });
                    }
                },
            }
        }

        possible
    }

    #[expect(clippy::cast_possible_truncation)]
    fn joint_side_word_prefix_possible_from_blank(
        &self,
        windows: &WinPossible<S, C>,
    ) -> JointSideWordPrefixPossible<S, C> {
        let mut trans = [[None; C]; S];
        for ((state, read), &(print, shift, next_state)) in self.iter()
        {
            trans[state as usize][read as usize] =
                Some((print as usize, shift, next_state as usize));
        }

        let mut possible = JointSideWordPrefixPossible::new();
        let mut q = VecDeque::new();

        let push =
            |node: JointSideWordPrefixNode,
             possible: &mut JointSideWordPrefixPossible<S, C>,
             q: &mut VecDeque<JointSideWordPrefixNode>| {
                if windows.right[node.st][node.scan][node.left]
                    & (1_u64 << node.right)
                    == 0
                {
                    return;
                }

                let index = JointSideWordPrefixPossible::<S, C>::index(
                    node.st, node.scan, node.left, node.right,
                );
                let alts = &mut possible.windows[index];

                if alts
                    .iter()
                    .copied()
                    .any(|old| old.subsumes(node.prefix))
                {
                    return;
                }
                alts.retain(|&old| !node.prefix.subsumes(old));

                if node.prefix != JointSideWordPrefix::Unknown
                    && alts.len() >= JOINT_SIDE_WORD_MAX_ALTS_PER_WINDOW
                {
                    alts.clear();
                    alts.push(JointSideWordPrefix::Unknown);
                    q.push_back(JointSideWordPrefixNode {
                        prefix: JointSideWordPrefix::Unknown,
                        ..node
                    });
                    return;
                }

                alts.push(node.prefix);
                q.push_back(node);
            };

        push(
            JointSideWordPrefixNode {
                st: 0,
                scan: 0,
                left: 0,
                right: 0,
                prefix: JointSideWordPrefix::blank(),
            },
            &mut possible,
            &mut q,
        );

        while let Some(node) = q.pop_front() {
            let index = JointSideWordPrefixPossible::<S, C>::index(
                node.st, node.scan, node.left, node.right,
            );
            if !possible.windows[index].contains(&node.prefix) {
                continue;
            }

            let Some((print, shift, tr)) = trans[node.st][node.scan]
            else {
                continue;
            };

            match node.prefix {
                JointSideWordPrefix::Unknown => {
                    if shift {
                        let mut rights =
                            windows.right[tr][node.right][print];
                        while rights != 0 {
                            let new_right =
                                rights.trailing_zeros() as usize;
                            rights &= rights - 1;
                            push(
                                JointSideWordPrefixNode {
                                    st: tr,
                                    scan: node.right,
                                    left: print,
                                    right: new_right,
                                    prefix:
                                        JointSideWordPrefix::Unknown,
                                },
                                &mut possible,
                                &mut q,
                            );
                        }
                    } else {
                        let mut lefts =
                            windows.left[tr][node.left][print];
                        while lefts != 0 {
                            let new_left =
                                lefts.trailing_zeros() as usize;
                            lefts &= lefts - 1;
                            push(
                                JointSideWordPrefixNode {
                                    st: tr,
                                    scan: node.left,
                                    left: new_left,
                                    right: print,
                                    prefix:
                                        JointSideWordPrefix::Unknown,
                                },
                                &mut possible,
                                &mut q,
                            );
                        }
                    }
                },
                JointSideWordPrefix::Specific { left, right } => {
                    if shift {
                        let left_next =
                            left.prepend(node.left as Color);
                        right.for_each_pull::<C>(|new_right, right_next| {
                            if windows.right[tr][node.right][print]
                                & (1_u64 << usize::from(new_right))
                                == 0
                            {
                                return;
                            }
                            push(
                                JointSideWordPrefixNode {
                                    st: tr,
                                    scan: node.right,
                                    left: print,
                                    right: usize::from(new_right),
                                    prefix: JointSideWordPrefix::Specific {
                                        left: left_next,
                                        right: right_next,
                                    },
                                },
                                &mut possible,
                                &mut q,
                            );
                        });
                    } else {
                        let right_next =
                            right.prepend(node.right as Color);
                        left.for_each_pull::<C>(|new_left, left_next| {
                            if windows.left[tr][node.left][print]
                                & (1_u64 << usize::from(new_left))
                                == 0
                            {
                                return;
                            }
                            push(
                                JointSideWordPrefixNode {
                                    st: tr,
                                    scan: node.left,
                                    left: usize::from(new_left),
                                    right: print,
                                    prefix: JointSideWordPrefix::Specific {
                                        left: left_next,
                                        right: right_next,
                                    },
                                },
                                &mut possible,
                                &mut q,
                            );
                        });
                    }
                },
            }
        }

        possible
    }
}

#[cfg(test)]
use crate::instrs::{read_color, read_shift, read_state};

#[cfg(test)]
fn read_entry(entry: &str) -> Entry {
    let (slot, instr) = entry.split_once(':').unwrap();

    let mut chars = instr.chars();
    let color = chars.next().unwrap();
    let shift = chars.next().unwrap();

    (Slot::read(slot), (read_color(color), read_shift(shift)))
}

#[cfg(test)]
macro_rules! assert_entrypoints {
    ($(($prog:literal, ($s:literal, $c:literal)) => [$($state:literal => ($same:expr, $diff:expr)),* $(,)?]),* $(,)?) => {
        $({
            let mut entrypoints = Entrypoints::new();

            $(
                entrypoints.insert(
                    read_state($state),
                    (
                        $same.into_iter().map(read_entry).collect(),
                        $diff.into_iter().map(read_entry).collect(),
                    ),
                );
            )*

            assert_eq!(
                entrypoints,
                Prog::<$s, $c>::from($prog).get_entrypoints(),
            );
        })*
    };
}

#[test]
fn test_entrypoints() {
    assert_entrypoints!(
        ("1RB ...  1LB 0RB", (2, 2)) => [
            'B' => (["B0:1L", "B1:0R"], ["A0:1RB"])
        ],
        ("1RB ... ...  0LB 2RB 0RB", (2, 3)) => [
            'B' => (["B0:0L", "B1:2R", "B2:0R"], ["A0:1RB"])
        ],
        ("1RB ... 2LB  2LB 2RA 0RA", (2, 3)) => [
            'A' => ([], ["B1:2R", "B2:0R"]),
            'B' => (["B0:2L"], ["A0:1R", "A2:2L"])
        ],
        ("1RB 0RB 1RA  1LB 2RB 0LA", (2, 3)) => [
            'A' => (["A2:1R"], ["B2:0L"]),
            'B' => (["B0:1L", "B1:2R"], ["A0:1R", "A1:0R"])
        ],
        ("1RB 1RC  0LA 1RA  0LB ...", (3, 2)) => [
            'A' => ([], ["B0:0L", "B1:1R"]),
            'B' => ([], ["A0:1R", "C0:0L"]),
            'C' => ([], ["A1:1R"])
        ],
        ("1RB ...  0LB 1RC  0LC 1RA", (3, 2)) => [
            'A' => ([], ["C1:1R"]),
            'B' => (["B0:0L"], ["A0:1R"]),
            'C' => (["C0:0L"], ["B1:1R"])
        ],
        ("1RB 1LB  1LA 1LC  1RC 0LC", (3, 2)) => [
            'A' => ([], ["B0:1L"]),
            'B' => ([], ["A0:1R", "A1:1L"]),
            'C' => (["C0:1R", "C1:0L"], ["B1:1L"])
        ],
        ("1RB 0LC  1LB 1LA  1RC 0LC", (3, 2)) => [
            'A' => ([], ["B1:1L"]),
            'B' => (["B0:1L"], ["A0:1R"]),
            'C' => (["C0:1R", "C1:0L"], ["A1:0L"])
        ],
        ("1RB 2RA 0RB 2RB  1LB 3RB 3LA 0LA", (2, 4)) => [
            'A' => (["A1:2R"], ["B2:3L", "B3:0L"]),
            'B' => (["B0:1L", "B1:3R"], ["A0:1R", "A2:0R", "A3:2R"])
        ],
        ("1RB ...  0LC ...  1RC 1LD  0LC 0LD", (4, 2)) => [
            'B' => ([], ["A0:1RB"]),
            'C' => (["C0:1R"], ["B0:0L", "D0:0L"]),
            'D' => (["D1:0L"], ["C1:1L"])
        ],
        ("1RB ...  0LC ...  1RC 1LD  0LC 0LB", (4, 2)) => [
            'B' => ([], ["A0:1RB", "D1:0L"]),
            'C' => (["C0:1R"], ["B0:0L", "D0:0L"]),
            'D' => ([], ["C1:1L"])
        ],
        ("1RB 1LC  1RD 1RB  0RD 0RC  1LD 1LA", (4, 2)) => [
            'A' => ([], ["D1:1L"]),
            'B' => (["B1:1R"], ["A0:1R"]),
            'C' => (["C1:0R"], ["A1:1L"]),
            'D' => (["D0:1L"], ["B0:1R", "C0:0R"])
        ],
        ("1RB 1LC  0LC 0RD  1RD 1LE  1RE 1LA  1LA 0LB", (5, 2)) => [
            'A' => ([], ["D1:1L", "E0:1L"]),
            'B' => ([], ["A0:1R", "E1:0L"]),
            'C' => ([], ["A1:1L", "B0:0L"]),
            'D' => ([], ["B1:0R", "C0:1R"]),
            'E' => ([], ["C1:1L", "D0:1R"])
        ],
    );
}

/**************************************/

#[derive(Clone)]
struct Config {
    state: State,
    tape: Tape,
}

impl Config {
    const fn new(state: State, tape: Tape) -> Self {
        Self { state, tape }
    }

    const fn init_halt(state: State, color: Color) -> Self {
        Self::new(state, Tape::init_halt(color))
    }

    const fn init_blank(state: State, color: Color) -> Self {
        Self::new(state, Tape::init_blank(color))
    }

    const fn init_spinout(state: State, shift: Shift) -> Self {
        Self::new(state, Tape::init_spinout(shift))
    }

    fn init_twostep(state: State, l_co: Color, r_co: Color) -> Self {
        Self::new(state, Tape::init_twostep(l_co, r_co))
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let tape = &self.tape;
        let slot = (self.state, tape.scan).show();

        write!(f, "{slot} | {tape}")
    }
}

/**************************************/

// Multi-step repeated-word widening.  Unlike the single-transition spinout
// path, this recognizes a recurring whole configuration skeleton whose one
// compound block grows by a stable number of copies every stable number of
// backward steps.  The recurrence is only a trigger: replacing one exact
// `(word)^k` configuration by `(word)^m..` for m <= k is itself a sound
// over-approximation, so a mistaken recurrence guess cannot create a false
// refutation.
const WORD_WIDEN_OBSERVATIONS: usize = 3;
const WORD_WIDEN_KEEP: usize = 12;

#[derive(Clone, PartialEq, Eq, Hash)]
struct WordGrowthKey {
    state: State,
    scan: Color,
    side: Side,
    position: usize,
    l_end: EndSig,
    r_end: EndSig,
    left: Vec<BlockSig>,
    right: Vec<BlockSig>,
}

#[derive(Default)]
struct WordGrowthObservations {
    samples: Vec<(Steps, usize)>,
    widened_min: Option<usize>,
}

#[derive(Default)]
struct WordWideningHistory {
    entries: Dict<WordGrowthKey, WordGrowthObservations>,
}

impl WordWideningHistory {
    fn skeleton(span: &Span) -> Vec<BlockSig> {
        let mut sig = span_runs(span);
        for block in &mut sig {
            if let BlockSig::Word { count, indef, .. } = block
                && !*indef
            {
                *count = 0;
            }
        }
        sig
    }

    fn threshold(
        &mut self,
        key: WordGrowthKey,
        step: Steps,
        count: usize,
    ) -> Option<usize> {
        let entry = self.entries.entry(key).or_default();
        if let Some(minimum) = entry.widened_min {
            return (count >= minimum).then_some(minimum);
        }

        if let Some(last) = entry.samples.last_mut()
            && last.0 == step
        {
            if count <= last.1 {
                return None;
            }
            last.1 = count;
        } else {
            if entry
                .samples
                .last()
                .is_some_and(|&(_, old)| old == count)
            {
                return None;
            }
            entry.samples.push((step, count));
            if entry.samples.len() > WORD_WIDEN_KEEP {
                entry.samples.remove(0);
            }
        }

        if entry.samples.len() < WORD_WIDEN_OBSERVATIONS {
            return None;
        }

        // Use the newest point and look through recent history for two points
        // at the same positive (step,count) spacing. This tolerates unrelated
        // branches interleaved between observations of the growing macro-edge.
        let (now_step, now_count) =
            entry.samples[entry.samples.len() - 1];
        for &(prev_step, prev_count) in entry.samples
            [..entry.samples.len() - 1]
            .iter()
            .rev()
            .take(8)
        {
            if prev_step >= now_step || prev_count >= now_count {
                continue;
            }

            let period = now_step - prev_step;
            let delta = now_count - prev_count;
            let Some(first_step) = prev_step.checked_sub(period) else {
                continue;
            };
            let Some(first_count) = prev_count.checked_sub(delta)
            else {
                continue;
            };
            if first_count == 0 {
                continue;
            }

            if entry.samples.contains(&(first_step, first_count)) {
                entry.widened_min = Some(first_count);
                return Some(first_count);
            }
        }

        None
    }

    fn widen(&mut self, config: &mut Config, step: Steps) -> bool {
        // Keep the common non-periodic path allocation-free. Only after an
        // exact compound block exists do we build the structural signatures.
        let mut candidates = Vec::new();
        for (side, span) in [
            (Side::Left, &config.tape.lspan),
            (Side::Right, &config.tape.rspan),
        ] {
            for (position, block) in span.span.iter().enumerate() {
                let Block::Word {
                    word,
                    count: WordCount::Exact(count),
                } = block
                else {
                    continue;
                };
                if word.len() > 1 {
                    candidates.push((side, position, *count));
                }
            }
        }

        if candidates.is_empty() {
            return false;
        }

        let left = Self::skeleton(&config.tape.lspan);
        let right = Self::skeleton(&config.tape.rspan);
        let l_end = EndSig::from_end(&config.tape.lspan.end);
        let r_end = EndSig::from_end(&config.tape.rspan.end);

        let mut changed = false;
        for (side, position, count) in candidates {
            let key = WordGrowthKey {
                state: config.state,
                scan: config.tape.scan,
                side,
                position,
                l_end,
                r_end,
                left: left.clone(),
                right: right.clone(),
            };

            let Some(minimum) = self.threshold(key, step, count) else {
                continue;
            };

            let span = if side == Side::Left {
                &mut config.tape.lspan
            } else {
                &mut config.tape.rspan
            };
            let physical = span.span.len() - 1 - position;
            let Some(Block::Word { count, .. }) =
                span.span.blocks.get_mut(physical)
            else {
                continue;
            };

            if matches!(count, WordCount::Exact(exact) if *exact >= minimum)
            {
                *count = WordCount::AtLeast(minimum);
                changed = true;
            }
        }

        if changed {
            config.tape.lspan.span.normalize_boundary();
            config.tape.rspan.span.normalize_boundary();
        }
        changed
    }
}

// Multi-step widening for a recurring monochromatic run.  This is the
// single-color counterpart of WordWideningHistory, but it retains the observed
// count stride instead of widening to an arbitrary lower bound.  For example,
// exact counts 1,3,5 become {1+2k}; 2,4,6 become {2+2k}.
//
// As with word widening, recurrence detection is only a trigger.  Replacing an
// exact count by any arithmetic progression containing that count is a sound
// over-approximation, so an accidental recurrence match can only weaken later
// pruning, never create a false refutation.
const RUN_SPINE_WIDEN_OBSERVATIONS: usize = 3;
const RUN_SPINE_WIDEN_KEEP: usize = 16;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct RunSpineGrowthKey {
    state: State,
    scan: Color,
    side: Side,
    skeleton: u64,
}

#[derive(Default)]
struct RunSpineGrowthObservations {
    samples: Vec<(Steps, Count)>,
    widened: Option<(Count, Count)>, // (minimum, stride)
}

#[derive(Default)]
struct RunSpineWideningHistory {
    entries: Dict<RunSpineGrowthKey, RunSpineGrowthObservations>,
}

impl RunSpineWideningHistory {
    fn key(config: &Config, side: Side) -> RunSpineGrowthKey {
        fn hash_span(
            span: &Span,
            wildcard_near_run: bool,
            h: &mut AHasher,
        ) {
            span.end.hash(h);
            span.span.len().hash(h);
            for (position, block) in span.span.iter().enumerate() {
                if wildcard_near_run && position == 0 {
                    let Block::Run { color, .. } = block else {
                        unreachable!(
                            "run-spine candidate must be a run"
                        )
                    };
                    // A distinct tag plus the run color preserves the complete
                    // skeleton while forgetting only this run's count/domain.
                    0xA5_u8.hash(h);
                    color.hash(h);
                } else {
                    block.hash(h);
                }
            }
        }

        let mut h = AHasher::default();
        match side {
            Side::Left => {
                hash_span(&config.tape.lspan, true, &mut h);
                hash_span(&config.tape.rspan, false, &mut h);
            },
            Side::Right => {
                hash_span(&config.tape.lspan, false, &mut h);
                hash_span(&config.tape.rspan, true, &mut h);
            },
        }

        RunSpineGrowthKey {
            state: config.state,
            scan: config.tape.scan,
            side,
            skeleton: h.finish(),
        }
    }

    fn threshold(
        &mut self,
        key: RunSpineGrowthKey,
        step: Steps,
        count: BlockCount,
    ) -> Option<(Count, Count)> {
        let entry = self.entries.entry(key).or_default();

        if let Some((minimum, stride)) = entry.widened {
            return count
                .covered_by_stride(minimum, stride)
                .then_some((minimum, stride));
        }

        let BlockCount::Exact(count) = count else {
            return None;
        };

        if let Some(last) = entry.samples.last_mut()
            && last.0 == step
        {
            if count <= last.1 {
                return None;
            }
            last.1 = count;
        } else {
            if entry
                .samples
                .last()
                .is_some_and(|&(_, old)| old == count)
            {
                return None;
            }
            entry.samples.push((step, count));
            if entry.samples.len() > RUN_SPINE_WIDEN_KEEP {
                entry.samples.remove(0);
            }
        }

        if entry.samples.len() < RUN_SPINE_WIDEN_OBSERVATIONS {
            return None;
        }

        let (now_step, now_count) =
            entry.samples[entry.samples.len() - 1];
        for &(prev_step, prev_count) in entry.samples
            [..entry.samples.len() - 1]
            .iter()
            .rev()
            .take(12)
        {
            if prev_step >= now_step || prev_count >= now_count {
                continue;
            }

            let period = now_step - prev_step;
            let stride = now_count - prev_count;
            if stride == 0 {
                continue;
            }
            let Some(first_step) = prev_step.checked_sub(period) else {
                continue;
            };
            let Some(first_count) = prev_count.checked_sub(stride)
            else {
                continue;
            };
            if first_count == 0 {
                continue;
            }

            if entry.samples.contains(&(first_step, first_count)) {
                entry.widened = Some((first_count, stride));
                return Some((first_count, stride));
            }
        }

        None
    }

    fn widen(&mut self, config: &mut Config, step: Steps) -> bool {
        let mut changed = false;

        for side in [Side::Left, Side::Right] {
            let count = {
                let span = if side == Side::Left {
                    &config.tape.lspan
                } else {
                    &config.tape.rspan
                };
                let Some(Block::Run { count, .. }) = span.span.first()
                else {
                    continue;
                };
                if !matches!(
                    *count,
                    BlockCount::Exact(_) | BlockCount::Stride { .. }
                ) {
                    continue;
                }
                *count
            };

            let key = Self::key(config, side);
            let Some((minimum, stride)) =
                self.threshold(key, step, count)
            else {
                continue;
            };

            let span = if side == Side::Left {
                &mut config.tape.lspan
            } else {
                &mut config.tape.rspan
            };
            let Some(Block::Run { count, .. }) = span.span.first_mut()
            else {
                continue;
            };

            let widened = BlockCount::stride(minimum, stride);
            if *count != widened
                && (*count).covered_by_stride(minimum, stride)
            {
                *count = widened;
                changed = true;
            }
        }

        changed
    }
}

// Expensive single-color growing-edge history used only after the ordinary
// pass has actually reached CountLimit. This mechanism never widens those run
// counts. Instead, it remembers exact predecessor edges that increase the near
// side. If that same edge reaches u8::MAX after recurring with a stable
// (step,count) period, only the overflowing edge is cut; sibling exits and
// unrelated frontier branches remain live.
const OVERFLOW_CYCLE_MIN_PRIOR: usize = 6;
const OVERFLOW_CYCLE_KEEP: usize = 96;

#[derive(Default)]
struct OverflowCycleHistory {
    edges: Dict<GrowthEdgeKey, Vec<(Steps, Count)>>,
}

impl OverflowCycleHistory {
    fn observe(
        &mut self,
        key: GrowthEdgeKey,
        step: Steps,
        count: Count,
    ) {
        let obs = self.edges.entry(key).or_default();
        let pair = (step, count);
        if obs.contains(&pair) {
            return;
        }

        obs.push(pair);
        if obs.len() > OVERFLOW_CYCLE_KEEP {
            obs.remove(0);
        }
    }

    fn certifies(
        &self,
        key: &GrowthEdgeKey,
        step: Steps,
        count: Count,
    ) -> bool {
        let Some(obs) = self.edges.get(key) else {
            return false;
        };

        // The current overflowing parent is the newest point. Infer a
        // candidate macro-period from a previous occurrence of the same exact
        // edge skeleton, then demand several earlier occurrences at exactly
        // that step/count spacing. This is deliberately much stronger than
        // "the count got large": overflow merely triggers the check.
        for &(prev_step, prev_count) in obs.iter().rev().take(32) {
            if prev_step >= step || prev_count >= count {
                continue;
            }

            let period = step - prev_step;
            let delta = count - prev_count;
            if period == 0 || delta == 0 {
                continue;
            }

            let mut want_step = prev_step;
            let mut want_count = prev_count;
            let mut prior = 1;

            while prior < OVERFLOW_CYCLE_MIN_PRIOR {
                let Some(next_step) = want_step.checked_sub(period)
                else {
                    break;
                };
                let Some(next_count) = want_count.checked_sub(delta)
                else {
                    break;
                };

                if !obs.contains(&(next_step, next_count)) {
                    break;
                }

                want_step = next_step;
                want_count = next_count;
                prior += 1;
            }

            if prior >= OVERFLOW_CYCLE_MIN_PRIOR {
                return true;
            }
        }

        false
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct GrowthEdgeKey {
    state: State,
    scan: Color,
    read: Color,
    shift: Shift,
    prev_state: State,
    grow_side: Side,
    l_end: EndSig,
    r_end: EndSig,
    left: Vec<BlockSig>,
    right: Vec<BlockSig>,
}

#[derive(Clone, PartialEq, Eq, Hash)]
enum BlockSig {
    Run {
        color: Color,
        count: usize,
        indef: bool,
        stride: Option<Count>,
    },
    Word {
        word: BlockWord,
        count: usize,
        indef: bool,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum EndSig {
    Blanks,
    Unknown,
}

impl EndSig {
    const fn from_end(end: &TapeEnd) -> Self {
        match end {
            TapeEnd::Blanks => Self::Blanks,
            TapeEnd::Unknown => Self::Unknown,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Side {
    Left,
    Right,
}

fn span_runs(span: &Span) -> Vec<BlockSig> {
    span.span
        .iter()
        .map(|block| match block {
            Block::Run { color, count } => BlockSig::Run {
                color: *color,
                count: usize::from(count.minimum()),
                indef: count.is_indef(),
                stride: count.stride_step(),
            },
            Block::Word { word, count } => BlockSig::Word {
                word: Arc::clone(word),
                count: count.minimum(),
                indef: count.is_indef(),
            },
        })
        .collect()
}

fn growth_edge_observation(
    config: &Config,
    instr: Instr,
) -> Option<(GrowthEdgeKey, Count)> {
    let (read, shift, prev_state) = instr;
    let tape = &config.tape;
    let (grow_side, push) = if shift {
        (Side::Right, &tape.rspan)
    } else {
        (Side::Left, &tape.lspan)
    };

    let block = push.span.first()?;
    let (color, block_count) = block.run()?;
    if color != tape.scan {
        return None;
    }

    let count = block_count.minimum();
    let mut left = span_runs(&tape.lspan);
    let mut right = span_runs(&tape.rspan);
    let grow = match grow_side {
        Side::Left => &mut left,
        Side::Right => &mut right,
    };

    // `SpanT::iter()` is near-to-far, so the run merged by push_single is the
    // first run. Zero is only a wildcard in this key; real runs are nonzero.
    #[expect(clippy::unwrap_in_result)]
    let nearest = grow.first_mut().unwrap();
    let BlockSig::Run {
        color,
        count: signature_count,
        ..
    } = nearest
    else {
        unreachable!("single-color growth edge must start with a run")
    };
    debug_assert_eq!(*color, tape.scan);
    *signature_count = 0;

    Some((
        GrowthEdgeKey {
            state: config.state,
            scan: tape.scan,
            read,
            shift,
            prev_state,
            grow_side,
            l_end: EndSig::from_end(&tape.lspan.end),
            r_end: EndSig::from_end(&tape.rspan.end),
            left,
            right,
        },
        count,
    ))
}

/**************************************/

#[derive(Clone, PartialEq, Eq, Hash)]
enum TapeEnd {
    Blanks,
    Unknown,
}

type Count = u8;

/// A run count used only by the backward prover.
///
/// `Exact(n)` denotes exactly `n` cells. `AtLeast(n)` denotes an arbitrary
/// finite run of at least `n` cells.  Keeping the lower bound allows a run
/// created as `c..` to become `c^2..`, `c^3..`, ... as definite cells are
/// prepended, rather than losing that information forever.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum BlockCount {
    Exact(Count),
    AtLeast(Count),
    // Arithmetic-progression widening for a recurring monochromatic spine.
    // Denotes exactly { min + k * step | k >= 0 }. `step` is always >= 2;
    // step 1 canonicalizes to AtLeast(min).
    Stride { min: Count, step: Count },
}

impl BlockCount {
    const fn exact(count: Count) -> Self {
        debug_assert!(count > 0);
        Self::Exact(count)
    }

    const fn at_least(count: Count) -> Self {
        debug_assert!(count > 0);
        Self::AtLeast(count)
    }

    const fn stride(min: Count, step: Count) -> Self {
        debug_assert!(min > 0);
        debug_assert!(step > 0);
        if step == 1 {
            Self::AtLeast(min)
        } else {
            Self::Stride { min, step }
        }
    }

    const fn minimum(self) -> Count {
        match self {
            Self::Exact(count) | Self::AtLeast(count) => count,
            Self::Stride { min, .. } => min,
        }
    }

    const fn is_single(self) -> bool {
        matches!(self, Self::Exact(1))
    }

    const fn is_indef(self) -> bool {
        matches!(self, Self::AtLeast(_) | Self::Stride { .. })
    }

    const fn can_be_one(self) -> bool {
        matches!(self, Self::AtLeast(1) | Self::Stride { min: 1, .. })
    }

    const fn stride_step(self) -> Option<Count> {
        match self {
            Self::Stride { step, .. } => Some(step),
            _ => None,
        }
    }

    const fn parity_variable(self) -> bool {
        match self {
            Self::Exact(_) => false,
            Self::AtLeast(_) => true,
            Self::Stride { step, .. } => step & 1 != 0,
        }
    }

    const fn covered_by_stride(self, min: Count, step: Count) -> bool {
        match self {
            Self::Exact(count) => {
                count >= min && (count - min).is_multiple_of(step)
            },
            Self::Stride {
                min: own_min,
                step: own_step,
            } => {
                own_min >= min
                    && (own_min - min).is_multiple_of(step)
                    && own_step % step == 0
            },
            Self::AtLeast(count) => step == 1 && count >= min,
        }
    }

    fn add_exact(&mut self, add: Count) -> Result<(), BackwardResult> {
        debug_assert!(add > 0);
        *self = match *self {
            Self::Exact(count) => {
                Self::Exact(count.checked_add(add).ok_or(CountLimit)?)
            },
            Self::AtLeast(count) => {
                Self::AtLeast(count.checked_add(add).ok_or(CountLimit)?)
            },
            Self::Stride { min, step } => Self::Stride {
                min: min.checked_add(add).ok_or(CountLimit)?,
                step,
            },
        };
        Ok(())
    }

    fn add_at_least(
        &mut self,
        add: Count,
    ) -> Result<(), BackwardResult> {
        debug_assert!(add > 0);
        let count =
            self.minimum().checked_add(add).ok_or(CountLimit)?;
        // Adding an arbitrary lower-bounded number of same-color cells erases
        // any congruence information carried by a Stride run.
        *self = Self::AtLeast(count);
        Ok(())
    }

    /// Remove one definitely present cell.
    ///
    /// `AtLeast(1)` is used only on the residual branch where the concrete
    /// run had length at least two, so its residual is again `AtLeast(1)`.
    /// For `Stride { min: 1 }`, step_configs likewise explores the exact-one
    /// branch separately; the residual progression therefore starts at step.
    const fn decrement_after_pull(&mut self) {
        *self = match *self {
            Self::Exact(count) => {
                debug_assert!(count > 1);
                Self::Exact(count - 1)
            },
            Self::AtLeast(1) => Self::AtLeast(1),
            Self::AtLeast(count) => Self::AtLeast(count - 1),
            Self::Stride { min: 1, step } => {
                Self::Stride { min: step, step }
            },
            Self::Stride { min, step } => {
                Self::Stride { min: min - 1, step }
            },
        };
    }
}

/// Copy count for a repeated compound word.  `AtLeast(n)` is the word-level
/// counterpart of an indefinite monochromatic run: it denotes any finite
/// number of complete copies greater than or equal to `n`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum WordCount {
    Exact(usize),
    AtLeast(usize),
}

impl WordCount {
    const fn exact(count: usize) -> Self {
        debug_assert!(count > 0);
        Self::Exact(count)
    }

    const fn at_least(count: usize) -> Self {
        debug_assert!(count > 0);
        Self::AtLeast(count)
    }

    const fn minimum(self) -> usize {
        match self {
            Self::Exact(count) | Self::AtLeast(count) => count,
        }
    }

    const fn is_indef(self) -> bool {
        matches!(self, Self::AtLeast(_))
    }

    const fn can_be_one(self) -> bool {
        matches!(self, Self::AtLeast(1))
    }

    const fn exact_copies(self) -> Option<usize> {
        match self {
            Self::Exact(count) => Some(count),
            Self::AtLeast(_) => None,
        }
    }

    /// Merge adjacent copies of the same periodic word.  If either side was
    /// widened, retain its established lower bound instead of strengthening it
    /// with newly materialized exact copies.  This is a deliberate widening:
    /// it is a sound superset and makes the abstract macro-cycle stable.
    fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Exact(left), Self::Exact(right)) => Self::Exact(
                left.checked_add(right)
                    .expect("BKW word count overflow"),
            ),
            (Self::AtLeast(min), Self::Exact(_))
            | (Self::Exact(_), Self::AtLeast(min)) => {
                Self::AtLeast(min)
            },
            (Self::AtLeast(left), Self::AtLeast(right)) => {
                Self::AtLeast(left.min(right))
            },
        }
    }
}

const BKW_REBALANCE_WINDOW: usize = 64;
const BKW_MAX_PATTERN: usize = 16;
const BKW_MIN_PATTERN_REPEATS: usize = 3;

type BlockWord = Arc<[Color]>;

fn bkw_primitive_word(word: &[Color]) -> (&[Color], usize) {
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

/// One exact/indefinite monochromatic run, or a repeated short word.
///
/// `Run` retains the original BKW semantics. `Word` uses the same exact versus
/// at-least distinction at copy granularity; symbols inside the primitive word
/// are stored from the head outward. This lets multi-step periodic growth reach
/// a finite abstract state such as `(1 2)^3..` instead of an unbounded ladder.
#[derive(Clone, PartialEq, Eq, Hash)]
enum Block {
    Run { color: Color, count: BlockCount },
    Word { word: BlockWord, count: WordCount },
}

impl Block {
    const fn exact(color: Color, count: Count) -> Self {
        Self::Run {
            color,
            count: BlockCount::exact(count),
        }
    }

    const fn at_least(color: Color, count: Count) -> Self {
        Self::Run {
            color,
            count: BlockCount::at_least(count),
        }
    }

    fn exact_word(word: &[Color], copies: usize) -> Self {
        debug_assert_ne!(word, []);
        debug_assert!(copies > 0);

        let (root, factor) = bkw_primitive_word(word);
        let copies = copies
            .checked_mul(factor)
            .expect("BKW word copy count overflow");

        if root.len() == 1 && copies <= usize::from(Count::MAX) {
            #[expect(clippy::cast_possible_truncation)]
            return Self::exact(root[0], copies as Count);
        }

        Self::Word {
            word: BlockWord::from(root.to_vec()),
            count: WordCount::exact(copies),
        }
    }

    fn at_least_word(word: &[Color], copies: usize) -> Self {
        debug_assert_ne!(word, []);
        debug_assert!(copies > 0);

        let (root, factor) = bkw_primitive_word(word);
        let copies = copies
            .checked_mul(factor)
            .expect("BKW word copy count overflow");

        if root.len() == 1 && copies <= usize::from(Count::MAX) {
            #[expect(clippy::cast_possible_truncation)]
            return Self::at_least(root[0], copies as Count);
        }

        Self::Word {
            word: BlockWord::from(root.to_vec()),
            count: WordCount::at_least(copies),
        }
    }

    const fn run(&self) -> Option<(Color, BlockCount)> {
        match self {
            Self::Run { color, count } => Some((*color, *count)),
            Self::Word { .. } => None,
        }
    }

    fn word(&self) -> &[Color] {
        match self {
            Self::Run { color, .. } => core::slice::from_ref(color),
            Self::Word { word, .. } => word,
        }
    }

    const fn exact_copies(&self) -> Option<usize> {
        match self {
            Self::Run {
                count: BlockCount::Exact(count),
                ..
            } => Some(*count as usize),
            Self::Run {
                count:
                    BlockCount::AtLeast(_) | BlockCount::Stride { .. },
                ..
            } => None,
            Self::Word { count, .. } => count.exact_copies(),
        }
    }

    const fn can_be_one(&self) -> bool {
        match self {
            Self::Run { count, .. } => count.can_be_one(),
            Self::Word { count, .. } => count.can_be_one(),
        }
    }

    fn first_color(&self) -> Color {
        self.word()[0]
    }

    fn width(&self) -> usize {
        self.word().len()
    }

    fn blank(&self) -> bool {
        self.word().iter().all(|&color| color == 0)
    }

    fn contains_nonblank(&self) -> bool {
        self.word().iter().any(|&color| color != 0)
    }

    fn total_exact_cells(&self) -> Option<usize> {
        self.width().checked_mul(self.exact_copies()?)
    }

    fn display(&self, reverse: bool) -> String {
        match self {
            Self::Run { color, count } => match count {
                BlockCount::Exact(1) => color.to_string(),
                BlockCount::Exact(count) => format!("{color}^{count}"),
                BlockCount::AtLeast(1) => format!("{color}.."),
                BlockCount::AtLeast(count) => {
                    format!("{color}^{count}..")
                },
                BlockCount::Stride { min, step } => {
                    format!("{color}^({min}+{step}k)")
                },
            },
            Self::Word { word, count } => {
                let symbols = if reverse {
                    word.iter()
                        .rev()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                } else {
                    word.iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                };
                let shown = format!("({})", symbols.join(" "));
                match count {
                    WordCount::Exact(1) => shown,
                    WordCount::Exact(count) => {
                        format!("{shown}^{count}")
                    },
                    WordCount::AtLeast(1) => format!("{shown}.."),
                    WordCount::AtLeast(count) => {
                        format!("{shown}^{count}..")
                    },
                }
            },
        }
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.display(false))
    }
}

/// Minimal near-head span implementation for BKW. Storage matches the shared
/// tape span: farthest block first, nearest block last. Compound block words
/// themselves are stored in near-to-far order.
#[derive(Clone, PartialEq, Eq, Hash)]
struct SpanT {
    blocks: Vec<Block>,
}

impl SpanT {
    const fn init_blank() -> Self {
        Self { blocks: vec![] }
    }

    const fn len(&self) -> usize {
        self.blocks.len()
    }

    const fn blank(&self) -> bool {
        self.blocks.is_empty()
    }

    fn iter(&self) -> impl DoubleEndedIterator<Item = &Block> {
        self.blocks.iter().rev()
    }

    fn first(&self) -> Option<&Block> {
        self.blocks.last()
    }

    fn first_mut(&mut self) -> Option<&mut Block> {
        self.blocks.last_mut()
    }

    fn pop_block(&mut self) -> Block {
        self.blocks.pop().unwrap()
    }

    fn push_exact(
        &mut self,
        color: Color,
        count: Count,
    ) -> Result<(), BackwardResult> {
        if let Some(Block::Run {
            color: near_color,
            count: near_count,
        }) = self.first_mut()
            && *near_color == color
        {
            return near_count.add_exact(count);
        }

        self.blocks.push(Block::exact(color, count));
        Ok(())
    }

    fn push_at_least(
        &mut self,
        color: Color,
        count: Count,
    ) -> Result<(), BackwardResult> {
        if let Some(Block::Run {
            color: near_color,
            count: near_count,
        }) = self.first_mut()
            && *near_color == color
        {
            return near_count.add_at_least(count);
        }

        self.blocks.push(Block::at_least(color, count));
        Ok(())
    }

    #[cfg(test)]
    fn push_block(
        &mut self,
        block: &Block,
    ) -> Result<(), BackwardResult> {
        match block {
            Block::Run { color, count } => match count {
                BlockCount::Exact(count) => {
                    self.push_exact(*color, *count)
                },
                BlockCount::AtLeast(count) => {
                    self.push_at_least(*color, *count)
                },
                BlockCount::Stride { .. } => {
                    self.blocks.push(block.clone());
                    Ok(())
                },
            },
            Block::Word { .. } => {
                self.blocks.push(block.clone());
                Ok(())
            },
        }
    }

    /// Copy up to the rebalance window of exact cells from the head outward.
    /// An `AtLeast` run is a semantic boundary: periodic structure must never
    /// be inferred through an unknown amount of tape.
    fn boundary_cells(
        &self,
        cells: &mut [Color; BKW_REBALANCE_WINDOW],
    ) -> usize {
        let mut size = 0;

        for block in self.iter() {
            let Some(copies) = block.exact_copies() else {
                break;
            };

            let word = block.word();
            let total = word.len().saturating_mul(copies);
            let take = total.min(BKW_REBALANCE_WINDOW - size);

            for offset in 0..take {
                cells[size + offset] = word[offset % word.len()];
            }
            size += take;

            if take < total || size == BKW_REBALANCE_WINDOW {
                break;
            }
        }

        size
    }

    fn best_boundary_repeat(
        &self,
    ) -> Option<(Vec<Color>, usize, usize)> {
        let mut cells = [0; BKW_REBALANCE_WINDOW];
        let size = self.boundary_cells(&mut cells);
        let mut best: Option<(usize, usize, usize)> = None;

        for width in
            2..=BKW_MAX_PATTERN.min(size / BKW_MIN_PATTERN_REPEATS)
        {
            let word = &cells[..width];
            let mut copies = 1;

            while (copies + 1) * width <= size
                && &cells[copies * width..(copies + 1) * width] == word
            {
                copies += 1;
            }

            if copies < BKW_MIN_PATTERN_REPEATS {
                continue;
            }

            let (root, factor) = bkw_primitive_word(word);
            if root.len() == 1 {
                continue;
            }

            let root_copies = copies * factor;
            let covered = width * copies;
            let root_width = root.len();
            let replace = best.is_none_or(
                |(best_width, best_copies, best_covered)| {
                    root_copies > best_copies
                        || (root_copies == best_copies
                            && covered > best_covered)
                        || (root_copies == best_copies
                            && covered == best_covered
                            && root_width < best_width)
                },
            );

            if replace {
                best = Some((root_width, root_copies, covered));
            }
        }

        best.map(|(width, copies, covered)| {
            (cells[..width].to_vec(), copies, covered)
        })
    }

    /// Remove exactly `cells` known cells from the near end. The caller only
    /// supplies a prefix returned by `boundary_cells`, so no `AtLeast` block is
    /// ever consumed here.
    fn consume_near_exact_cells(&mut self, mut cells: usize) {
        while cells != 0 {
            let block = self.pop_block();
            let total = block
                .total_exact_cells()
                .expect("rebalance cannot consume an indefinite run");

            if cells >= total {
                cells -= total;
                continue;
            }

            #[expect(clippy::match_same_arms)]
            match block {
                Block::Run {
                    color,
                    count: BlockCount::Exact(count),
                } => {
                    debug_assert!(cells < usize::from(count));
                    #[expect(clippy::cast_possible_truncation)]
                    self.blocks.push(Block::exact(
                        color,
                        count - cells as Count,
                    ));
                },
                Block::Word {
                    word,
                    count: WordCount::Exact(count),
                } => {
                    let width = word.len();
                    let whole = cells / width;
                    let offset = cells % width;
                    let mut remaining = count - whole;

                    if offset != 0 {
                        remaining -= 1;
                    }

                    if remaining != 0 {
                        self.blocks.push(Block::Word {
                            word: Arc::clone(&word),
                            count: WordCount::exact(remaining),
                        });
                    }
                    if offset != 0 {
                        self.blocks.push(Block::exact_word(
                            &word[offset..],
                            1,
                        ));
                    }
                },
                Block::Word {
                    count: WordCount::AtLeast(_),
                    ..
                } => unreachable!(),
                Block::Run {
                    count:
                        BlockCount::AtLeast(_) | BlockCount::Stride { .. },
                    ..
                } => unreachable!(),
            }

            cells = 0;
        }
    }

    fn merge_near_equal_words(&mut self) {
        while self.blocks.len() >= 2 {
            let near = self.blocks.len() - 1;
            let far = near - 1;

            let merge = match (&self.blocks[far], &self.blocks[near]) {
                (
                    Block::Word {
                        word: far_word,
                        count: far_count,
                    },
                    Block::Word {
                        word: near_word,
                        count: near_count,
                    },
                ) if far_word == near_word => Some(Block::Word {
                    word: Arc::clone(far_word),
                    count: far_count.merge(*near_count),
                }),
                _ => None,
            };

            let Some(merged) = merge else {
                break;
            };

            self.blocks.pop();
            self.blocks.pop();
            self.blocks.push(merged);
        }
    }

    fn discover_boundary(&mut self) -> bool {
        if self.blocks.len() < 2 {
            return false;
        }

        let Some((word, copies, covered)) = self.best_boundary_repeat()
        else {
            return false;
        };

        if self.first().is_some_and(|block| {
            matches!(
                block,
                Block::Word { word: near_word, .. }
                    if near_word.as_ref() == word.as_slice()
            )
        }) {
            return false;
        }

        self.consume_near_exact_cells(covered);
        self.blocks.push(Block::exact_word(&word, copies));
        self.merge_near_equal_words();
        true
    }

    fn absorb_exact_prefix_into_indef_word(&mut self) -> bool {
        let Some((position, word)) =
            self.iter().enumerate().find_map(|(position, block)| {
                match block {
                    Block::Word {
                        word,
                        count: WordCount::AtLeast(_),
                    } => Some((position, Arc::clone(word))),
                    _ => None,
                }
            })
        else {
            return false;
        };

        if position == 0 {
            return false;
        }

        let mut cells = [0; BKW_REBALANCE_WINDOW];
        let mut size = 0_usize;
        for block in self.iter().take(position) {
            let Some(copies) = block.exact_copies() else {
                return false;
            };
            let block_word = block.word();
            let total = block_word.len().saturating_mul(copies);
            if total > BKW_REBALANCE_WINDOW - size {
                return false;
            }
            for offset in 0..total {
                cells[size + offset] =
                    block_word[offset % block_word.len()];
            }
            size += total;
        }

        if size == 0 || !size.is_multiple_of(word.len()) {
            return false;
        }
        if (0..size)
            .any(|offset| cells[offset] != word[offset % word.len()])
        {
            return false;
        }

        // The exact copies are guaranteed, but an already-widened word is
        // intentionally stable under adding complete copies. Dropping this
        // exact prefix therefore widens `(word)^r (word)^m..` back to the
        // existing `(word)^m..`, a sound superset that closes the macro-cycle.
        self.consume_near_exact_cells(size);
        true
    }

    fn normalize_boundary(&mut self) {
        self.merge_near_equal_words();
        self.absorb_exact_prefix_into_indef_word();
        self.merge_near_equal_words();

        // Before the first compound block exists, three repeats of a
        // non-homogeneous primitive word require at least six ordinary runs.
        // Keep the overwhelmingly common short-span path O(1). Once a word is
        // at the boundary, however, even one newly pushed cell may change its
        // phase and should trigger rebalancing.
        let near_word = self
            .iter()
            .take(2)
            .any(|block| matches!(block, Block::Word { .. }));
        if !near_word && self.blocks.len() < 6 {
            return;
        }

        // One rewrite is normally sufficient. A small bounded loop handles a
        // phase change that exposes a second identical word boundary without
        // turning normalization into an unbounded search.
        for _ in 0..4 {
            if !self.discover_boundary() {
                break;
            }
        }
    }

    fn pull_one(&mut self) {
        let Some(block) = self.first().cloned() else {
            return;
        };

        match block {
            Block::Run { count, .. } if count.is_single() => {
                self.pop_block();
            },
            Block::Run { .. } => {
                let Block::Run { count, .. } =
                    self.first_mut().unwrap()
                else {
                    unreachable!()
                };
                count.decrement_after_pull();
            },
            Block::Word { word, count } => {
                self.pop_block();

                match count {
                    WordCount::Exact(count) => {
                        if count > 1 {
                            self.blocks.push(Block::Word {
                                word: Arc::clone(&word),
                                count: WordCount::exact(count - 1),
                            });
                        }
                    },
                    WordCount::AtLeast(1) => {
                        // `step_configs` separately explores the exact-one-copy
                        // branch. This residual branch therefore represents two
                        // or more copies before the pull, leaving one-or-more.
                        self.blocks
                            .push(Block::at_least_word(&word, 1));
                    },
                    WordCount::AtLeast(count) => {
                        self.blocks.push(Block::at_least_word(
                            &word,
                            count - 1,
                        ));
                    },
                }

                if word.len() > 1 {
                    self.blocks.push(Block::exact_word(&word[1..], 1));
                }
            },
        }

        self.normalize_boundary();
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct Span {
    span: SpanT,
    end: TapeEnd,
}

impl Span {
    const fn init_blank() -> Self {
        Self {
            span: SpanT::init_blank(),
            end: TapeEnd::Blanks,
        }
    }

    const fn init_unknown() -> Self {
        Self {
            span: SpanT::init_blank(),
            end: TapeEnd::Unknown,
        }
    }

    fn init_unknown_with(color: Color) -> Self {
        let mut span = Self {
            span: SpanT::init_blank(),
            end: TapeEnd::Unknown,
        };

        span.push_single(color)
            .expect("single cell cannot overflow an empty span");

        span
    }

    const fn end_str(&self) -> &str {
        match self.end {
            TapeEnd::Blanks => "0+",
            TapeEnd::Unknown => "?",
        }
    }

    fn blank(&self) -> bool {
        self.span.iter().all(Block::blank)
    }

    fn matches_color(&self, print: Color) -> bool {
        self.span.first().map_or_else(
            || match self.end {
                TapeEnd::Blanks => print == 0,
                TapeEnd::Unknown => true,
            },
            |block| block.first_color() == print,
        )
    }

    fn pull(&mut self) {
        self.span.pull_one();
    }

    fn push_single(
        &mut self,
        color: Color,
    ) -> Result<(), BackwardResult> {
        if self.span.first().is_none()
            && color == 0
            && self.end == TapeEnd::Blanks
        {
            return Ok(());
        }

        self.span.push_exact(color, 1)?;
        self.span.normalize_boundary();
        Ok(())
    }

    fn push_indef(
        &mut self,
        color: Color,
    ) -> Result<(), BackwardResult> {
        if color == 0
            && self.span.blank()
            && self.end == TapeEnd::Blanks
        {
            return Ok(());
        }

        self.span.push_at_least(color, 1)
    }

    fn set_head_to_one(&mut self) {
        let block = self.span.first_mut().unwrap();
        match block {
            Block::Run { count, .. } => {
                debug_assert!(count.can_be_one());
                *count = BlockCount::Exact(1);
            },
            Block::Word { count, .. } => {
                debug_assert!(count.can_be_one());
                *count = WordCount::Exact(1);
            },
        }
    }

    /// If this span's end is known to be all blanks (`0+`), then any explicit
    /// trailing blank blocks at the *far* end are redundant and can be dropped.
    ///
    /// This keeps canonical forms like `0+ 0 [x] ?` from persisting as distinct
    /// configurations; it becomes `0+ [x] ?`.
    fn absorb_trailing_blanks(&mut self) {
        if self.end != TapeEnd::Blanks {
            return;
        }

        // Storage is farthest-to-nearest, so whole blank blocks at index zero
        // are already absorbed by the known blank tail. A mixed compound word
        // is retained: only its far suffix is redundant, and keeping it explicit
        // is semantically exact and avoids splitting a periodic block here.
        while self.span.blocks.first().is_some_and(Block::blank) {
            self.span.blocks.remove(0);
        }
    }
}

#[expect(clippy::multiple_inherent_impl)]
impl Span {
    fn explicit_nonblank(&self) -> bool {
        self.span.iter().any(Block::contains_nonblank)
    }

    #[expect(clippy::cast_possible_truncation)]
    fn tail_color_count_masks<const C: usize>(&self) -> [u8; C] {
        let mut minimum = [0_u8; C];
        let mut variable = [self.end == TapeEnd::Unknown; C];
        let mut first_block = true;

        for block in self.span.iter() {
            match block {
                Block::Run { color, count } => {
                    let color = *color as usize;
                    if color != 0 {
                        // Tail summaries are strictly beyond the immediate
                        // neighbor. Remove that cell before applying the 2+
                        // cap; capping first would turn a true tail count of
                        // 2+ into an unsound exact-one requirement for a near
                        // run of length at least three.
                        let contribution = usize::from(count.minimum())
                            .saturating_sub(usize::from(first_block));
                        minimum[color] = minimum[color]
                            .saturating_add(contribution.min(2) as u8)
                            .min(2);
                        variable[color] |= count.is_indef();
                    }
                },
                Block::Word { word, count } => {
                    for color in 1..C {
                        let per_word = word
                            .iter()
                            .filter(|&&symbol| symbol as usize == color)
                            .count();
                        if per_word == 0 {
                            continue;
                        }

                        let mut contribution =
                            per_word.saturating_mul(count.minimum());
                        if first_block && word[0] as usize == color {
                            contribution =
                                contribution.saturating_sub(1);
                        }

                        minimum[color] = minimum[color]
                            .saturating_add(contribution.min(2) as u8)
                            .min(2);
                        variable[color] |= count.is_indef();
                    }
                },
            }

            first_block = false;
        }

        let mut out = [0_u8; C];
        for color in 1..C {
            out[color] = match (minimum[color], variable[color]) {
                (2, _) => 0b100,
                (1, true) => 0b110,
                (0, true) => 0b111,
                (count, false) => 1_u8 << count,
                _ => unreachable!(),
            };
        }
        out
    }

    /// Exact nonblank residue modulo `modulus`, or `None` if an unknown end or
    /// an indefinite nonblank run makes the residue unconstrained.
    #[expect(clippy::cast_possible_truncation)]
    fn nonblank_residue(&self, modulus: u8) -> Option<u8> {
        if self.end == TapeEnd::Unknown {
            return None;
        }

        let mut residue = 0_u8;
        for block in self.span.iter() {
            match block {
                Block::Run { color, count } => {
                    if *color == 0 {
                        continue;
                    }
                    match count {
                        BlockCount::Exact(count) => {
                            residue =
                                (residue + count % modulus) % modulus;
                        },
                        BlockCount::AtLeast(_) => return None,
                        BlockCount::Stride { min, step } => {
                            if step % modulus != 0 {
                                return None;
                            }
                            residue =
                                (residue + min % modulus) % modulus;
                        },
                    }
                },
                Block::Word { word, count } => {
                    let marked = word
                        .iter()
                        .filter(|&&color| color != 0)
                        .count();
                    let per_copy = marked % usize::from(modulus);
                    if count.is_indef() && per_copy != 0 {
                        return None;
                    }
                    let add = per_copy
                        * (count.minimum() % usize::from(modulus));
                    residue = (residue + add as u8) % modulus;
                },
            }
        }
        Some(residue)
    }

    fn colors_allowed<const C: usize>(
        &self,
        forbidden: &[bool; C],
    ) -> bool {
        self.span.iter().all(|block| {
            block.word().iter().all(|&color| !forbidden[color as usize])
        })
    }

    /// Check the fresh-zero ordering rule on one side. Only two copies of a
    /// periodic word ever need inspection: if a zero in one copy can be
    /// followed by a nonzero in a later copy, the second copy witnesses it.
    fn fresh_zero_order_valid(&self) -> (bool, bool) {
        let mut seen_zero = false;

        for block in self.span.iter() {
            match block {
                Block::Run { color, .. } => {
                    if seen_zero && *color != 0 {
                        return (false, seen_zero);
                    }
                    if *color == 0 {
                        seen_zero = true;
                    }
                },
                Block::Word { word, count } => {
                    // Only guaranteed copies may witness a contradiction. For
                    // `AtLeast(1)`, a violation that appears only across the
                    // first/second-copy boundary is not universal.
                    for _ in 0..count.minimum().min(2) {
                        for &color in word.iter() {
                            if seen_zero && color != 0 {
                                return (false, seen_zero);
                            }
                            if color == 0 {
                                seen_zero = true;
                            }
                        }
                    }
                },
            }
        }

        (true, seen_zero)
    }
}

/**************************************/

#[derive(Clone, PartialEq, Eq)]
struct Tape {
    scan: Color,
    lspan: Span,
    rspan: Span,
}

impl Scan for Tape {
    fn scan(&self) -> Color {
        self.scan
    }
}

impl fmt::Display for Tape {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} {} {}",
            self.lspan.end_str(),
            self.lspan
                .span
                .iter()
                .rev()
                .map(|block| block.display(true))
                .chain(once(format!("[{}]", self.scan)))
                .chain(
                    self.rspan
                        .span
                        .iter()
                        .map(|block| block.display(false)),
                )
                .collect::<Vec<_>>()
                .join(" "),
            self.rspan.end_str(),
        )
    }
}

impl Tape {
    const fn init_halt(scan: Color) -> Self {
        Self {
            scan,
            lspan: Span::init_unknown(),
            rspan: Span::init_unknown(),
        }
    }

    const fn init_blank(scan: Color) -> Self {
        Self {
            scan,
            lspan: Span::init_blank(),
            rspan: Span::init_blank(),
        }
    }

    const fn init_spinout(dir: Shift) -> Self {
        if dir {
            Self::init_r_spinout()
        } else {
            Self::init_l_spinout()
        }
    }

    const fn init_r_spinout() -> Self {
        Self {
            scan: 0,
            lspan: Span::init_unknown(),
            rspan: Span::init_blank(),
        }
    }

    const fn init_l_spinout() -> Self {
        Self {
            scan: 0,
            lspan: Span::init_blank(),
            rspan: Span::init_unknown(),
        }
    }

    fn init_twostep(l_co: Color, r_co: Color) -> Self {
        Self {
            scan: l_co,
            lspan: Span::init_unknown(),
            rspan: Span::init_unknown_with(r_co),
        }
    }

    fn blank(&self) -> bool {
        self.scan == 0 && self.lspan.blank() && self.rspan.blank()
    }

    /// Check whole-side facts against the same-run forward abstraction.
    ///
    /// A backward span can prove one of three things about each side:
    /// - wholly blank (`0+` with no explicit nonblank),
    /// - definitely dirty (some explicit nonblank), or
    /// - unknown.
    ///
    /// The cheap state/scan status table is checked first, then the same status
    /// requirement must coexist with a compatible exact local window. Exact
    /// blank single-side facts additionally retain the stronger excursion-based
    /// halfblank checks.
    fn obeys_blank_side_possible<const S: usize, const C: usize>(
        &self,
        state: State,
        possible: &BlankSidePossible<S, C>,
    ) -> bool {
        #[derive(Clone, Copy)]
        enum RequiredStatus {
            Blank,
            Dirty,
            Unknown,
        }

        fn status(span: &Span) -> RequiredStatus {
            // One pass over explicit blocks.  An explicit nonblank proves the
            // side dirty even when the far end is unknown; otherwise a blank
            // end proves the whole side blank.
            if span.explicit_nonblank() {
                RequiredStatus::Dirty
            } else if span.end == TapeEnd::Blanks {
                RequiredStatus::Blank
            } else {
                RequiredStatus::Unknown
            }
        }

        const fn allowed_status_mask(
            left: RequiredStatus,
            right: RequiredStatus,
        ) -> u8 {
            use RequiredStatus::{Blank, Dirty, Unknown};

            match (left, right) {
                (Blank, Blank) => 1_u8 << BOTH_BLANK_FLAGS,
                (Blank, Dirty) => 1_u8 << LEFT_BLANK_FLAG,
                (Blank, Unknown) => {
                    (1_u8 << LEFT_BLANK_FLAG)
                        | (1_u8 << BOTH_BLANK_FLAGS)
                },
                (Dirty, Blank) => 1_u8 << RIGHT_BLANK_FLAG,
                (Dirty, Dirty) => 1_u8,
                (Dirty, Unknown) => 1_u8 | (1_u8 << RIGHT_BLANK_FLAG),
                (Unknown, Blank) => {
                    (1_u8 << RIGHT_BLANK_FLAG)
                        | (1_u8 << BOTH_BLANK_FLAGS)
                },
                (Unknown, Dirty) => 1_u8 | (1_u8 << LEFT_BLANK_FLAG),
                (Unknown, Unknown) => 0b1111,
            }
        }

        let st = state as usize;
        let sc = self.scan as usize;
        let left_status = status(&self.lspan);
        let right_status = status(&self.rspan);

        // No whole-side fact is known, so this abstraction cannot add any
        // pruning.  In particular avoid the C/C^2 exact-window scan common in
        // halt cones with two unknown tails.
        if matches!(left_status, RequiredStatus::Unknown)
            && matches!(right_status, RequiredStatus::Unknown)
        {
            return true;
        }

        let known_left = self.left_neighbor_color().map(usize::from);
        let known_right = self.right_neighbor_color().map(usize::from);

        if matches!(left_status, RequiredStatus::Blank) {
            let inward = possible.left_half[st][sc];
            let halfblank_ok = known_right
                .map_or(inward != 0, |right| {
                    inward & (1_u64 << right) != 0
                });
            if !halfblank_ok {
                return false;
            }
        }
        if matches!(right_status, RequiredStatus::Blank) {
            let inward = possible.right_half[st][sc];
            let halfblank_ok = known_left.map_or(inward != 0, |left| {
                inward & (1_u64 << left) != 0
            });
            if !halfblank_ok {
                return false;
            }
        }

        let allowed = allowed_status_mask(left_status, right_status);
        if possible.joint.any[st][sc] & allowed == 0 {
            return false;
        }

        let matches_window = |left: usize, right: usize| {
            possible.joint.window_mask(st, sc, left, right) & allowed
                != 0
        };

        match (known_left, known_right) {
            (Some(left), Some(right)) => matches_window(left, right),
            (Some(left), None) => {
                (0..C).any(|right| matches_window(left, right))
            },
            (None, Some(right)) => {
                (0..C).any(|left| matches_window(left, right))
            },
            (None, None) => (0..C).any(|left| {
                (0..C).any(|right| matches_window(left, right))
            }),
        }
    }

    /// Check capped per-color tail counts and pairwise same-run presence.
    ///
    /// Each color first gets the stronger independent `0 / 1 / 2+` count
    /// check on both tails. The same count requirements are then coarsened to
    /// absent/present masks for the pairwise same-run filter, so the stronger
    /// count layer does not add another backward tape scan.
    fn obeys_tail_presence<const S: usize, const C: usize>(
        &self,
        state: State,
        count: &ColorTailCountPossible<S, C>,
        pair: &PairTailPresencePossible<S, C>,
    ) -> bool {
        /// Nine-bit set of allowed `(left_count, right_count)` statuses.
        /// Status is `left_count + 3 * right_count`.
        const fn count_allowed_mask(left: u8, right: u8) -> u16 {
            let mut out = 0_u16;
            let mut l = 0_u8;
            while l < 3 {
                if left & (1_u8 << l) != 0 {
                    let mut r = 0_u8;
                    while r < 3 {
                        if right & (1_u8 << r) != 0 {
                            out |= 1_u16 << (l + 3 * r);
                        }
                        r += 1;
                    }
                }
                l += 1;
            }
            out
        }

        /// Coarsen capped count masks to the old four-state presence product.
        const fn presence_allowed_mask(left: u8, right: u8) -> u8 {
            let left_absent = left & 0b001 != 0;
            let left_present = left & 0b110 != 0;
            let right_absent = right & 0b001 != 0;
            let right_present = right & 0b110 != 0;

            let mut out = 0_u8;
            if left_absent && right_absent {
                out |= 1 << 0;
            }
            if left_present && right_absent {
                out |= 1 << 1;
            }
            if left_absent && right_present {
                out |= 1 << 2;
            }
            if left_present && right_present {
                out |= 1 << 3;
            }
            out
        }

        /// Lift two four-state single-color presence masks to the 16-state
        /// pair mask. Pair status is `a_status | (b_status << 2)`.
        const fn pair_allowed_mask(a: u8, b: u8) -> u16 {
            let a = a as u16;
            let mut out = 0_u16;
            if b & 0b0001 != 0 {
                out |= a;
            }
            if b & 0b0010 != 0 {
                out |= a << 4;
            }
            if b & 0b0100 != 0 {
                out |= a << 8;
            }
            if b & 0b1000 != 0 {
                out |= a << 12;
            }
            out
        }

        let st = state as usize;
        let sc = self.scan as usize;
        let left_neighbor = self.left_neighbor_color().map(usize::from);
        let right_neighbor =
            self.right_neighbor_color().map(usize::from);

        let left_counts = self.lspan.tail_color_count_masks::<C>();
        let right_counts = self.rspan.tail_color_count_masks::<C>();
        let mut presence_allowed = [0b1111_u8; C];
        let mut constrained_presence = 0_u64;

        for color in 1..C {
            let left_count = left_counts[color];
            let right_count = right_counts[color];
            let required_count =
                count_allowed_mask(left_count, right_count);

            // Both unknown tails permit all nine count combinations, so the
            // independent count layer cannot prune this color.
            if required_count != 0x01ff {
                let forward = count.mask(
                    st,
                    sc,
                    left_neighbor,
                    right_neighbor,
                    color,
                );
                if forward & required_count == 0 {
                    return false;
                }
            }

            let required_presence =
                presence_allowed_mask(left_count, right_count);
            presence_allowed[color] = required_presence;
            if required_presence != 0b1111 {
                constrained_presence |= 1_u64 << color;
            }
        }

        if C < 3 || constrained_presence.count_ones() < 2 {
            return true;
        }

        let mut a_colors = constrained_presence;
        while a_colors != 0 {
            let a = a_colors.trailing_zeros() as usize;
            a_colors &= a_colors - 1;

            let mut b_colors = a_colors;
            while b_colors != 0 {
                let b = b_colors.trailing_zeros() as usize;
                b_colors &= b_colors - 1;

                let required = pair_allowed_mask(
                    presence_allowed[a],
                    presence_allowed[b],
                );
                let forward = pair.mask(
                    st,
                    sc,
                    left_neighbor,
                    right_neighbor,
                    a,
                    b,
                );
                if forward & required == 0 {
                    return false;
                }
            }
        }

        true
    }

    /// Return possible nonblank-count parities for the left and right spans
    /// separately. Bit 0 means even, bit 1 means odd. Unknown ends and
    /// indefinite nonblank runs permit either parity; indefinite blank runs do
    /// not affect nonblank parity.
    fn side_nonblank_parity_masks(&self) -> (u8, u8) {
        fn span_mask(span: &Span) -> u8 {
            span.nonblank_residue(2)
                .map_or(0b11, |parity| 1_u8 << parity)
        }

        (span_mask(&self.lspan), span_mask(&self.rspan))
    }

    /// Return possible nonblank-count residues modulo 3 for the left and right
    /// spans separately. Bits 0..=2 correspond to residues 0..=2. Unknown
    /// ends and indefinite nonblank runs permit every residue.
    fn side_nonblank_mod3_masks(&self) -> (u8, u8) {
        fn span_mask(span: &Span) -> u8 {
            span.nonblank_residue(3)
                .map_or(0b111, |residue| 1_u8 << residue)
        }

        (span_mask(&self.lspan), span_mask(&self.rspan))
    }

    /// Return the possible global per-color parity vectors.  Vector bit
    /// `color - 1` is the parity of the number of cells of that nonblank color;
    /// the returned u64 is a bitset over those vectors. Unknown tape ends make
    /// every vector possible. An indefinite run makes only its own color bit
    /// unknown, preserving exact parity information for the other colors.
    #[expect(clippy::cast_possible_truncation)]
    fn color_parity_mask<const C: usize>(&self) -> u64 {
        if !WinPossible::<1, C>::color_parity_enabled() {
            return u64::MAX;
        }

        let all = WinPossible::<1, C>::all_color_parity_vectors();
        if self.lspan.end == TapeEnd::Unknown
            || self.rspan.end == TapeEnd::Unknown
        {
            return all;
        }

        const fn color_bit(color: Color) -> u8 {
            if color == 0 {
                0
            } else {
                1_u8 << (color as usize - 1)
            }
        }

        fn word_vector(word: &[Color]) -> u8 {
            word.iter()
                .fold(0_u8, |vector, &color| vector ^ color_bit(color))
        }

        fn block_deltas(block: &Block) -> u64 {
            match block {
                Block::Run { color, count } => {
                    let toggle = color_bit(*color);
                    let base = if count.minimum() & 1 == 0 {
                        0
                    } else {
                        toggle
                    };
                    let mut out = 1_u64 << base;
                    if count.parity_variable() && toggle != 0 {
                        out |= 1_u64 << (base ^ toggle);
                    }
                    out
                },
                Block::Word { word, count } => {
                    let toggle = word_vector(word);
                    let base = if count.minimum() & 1 == 0 {
                        0
                    } else {
                        toggle
                    };
                    let mut out = 1_u64 << base;
                    if count.is_indef() && toggle != 0 {
                        // The optional number of extra copies toggles the
                        // complete per-word parity vector jointly, retaining
                        // correlation between colors inside the word.
                        out |= 1_u64 << (base ^ toggle);
                    }
                    out
                },
            }
        }

        let scan_vector = color_bit(self.scan);
        let mut possible = 1_u64 << scan_vector;

        for block in
            self.lspan.span.iter().chain(self.rspan.span.iter())
        {
            let deltas = block_deltas(block);
            let mut next = 0_u64;
            let mut sources = possible;
            while sources != 0 {
                let source = sources.trailing_zeros() as u8;
                sources &= sources - 1;

                let mut choices = deltas;
                while choices != 0 {
                    let delta = choices.trailing_zeros() as u8;
                    choices &= choices - 1;
                    next |= 1_u64 << (source ^ delta);
                }
            }
            possible = next;
        }

        possible
    }

    /// Return the possible parities of the total number of nonblank cells.
    /// Bit 0 means even is possible; bit 1 means odd is possible.
    ///
    /// A fully bounded tape with fixed counts has an exact parity. Unknown
    /// ends or an indefinite block whose primitive contributes odd support
    /// permit either parity; even-support repeated words remain exact.
    fn nonblank_parity_mask(&self) -> u8 {
        let mut parity = u8::from(self.scan != 0);

        for span in [&self.lspan, &self.rspan] {
            let Some(side) = span.nonblank_residue(2) else {
                return 0b11;
            };
            parity ^= side;
        }

        1_u8 << parity
    }

    fn has_indef_word(&self) -> bool {
        [&self.lspan, &self.rspan].into_iter().any(|span| {
            span.span.iter().any(|block| {
                matches!(
                    block,
                    Block::Word { count, .. } if count.is_indef()
                )
            })
        })
    }

    fn has_stride_run(&self) -> bool {
        [&self.lspan, &self.rspan].into_iter().any(|span| {
            span.span.iter().any(|block| {
                matches!(
                    block,
                    Block::Run {
                        count: BlockCount::Stride { .. },
                        ..
                    }
                )
            })
        })
    }

    fn hash(&self) -> u64 {
        let mut h = AHasher::default();
        self.scan.hash(&mut h);
        self.lspan.hash(&mut h);
        self.rspan.hash(&mut h);
        h.finish()
    }

    /// Return the immediate left neighbor color if it is determined by
    /// this tape description. If the left side is completely unknown
    /// (`?`) and there are no explicit blocks, returns None.
    fn left_neighbor_color(&self) -> Option<Color> {
        self.lspan.span.first().map(Block::first_color).or_else(|| {
            matches!(self.lspan.end, TapeEnd::Blanks).then_some(0)
        })
    }

    /// Return the immediate right neighbor color if it is determined by
    /// this tape description. If the right side is completely unknown
    /// (`?`) and there are no explicit blocks, returns None.
    fn right_neighbor_color(&self) -> Option<Color> {
        self.rspan.span.first().map(Block::first_color).or_else(|| {
            matches!(self.rspan.end, TapeEnd::Blanks).then_some(0)
        })
    }

    /// Return the second cell away from the head when this span description
    /// determines it uniquely.  Variable length-one prefixes deliberately
    /// return None: their second cell may either remain in the first block or
    /// come from the following block/tape end.
    fn second_neighbor_color(span: &Span) -> Option<Color> {
        let Some(first) = span.span.first() else {
            return matches!(span.end, TapeEnd::Blanks).then_some(0);
        };

        let after_exact_one = || {
            span.span.iter().nth(1).map(Block::first_color).or_else(
                || matches!(span.end, TapeEnd::Blanks).then_some(0),
            )
        };

        match first {
            Block::Run { color, count } => {
                if count.minimum() >= 2 {
                    return Some(*color);
                }
                count.is_single().then(after_exact_one).flatten()
            },
            Block::Word { word, count } => {
                if word.len() >= 2 {
                    return Some(word[1]);
                }
                if count.minimum() >= 2 {
                    return Some(word[0]);
                }
                (count.exact_copies() == Some(1))
                    .then(after_exact_one)
                    .flatten()
            },
        }
    }

    fn left_second_neighbor_color(&self) -> Option<Color> {
        Self::second_neighbor_color(&self.lspan)
    }

    fn right_second_neighbor_color(&self) -> Option<Color> {
        Self::second_neighbor_color(&self.rspan)
    }

    fn is_valid_step(&self, shift: Shift, print: Color) -> bool {
        (if shift { &self.lspan } else { &self.rspan })
            .matches_color(print)
    }

    const fn is_spinout(&self, shift: Shift, read: Color) -> bool {
        if self.scan != read {
            return false;
        }

        let pull = if shift { &self.lspan } else { &self.rspan };

        pull.span.blank()
    }

    /// `AtLeast(1)` needs two predecessor branches when pulled: concrete
    /// length one disappears, while concrete length at least two leaves an
    /// `AtLeast(1)` residual. Larger lower bounds decrement without a split.
    fn pull_needs_count_one_split(&self, shift: Shift) -> bool {
        let pull = if shift { &self.lspan } else { &self.rspan };

        let Some(block) = pull.span.first() else {
            return false;
        };

        block.can_be_one()
    }

    fn backstep(
        &mut self,
        shift: Shift,
        read: Color,
    ) -> Result<(), BackwardResult> {
        let (pull, push) = if shift {
            (&mut self.lspan, &mut self.rspan)
        } else {
            (&mut self.rspan, &mut self.lspan)
        };

        pull.pull();

        push.push_single(self.scan)?;

        self.scan = read;
        Ok(())
    }

    fn push_indef(
        &mut self,
        shift: Shift,
    ) -> Result<(), BackwardResult> {
        let push = if shift {
            &mut self.rspan
        } else {
            &mut self.lspan
        };

        push.push_indef(self.scan)
    }

    /// One-sided "fresh blank" invariants.
    ///
    /// Starting from the blank tape and moving one cell at a time, visited
    /// cells form a contiguous interval.
    ///
    /// - If the program never writes blank (`0`) on an R-move, then a cell
    ///   to the **left** of the head cannot end up as `0` via being visited
    ///   (because the last visit would have to leave it behind on a Right
    ///   move). So any observed `0` on the left must be unvisited, and thus
    ///   nothing non-blank can appear farther left.
    /// - Symmetrically, if the program never writes `0` on an L-move, any
    ///   observed `0` on the right must be unvisited, so nothing non-blank
    ///   can appear farther right.
    ///
    /// This is a *sound* pruning/normalization step that rejects impossible
    /// spans and can tighten `?` ends to `0+` when an explicit `0` block is
    /// present on the applicable side.
    fn enforce_fresh_zero_side_invariants(
        &mut self,
        left_fresh_zero: bool,
        right_fresh_zero: bool,
    ) -> bool {
        fn check_side(span: &mut Span) -> bool {
            let (valid, seen_zero) = span.fresh_zero_order_valid();
            if !valid {
                return false;
            }

            if seen_zero {
                // Beyond the outermost explicit cell is certainly blank.
                span.end = TapeEnd::Blanks;
                span.absorb_trailing_blanks();
            }

            true
        }

        let sides_ok = (if left_fresh_zero {
            check_side(&mut self.lspan)
        } else {
            true
        }) && (if right_fresh_zero {
            check_side(&mut self.rspan)
        } else {
            true
        });

        if !sides_ok {
            return false;
        }

        // If blank is never written in either direction, a scanned blank is
        // being visited for the first time. The previously visited interval
        // must therefore lie wholly on one side of the head. If an explicit
        // nonblank cell identifies that side, the opposite tail is forced to
        // be blank. Explicit nonblank cells on both sides are impossible.
        if self.scan == 0 && left_fresh_zero && right_fresh_zero {
            let left_nonblank = self.lspan.explicit_nonblank();
            let right_nonblank = self.rspan.explicit_nonblank();

            match (left_nonblank, right_nonblank) {
                (true, true) => return false,
                (true, false) => self.rspan = Span::init_blank(),
                (false, true) => self.lspan = Span::init_blank(),
                (false, false) => {},
            }
        }

        true
    }

    /// Reject explicit side colors forbidden by shift-side analysis.
    ///
    /// The three-cell window filter sees only immediate neighbors.  This
    /// check carries the same per-color invariant across every explicit block
    /// in both spans, so impossible colors cannot survive farther from the
    /// head.
    fn obeys_shift_side<const C: usize>(
        &self,
        forbid_left: &[bool; C],
        forbid_right: &[bool; C],
    ) -> bool {
        self.lspan.colors_allowed(forbid_left)
            && self.rspan.colors_allowed(forbid_right)
    }

    /// Check every explicit side color and adjacent pair against a *single*
    /// compatible exact-window summary.  If one or both immediate neighbors
    /// are unknown, existentially try reachable local windows, but require the
    /// left and right whole-side constraints to be satisfied by the same
    /// window so their correlation is not joined away again at query time.
    fn obeys_state_side<const S: usize, const C: usize>(
        &self,
        state: State,
        possible: &SidePossible<S, C>,
    ) -> bool {
        struct SideRequirements<const C: usize> {
            colors: u64,
            pairs: [u64; C],
            pair_nears: u64,
            tail_any: Option<usize>,
        }

        #[expect(clippy::missing_asserts_for_indexing)]
        fn compile_span<const C: usize>(
            span: &Span,
        ) -> SideRequirements<C> {
            let mut req = SideRequirements {
                colors: 0,
                pairs: [0; C],
                pair_nears: 0,
                tail_any: None,
            };
            let mut previous = None;

            let add_pair = |req: &mut SideRequirements<C>,
                            near: usize,
                            far: usize| {
                req.pairs[near] |= 1_u64 << far;
                req.pair_nears |= 1_u64 << near;
            };

            for block in span.span.iter() {
                match block {
                    Block::Run { color, count } => {
                        let color = *color as usize;
                        req.colors |= 1_u64 << color;

                        if let Some(near) = previous {
                            add_pair(&mut req, near, color);
                        }
                        if count.minimum() > 1 {
                            add_pair(&mut req, color, color);
                        }

                        previous = Some(color);
                    },
                    Block::Word { word, count } => {
                        for &color in word.iter() {
                            req.colors |= 1_u64 << color as usize;
                        }

                        let first = word[0] as usize;
                        if let Some(near) = previous {
                            add_pair(&mut req, near, first);
                        }

                        for pair in word.windows(2) {
                            add_pair(
                                &mut req,
                                pair[0] as usize,
                                pair[1] as usize,
                            );
                        }
                        if count.minimum() > 1 {
                            add_pair(
                                &mut req,
                                *word.last().unwrap() as usize,
                                first,
                            );
                        }

                        previous = Some(*word.last().unwrap() as usize);
                    },
                }
            }

            match (&span.end, previous) {
                (TapeEnd::Blanks, Some(near)) => {
                    req.pairs[near] |= 1;
                    req.pair_nears |= 1_u64 << near;
                },
                (TapeEnd::Blanks, None) => {
                    req.pairs[0] |= 1;
                    req.pair_nears |= 1;
                },
                (TapeEnd::Unknown, Some(near)) => {
                    req.tail_any = Some(near);
                },
                (TapeEnd::Unknown, None) => {},
            }

            req
        }

        fn check_requirements<const C: usize>(
            req: &SideRequirements<C>,
            color_mask: u64,
            pair_masks: &[u64; C],
        ) -> bool {
            if req.colors & !color_mask != 0 {
                return false;
            }

            let mut nears = req.pair_nears;
            while nears != 0 {
                let near = nears.trailing_zeros() as usize;
                nears &= nears - 1;
                if req.pairs[near] & !pair_masks[near] != 0 {
                    return false;
                }
            }

            req.tail_any.is_none_or(|near| pair_masks[near] != 0)
        }

        let st = state as usize;
        let sc = self.scan as usize;

        let left_req = compile_span::<C>(&self.lspan);
        let right_req = compile_span::<C>(&self.rspan);

        let known_left = self.left_neighbor_color().map(usize::from);
        let known_right = self.right_neighbor_color().map(usize::from);

        let matches_window = |left: usize, right: usize| {
            let summary = possible.window(st, sc, left, right);
            summary.reachable
                && check_requirements(
                    &left_req,
                    summary.colors[LEFT_SIDE],
                    &summary.pairs[LEFT_SIDE],
                )
                && check_requirements(
                    &right_req,
                    summary.colors[RIGHT_SIDE],
                    &summary.pairs[RIGHT_SIDE],
                )
        };

        match (known_left, known_right) {
            (Some(left), Some(right)) => matches_window(left, right),
            (Some(left), None) => {
                (0..C).any(|right| matches_window(left, right))
            },
            (None, Some(right)) => {
                (0..C).any(|left| matches_window(left, right))
            },
            (None, None) => (0..C).any(|left| {
                (0..C).any(|right| matches_window(left, right))
            }),
        }
    }

    /// Check ordered triples forced by the explicit portions of both tape
    /// sides against one compatible exact-window triple summary.
    ///
    /// Only triples present in *every* concrete tape denoted by a backward span
    /// are required.  In particular, an `AtLeast(1)` run does not contribute a
    /// cross-block suffix pair, because its concrete length may be one or more.
    /// This keeps the filter an under-approximation of the backward
    /// requirements and therefore sound for pruning.
    #[expect(
        clippy::excessive_nesting,
        clippy::missing_asserts_for_indexing
    )]
    fn obeys_state_triples<const S: usize, const C: usize>(
        &self,
        state: State,
        possible: &SideTriplePossible<S, C>,
    ) -> bool {
        if !possible.enabled {
            return true;
        }

        struct TripleRequirements<const C: usize> {
            masks: [[u64; C]; C],
        }

        impl<const C: usize> TripleRequirements<C> {
            const fn add(&mut self, a: usize, b: usize, c: usize) {
                self.masks[a][b] |= 1_u64 << c;
            }
        }

        #[derive(Clone, Copy)]
        struct BlockBoundary {
            first: usize,
            last: usize,
            prefix2: Option<(usize, usize)>,
            suffix2: Option<(usize, usize)>,
            exactly_one_cell: bool,
        }

        fn compile_span<const C: usize>(
            span: &Span,
        ) -> TripleRequirements<C> {
            let mut req = TripleRequirements { masks: [[0; C]; C] };
            let mut previous_last: Option<usize> = None;
            let mut previous_suffix2: Option<(usize, usize)> = None;

            for block in span.span.iter() {
                let boundary = match block {
                    Block::Run { color, count } => {
                        let color = *color as usize;
                        let minimum = usize::from(count.minimum());
                        if minimum >= 3 {
                            req.add(color, color, color);
                        }

                        BlockBoundary {
                            first: color,
                            last: color,
                            prefix2: (minimum >= 2)
                                .then_some((color, color)),
                            suffix2: (minimum >= 2)
                                .then_some((color, color)),
                            exactly_one_cell: matches!(
                                *count,
                                BlockCount::Exact(1)
                            ),
                        }
                    },
                    Block::Word { word, count } => {
                        let first = word[0] as usize;
                        let last = *word.last().unwrap() as usize;
                        let minimum = count.minimum();

                        // Every concrete member contains at least `minimum`
                        // whole copies.  Three copies suffice to expose every
                        // possible length-3 window of a periodic word.
                        let copies = minimum.min(3);
                        let mut a = None;
                        let mut b = None;
                        for _ in 0..copies {
                            for &color in word.iter() {
                                let color = color as usize;
                                if let (Some(a), Some(b)) = (a, b) {
                                    req.add(a, b, color);
                                }
                                a = b;
                                b = Some(color);
                            }
                        }

                        let prefix2 = if word.len() >= 2 {
                            Some((word[0] as usize, word[1] as usize))
                        } else if minimum >= 2 {
                            Some((first, first))
                        } else {
                            None
                        };
                        let suffix2 = if word.len() >= 2 {
                            Some((
                                word[word.len() - 2] as usize,
                                word[word.len() - 1] as usize,
                            ))
                        } else if minimum >= 2 {
                            Some((last, last))
                        } else {
                            None
                        };

                        BlockBoundary {
                            first,
                            last,
                            prefix2,
                            suffix2,
                            exactly_one_cell: word.len() == 1
                                && matches!(
                                    *count,
                                    WordCount::Exact(1)
                                ),
                        }
                    },
                };

                // Triples crossing the block boundary are required only when
                // the necessary two-cell suffix/prefix is fixed for every
                // concretization of the adjacent blocks.
                if let Some((a, b)) = previous_suffix2 {
                    req.add(a, b, boundary.first);
                }
                if let (Some(a), Some((b, c))) =
                    (previous_last, boundary.prefix2)
                {
                    req.add(a, b, c);
                }

                let combined_suffix2 = boundary.suffix2.map_or_else(
                    || {
                        if boundary.exactly_one_cell {
                            previous_last.map(|a| (a, boundary.last))
                        } else {
                            None
                        }
                    },
                    Some,
                );

                previous_last = Some(boundary.last);
                previous_suffix2 = combined_suffix2;
            }

            if matches!(&span.end, &TapeEnd::Blanks) {
                // The infinite blank suffix guarantees every boundary triple
                // that can be formed from the fixed explicit suffix, plus 000.
                if let Some((a, b)) = previous_suffix2 {
                    req.add(a, b, 0);
                }
                if let Some(a) = previous_last {
                    req.add(a, 0, 0);
                }
                req.add(0, 0, 0);
            }

            req
        }

        fn side_ok<const S: usize, const C: usize>(
            req: &TripleRequirements<C>,
            possible: &SideTriplePossible<S, C>,
            st: usize,
            sc: usize,
            left: usize,
            right: usize,
            side: usize,
        ) -> bool {
            for near in 0..C {
                for middle in 0..C {
                    let required = req.masks[near][middle];
                    if required == 0 {
                        continue;
                    }
                    if required
                        & !possible.mask(
                            st, sc, left, right, side, near, middle,
                        )
                        != 0
                    {
                        return false;
                    }
                }
            }
            true
        }

        let st = state as usize;
        let sc = self.scan as usize;
        let left_req = compile_span::<C>(&self.lspan);
        let right_req = compile_span::<C>(&self.rspan);
        let known_left = self.left_neighbor_color().map(usize::from);
        let known_right = self.right_neighbor_color().map(usize::from);

        let matches_window = |left: usize, right: usize| {
            side_ok(&left_req, possible, st, sc, left, right, LEFT_SIDE)
                && side_ok(
                    &right_req, possible, st, sc, left, right,
                    RIGHT_SIDE,
                )
        };

        match (known_left, known_right) {
            (Some(left), Some(right)) => matches_window(left, right),
            (Some(left), None) => {
                (0..C).any(|right| matches_window(left, right))
            },
            (None, Some(right)) => {
                (0..C).any(|left| matches_window(left, right))
            },
            (None, None) => (0..C).any(|left| {
                (0..C).any(|right| matches_window(left, right))
            }),
        }
    }

    /// Check cross-side correlation between triples forced by the two explicit
    /// backward spans. Every required left triple must be able to co-occur with
    /// every required right triple in at least one forward execution of the
    /// same exact local window. If either side forces no triple, this reduced
    /// product contributes no extra pruning.
    #[expect(
        clippy::excessive_nesting,
        clippy::missing_asserts_for_indexing
    )]
    fn obeys_joint_state_triples<const S: usize, const C: usize>(
        &self,
        state: State,
        possible: &JointSideTriplePossible<S, C>,
    ) -> bool {
        if !possible.enabled {
            return true;
        }

        fn compile_span<const C: usize>(span: &Span) -> u64 {
            debug_assert!(C * C * C <= 64);
            let mut bits = 0_u64;
            let mut add = |a: usize, b: usize, c: usize| {
                let id = (a * C + b) * C + c;
                bits |= 1_u64 << id;
            };

            #[derive(Clone, Copy)]
            struct BlockBoundary {
                first: usize,
                last: usize,
                prefix2: Option<(usize, usize)>,
                suffix2: Option<(usize, usize)>,
                exactly_one_cell: bool,
            }

            let mut previous_last: Option<usize> = None;
            let mut previous_suffix2: Option<(usize, usize)> = None;

            for block in span.span.iter() {
                let boundary = match block {
                    Block::Run { color, count } => {
                        let color = *color as usize;
                        let minimum = usize::from(count.minimum());
                        if minimum >= 3 {
                            add(color, color, color);
                        }
                        BlockBoundary {
                            first: color,
                            last: color,
                            prefix2: (minimum >= 2)
                                .then_some((color, color)),
                            suffix2: (minimum >= 2)
                                .then_some((color, color)),
                            exactly_one_cell: matches!(
                                *count,
                                BlockCount::Exact(1)
                            ),
                        }
                    },
                    Block::Word { word, count } => {
                        let first = word[0] as usize;
                        let last = *word.last().unwrap() as usize;
                        let minimum = count.minimum();
                        let copies = minimum.min(3);
                        let mut a = None;
                        let mut b = None;
                        for _ in 0..copies {
                            for &color in word.iter() {
                                let color = color as usize;
                                if let (Some(a), Some(b)) = (a, b) {
                                    add(a, b, color);
                                }
                                a = b;
                                b = Some(color);
                            }
                        }

                        let prefix2 = if word.len() >= 2 {
                            Some((word[0] as usize, word[1] as usize))
                        } else if minimum >= 2 {
                            Some((first, first))
                        } else {
                            None
                        };
                        let suffix2 = if word.len() >= 2 {
                            Some((
                                word[word.len() - 2] as usize,
                                word[word.len() - 1] as usize,
                            ))
                        } else if minimum >= 2 {
                            Some((last, last))
                        } else {
                            None
                        };

                        BlockBoundary {
                            first,
                            last,
                            prefix2,
                            suffix2,
                            exactly_one_cell: word.len() == 1
                                && matches!(
                                    *count,
                                    WordCount::Exact(1)
                                ),
                        }
                    },
                };

                if let Some((a, b)) = previous_suffix2 {
                    add(a, b, boundary.first);
                }
                if let (Some(a), Some((b, c))) =
                    (previous_last, boundary.prefix2)
                {
                    add(a, b, c);
                }

                previous_suffix2 = boundary.suffix2.map_or_else(
                    || {
                        if boundary.exactly_one_cell {
                            previous_last.map(|a| (a, boundary.last))
                        } else {
                            None
                        }
                    },
                    Some,
                );
                previous_last = Some(boundary.last);
            }

            if matches!(&span.end, &TapeEnd::Blanks) {
                if let Some((a, b)) = previous_suffix2 {
                    add(a, b, 0);
                }
                if let Some(a) = previous_last {
                    add(a, 0, 0);
                }
                add(0, 0, 0);
            }

            bits
        }

        let left_req = compile_span::<C>(&self.lspan);
        let right_req = compile_span::<C>(&self.rspan);
        if left_req == 0 || right_req == 0 {
            return true;
        }

        let st = state as usize;
        let sc = self.scan as usize;
        let known_left = self.left_neighbor_color().map(usize::from);
        let known_right = self.right_neighbor_color().map(usize::from);

        let matches_window = |left: usize, right: usize| {
            let mut left_bits = left_req;
            while left_bits != 0 {
                let left_id = left_bits.trailing_zeros() as usize;
                left_bits &= left_bits - 1;
                let mut right_bits = right_req;
                while right_bits != 0 {
                    let right_id = right_bits.trailing_zeros() as usize;
                    right_bits &= right_bits - 1;
                    if !possible.contains_ids(
                        st, sc, left, right, left_id, right_id,
                    ) {
                        return false;
                    }
                }
            }
            true
        };

        match (known_left, known_right) {
            (Some(left), Some(right)) => matches_window(left, right),
            (Some(left), None) => {
                (0..C).any(|right| matches_window(left, right))
            },
            (None, Some(right)) => {
                (0..C).any(|left| matches_window(left, right))
            },
            (None, None) => (0..C).any(|left| {
                (0..C).any(|right| matches_window(left, right))
            }),
        }
    }

    /// Match the ordered two-run-plus-spill forward prefixes on both sides
    /// against one compatible exact local window.
    fn obeys_side_prefix_possible<const S: usize, const C: usize>(
        &self,
        state: State,
        possible: &SidePrefixPossible<S, C>,
    ) -> bool {
        let mut requirements = SideMatchRequirements::default();
        self.obeys_side_prefix_possible_cached(
            state,
            possible,
            &mut requirements,
        )
    }

    fn obeys_side_prefix_possible_cached<
        const S: usize,
        const C: usize,
    >(
        &self,
        state: State,
        possible: &SidePrefixPossible<S, C>,
        requirements: &mut SideMatchRequirements,
    ) -> bool {
        fn matches_side<const S: usize, const C: usize>(
            possible: &SidePrefixPossible<S, C>,
            st: usize,
            sc: usize,
            left: usize,
            right: usize,
            side: usize,
            reqs: RequirementSet,
        ) -> bool {
            let prefixes = possible.prefixes(st, sc, left, right, side);
            if prefixes.is_empty() {
                return false;
            }

            // Either the backward side or the forward antichain is universal.
            if reqs.unconstrained
                || possible
                    .side_unconstrained(st, sc, left, right, side)
            {
                return true;
            }

            prefixes.iter().copied().any(|prefix| {
                (0..usize::from(reqs.len)).any(|index| {
                    side_run_matches(prefix, reqs.reqs[index])
                })
            })
        }

        fn matches_word_side<const S: usize, const C: usize>(
            possible: &SidePrefixPossible<S, C>,
            st: usize,
            sc: usize,
            left: usize,
            right: usize,
            side: usize,
            req: SideWordPrefix,
        ) -> bool {
            let prefixes =
                possible.word_prefixes(st, sc, left, right, side);
            if prefixes.is_empty() {
                return false;
            }

            if req.is_unconstrained()
                || possible
                    .word_side_unconstrained(st, sc, left, right, side)
            {
                return true;
            }

            prefixes
                .iter()
                .copied()
                .any(|prefix| prefix.prefix_compatible(req))
        }

        let st = state as usize;
        let sc = self.scan as usize;
        let known_left = self.left_neighbor_color().map(usize::from);
        let known_right = self.right_neighbor_color().map(usize::from);

        let candidate_unconstrained = |left: usize, right: usize| {
            possible.side_unconstrained(st, sc, left, right, LEFT_SIDE)
                && possible
                    .side_unconstrained(st, sc, left, right, RIGHT_SIDE)
                && possible.word_side_unconstrained(
                    st, sc, left, right, LEFT_SIDE,
                )
                && possible.word_side_unconstrained(
                    st, sc, left, right, RIGHT_SIDE,
                )
        };

        // Before compiling either backward span, look for an exact reachable
        // window whose two forward side antichains are already universal. This
        // also handles unknown immediate neighbors and is common once the
        // two-run-plus-spill horizon has been crossed.
        let has_unconstrained_window = match (known_left, known_right) {
            (Some(left), Some(right)) => {
                candidate_unconstrained(left, right)
            },
            (Some(left), None) => {
                (0..C).any(|right| candidate_unconstrained(left, right))
            },
            (None, Some(right)) => {
                (0..C).any(|left| candidate_unconstrained(left, right))
            },
            (None, None) => (0..C).any(|left| {
                (0..C).any(|right| candidate_unconstrained(left, right))
            }),
        };
        if has_unconstrained_window {
            return true;
        }

        // Compile both run and word requirements at most once for this tape.
        // The same bundle is reused by the joint run/word checks in the hot
        // backward stepping path.
        let [left_req, right_req] = requirements.runs(self);
        let [left_word_req, right_word_req] = requirements.words(self);

        let matches_window = |left: usize, right: usize| {
            matches_side(
                possible, st, sc, left, right, LEFT_SIDE, left_req,
            ) && matches_side(
                possible, st, sc, left, right, RIGHT_SIDE, right_req,
            ) && matches_word_side(
                possible,
                st,
                sc,
                left,
                right,
                LEFT_SIDE,
                left_word_req,
            ) && matches_word_side(
                possible,
                st,
                sc,
                left,
                right,
                RIGHT_SIDE,
                right_word_req,
            )
        };

        match (known_left, known_right) {
            (Some(left), Some(right)) => matches_window(left, right),
            (Some(left), None) => {
                (0..C).any(|right| matches_window(left, right))
            },
            (None, Some(right)) => {
                (0..C).any(|left| matches_window(left, right))
            },
            (None, None) => (0..C).any(|left| {
                (0..C).any(|right| matches_window(left, right))
            }),
        }
    }

    /// Match both ordered word-prefix requirements against one joint forward
    /// witness. This rejects cross-side joins that pass the two independent
    /// `word_windows` projections separately.
    fn obeys_joint_side_word_prefix_possible<
        const S: usize,
        const C: usize,
    >(
        &self,
        state: State,
        possible: &JointSideWordPrefixPossible<S, C>,
    ) -> bool {
        let mut requirements = SideMatchRequirements::default();
        self.obeys_joint_side_word_prefix_possible_cached(
            state,
            possible,
            &mut requirements,
        )
    }

    fn obeys_joint_side_word_prefix_possible_cached<
        const S: usize,
        const C: usize,
    >(
        &self,
        state: State,
        possible: &JointSideWordPrefixPossible<S, C>,
        requirements: &mut SideMatchRequirements,
    ) -> bool {
        let st = state as usize;
        let sc = self.scan as usize;
        let known_left = self.left_neighbor_color().map(usize::from);
        let known_right = self.right_neighbor_color().map(usize::from);

        let mut matches_window = |left: usize, right: usize| {
            possible.window(st, sc, left, right).iter().copied().any(
                |prefix| match prefix {
                    JointSideWordPrefix::Unknown => true,
                    JointSideWordPrefix::Specific { left, right } => {
                        let [left_req, right_req] =
                            requirements.words(self);
                        left.prefix_compatible(left_req)
                            && right.prefix_compatible(right_req)
                    },
                },
            )
        };

        match (known_left, known_right) {
            (Some(left), Some(right)) => matches_window(left, right),
            (Some(left), None) => {
                (0..C).any(|right| matches_window(left, right))
            },
            (None, Some(right)) => {
                (0..C).any(|left| matches_window(left, right))
            },
            (None, None) => (0..C).any(|left| {
                (0..C).any(|right| matches_window(left, right))
            }),
        }
    }

    /// Match both run requirements against the same forward alternative.
    fn obeys_joint_side_prefix_possible<
        const S: usize,
        const C: usize,
    >(
        &self,
        state: State,
        possible: Option<&JointSidePrefixPossible<S, C>>,
    ) -> bool {
        let mut requirements = SideMatchRequirements::default();
        self.obeys_joint_side_prefix_possible_cached(
            state,
            possible,
            &mut requirements,
        )
    }

    #[expect(clippy::excessive_nesting)]
    fn obeys_joint_side_prefix_possible_cached<
        const S: usize,
        const C: usize,
    >(
        &self,
        state: State,
        possible: Option<&JointSidePrefixPossible<S, C>>,
        requirements: &mut SideMatchRequirements,
    ) -> bool {
        let Some(possible) = possible else {
            return true;
        };

        let matches =
            |prefix, req: &RequirementSet, far_colors: u64| {
                req.unconstrained
                    || (0..usize::from(req.len)).any(|index| {
                        side_run_matches_with_far_colors(
                            prefix,
                            req.reqs[index],
                            Some(far_colors),
                        )
                    })
            };

        let st = state as usize;
        let sc = self.scan as usize;
        let known_left = self.left_neighbor_color().map(usize::from);
        let known_right = self.right_neighbor_color().map(usize::from);

        // Compute the backward whole-side parity only if a shape-compatible
        // forward alternative actually has a restrictive parity relation.
        // Many buckets have already widened to all four pairs, and shape-only
        // rejection should not pay for another full scan of the tape spans.
        let mut required_parity = None;
        let mut run_requirements: Option<[RequirementSet; 2]> = None;
        let mut matches_window = |left: usize, right: usize| {
            possible.window(st, sc, left, right).iter().copied().any(
                |prefix| {
                    let shape_matches = match prefix {
                        JointSidePrefix::Unknown { .. } => true,
                        JointSidePrefix::Specific {
                            left,
                            right,
                            far_colors,
                            ..
                        } => {
                            let reqs = run_requirements
                                .get_or_insert_with(|| {
                                    requirements.runs(self)
                                });
                            matches(
                                left,
                                &reqs[LEFT_SIDE],
                                far_colors[LEFT_SIDE],
                            ) && matches(
                                right,
                                &reqs[RIGHT_SIDE],
                                far_colors[RIGHT_SIDE],
                            )
                        },
                    };
                    if !shape_matches {
                        return false;
                    }

                    let parity_mask = prefix.parity_mask();
                    if !possible.track_parity || parity_mask == 0b1111 {
                        return true;
                    }

                    let required =
                        *required_parity.get_or_insert_with(|| {
                            let (left_mask, right_mask) =
                                self.side_nonblank_parity_masks();
                            let mut pairs = 0_u8;
                            for lp in 0..2 {
                                if left_mask & (1_u8 << lp) == 0 {
                                    continue;
                                }
                                for rp in 0..2 {
                                    if right_mask & (1_u8 << rp) == 0 {
                                        continue;
                                    }
                                    pairs |= 1_u8 << (lp | (rp << 1));
                                }
                            }
                            pairs
                        });
                    parity_mask & required != 0
                },
            )
        };

        match (known_left, known_right) {
            (Some(left), Some(right)) => matches_window(left, right),
            (Some(left), None) => {
                (0..C).any(|right| matches_window(left, right))
            },
            (None, Some(right)) => {
                (0..C).any(|left| matches_window(left, right))
            },
            (None, None) => (0..C).any(|left| {
                (0..C).any(|right| matches_window(left, right))
            }),
        }
    }

    /// Match the two sides' first two cells beyond the immediate neighbors
    /// against one joint forward witness. Unknown positions in the backward
    /// tape are left unconstrained; exact/guaranteed positions must agree.
    fn obeys_joint_short_possible<const S: usize, const C: usize>(
        &self,
        state: State,
        possible: &JointShortPossible<S, C>,
    ) -> bool {
        #[derive(Clone, Copy)]
        struct Requirement {
            cells: [Option<Color>; JOINT_SHORT_DEPTH],
        }

        impl Requirement {
            const fn unconstrained(self) -> bool {
                self.cells[0].is_none() && self.cells[1].is_none()
            }
        }

        fn requirement(span: &Span) -> Requirement {
            let mut cells = [None; JOINT_SHORT_DEPTH];
            let mut len = 0_usize;
            let mut skip = 1_usize;

            const fn append(
                cells: &mut [Option<Color>; JOINT_SHORT_DEPTH],
                len: &mut usize,
                color: Color,
            ) {
                if *len < JOINT_SHORT_DEPTH {
                    cells[*len] = Some(color);
                    *len += 1;
                }
            }

            for block in span.span.iter() {
                if len == JOINT_SHORT_DEPTH {
                    break;
                }

                match block {
                    Block::Run { color, count } => {
                        let minimum = usize::from(count.minimum());
                        let start = skip.min(minimum);
                        skip -= start;
                        let guaranteed = minimum - start;
                        for _ in 0..guaranteed {
                            if len == JOINT_SHORT_DEPTH {
                                break;
                            }
                            append(&mut cells, &mut len, *color);
                        }
                        if count.is_indef() && len < JOINT_SHORT_DEPTH {
                            return Requirement { cells };
                        }
                    },
                    Block::Word { word, count } => {
                        let width = word.len();
                        let minimum =
                            count.minimum().saturating_mul(width);
                        let start = skip.min(minimum);
                        skip -= start;
                        for offset in start..minimum {
                            if len == JOINT_SHORT_DEPTH {
                                break;
                            }
                            append(
                                &mut cells,
                                &mut len,
                                word[offset % width],
                            );
                        }
                        if count.is_indef() && len < JOINT_SHORT_DEPTH {
                            return Requirement { cells };
                        }
                    },
                }
            }

            if len < JOINT_SHORT_DEPTH {
                match span.end {
                    TapeEnd::Blanks => {
                        // If the immediate neighbor itself came from the blank
                        // end, consume that skipped zero first; every farther
                        // position is also exact zero.
                        while len < JOINT_SHORT_DEPTH {
                            append(&mut cells, &mut len, 0);
                        }
                    },
                    TapeEnd::Unknown => {},
                }
            }

            Requirement { cells }
        }

        fn side_matches(
            side: JointShortSide,
            req: Requirement,
        ) -> bool {
            for index in 0..JOINT_SHORT_DEPTH {
                if let Some(required) = req.cells[index]
                    && let Some(actual) = side.cell(index)
                    && actual != required
                {
                    return false;
                }
            }
            true
        }

        let left_req = requirement(&self.lspan);
        let right_req = requirement(&self.rspan);
        if left_req.unconstrained() && right_req.unconstrained() {
            return true;
        }

        let st = state as usize;
        let sc = self.scan as usize;
        let known_left = self.left_neighbor_color().map(usize::from);
        let known_right = self.right_neighbor_color().map(usize::from);

        let matches_window = |left: usize, right: usize| {
            possible.window(st, sc, left, right).iter().copied().any(
                |prefix| {
                    side_matches(prefix.left, left_req)
                        && side_matches(prefix.right, right_req)
                },
            )
        };

        match (known_left, known_right) {
            (Some(left), Some(right)) => matches_window(left, right),
            (Some(left), None) => {
                (0..C).any(|right| matches_window(left, right))
            },
            (None, Some(right)) => {
                (0..C).any(|left| matches_window(left, right))
            },
            (None, None) => (0..C).any(|left| {
                (0..C).any(|right| matches_window(left, right))
            }),
        }
    }

    /// Enforce sides proved to contain blanks only.
    ///
    /// Merely changing an unknown end to `0+` is insufficient when an
    /// explicit nonblank block is already present.  Such a tape contradicts
    /// the invariant and must be rejected.  Otherwise every explicit zero is
    /// redundant and the whole side can be canonicalized to a blank span.
    fn tighten_forced_blank_ends(
        &mut self,
        left_forced_blank: bool,
        right_forced_blank: bool,
    ) -> bool {
        fn force_blank(span: &mut Span) -> bool {
            if span.explicit_nonblank() {
                return false;
            }

            *span = Span::init_blank();
            true
        }

        (!left_forced_blank || force_blank(&mut self.lspan))
            && (!right_forced_blank || force_blank(&mut self.rspan))
    }
}

/**************************************/

#[cfg(test)]
impl From<&str> for Block {
    fn from(s: &str) -> Self {
        if let Some(body) = s.strip_suffix("..") {
            if let Some((color, count)) = body.split_once('^') {
                return Self::at_least(
                    color.parse().unwrap(),
                    count.parse().unwrap(),
                );
            }

            return Self::at_least(body.parse().unwrap(), 1);
        }

        if let Some((color, count)) = s.split_once('^') {
            return Self::exact(
                color.parse().unwrap(),
                count.parse().unwrap(),
            );
        }

        Self::exact(s.parse().unwrap(), 1)
    }
}

#[cfg(test)]
impl Span {
    fn new(end: &str, blocks: Vec<Block>) -> Self {
        let mut span = (match end {
            "0+" => Self::init_blank,
            "?" => Self::init_unknown,
            _ => unreachable!(),
        })();

        for block in blocks {
            span.span.push_block(&block).unwrap();
        }

        span
    }
}

#[cfg(test)]
impl From<&str> for Tape {
    fn from(s: &str) -> Self {
        let parts: Vec<&str> = s.split_whitespace().collect();

        let l_end = parts[0];

        assert!(matches!(l_end, "?" | "0+"));

        let l_blocks: Vec<Block> = parts[1..]
            .iter()
            .take_while(|p| !p.starts_with('['))
            .map(|&p| p.into())
            .collect::<Vec<_>>()
            .into_iter()
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

        let r_end = *parts.last().unwrap();

        assert!(matches!(l_end, "?" | "0+"));

        let r_blocks: Vec<Block> = parts[rspan_start..parts.len() - 1]
            .iter()
            .map(|&p| p.into())
            .rev()
            .collect();

        Self {
            scan,
            lspan: Span::new(l_end, l_blocks),
            rspan: Span::new(r_end, r_blocks),
        }
    }
}

/**************************************/

#[cfg(test)]
impl Tape {
    #[track_caller]
    fn assert(&self, exp: &str) {
        assert_eq!(self.to_string(), exp);
    }

    #[track_caller]
    fn tbackstep(
        &mut self,
        shift: u8,
        print: Color,
        read: Color,
        success: bool,
    ) {
        assert!(matches!(shift, 0 | 1));

        let shift = shift != 0;

        let step = self.is_valid_step(shift, print);

        assert_eq!(step, success);

        if !step {
            return;
        }

        self.backstep(shift, read).unwrap();
    }
}

#[test]
fn test_backstep_halt() {
    let mut tape = Tape::init_halt(2);

    tape.assert("? [2] ?");

    tape.tbackstep(0, 2, 1, true);

    tape.assert("? 2 [1] ?");

    tape.tbackstep(1, 1, 2, false);

    tape.assert("? 2 [1] ?");

    tape.tbackstep(1, 2, 0, true);

    tape.assert("? [0] 1 ?");

    tape.tbackstep(1, 0, 2, true);

    tape.assert("? [2] 0 1 ?");
}

#[test]
fn test_backstep_blank() {
    let mut tape = Tape::init_blank(2);

    tape.assert("0+ [2] 0+");

    tape.tbackstep(0, 1, 1, false);
    tape.tbackstep(0, 2, 1, false);
    tape.tbackstep(0, 0, 1, true);

    tape.assert("0+ 2 [1] 0+");

    tape.tbackstep(1, 0, 0, false);
    tape.tbackstep(1, 1, 0, false);
    tape.tbackstep(1, 2, 0, true);

    tape.assert("0+ [0] 1 0+");

    tape.tbackstep(1, 1, 0, false);
    tape.tbackstep(1, 2, 0, false);
    tape.tbackstep(1, 0, 0, true);

    tape.assert("0+ [0] 0 1 0+");
}

#[test]
fn test_backstep_spinout() {
    let mut tape = Tape::init_spinout(true);

    tape.assert("? [0] 0+");

    tape.tbackstep(0, 1, 1, false);
    tape.tbackstep(0, 2, 1, false);
    tape.tbackstep(0, 0, 1, true);

    tape.assert("? 0 [1] 0+");

    tape.tbackstep(0, 1, 2, false);
    tape.tbackstep(0, 2, 2, false);
    tape.tbackstep(0, 0, 2, true);

    tape.assert("? 0 1 [2] 0+");

    tape.tbackstep(1, 1, 2, true);
    tape.tbackstep(1, 0, 1, true);
    tape.tbackstep(1, 0, 0, true);
    tape.tbackstep(1, 0, 0, true);

    tape.assert("? [0] 0 1 2^2 0+");
}

#[test]
fn test_backstep_required() {
    let mut tape: Tape = "0+ [1] 1 0 ?".into();

    tape.assert("0+ [1] 1 0 ?");

    tape.tbackstep(0, 1, 0, true);

    tape.assert("0+ 1 [0] 0 ?");
}

#[test]
fn test_spinout() {
    let mut tape: Tape = "0+ [1] 0^2 ?".into();

    tape.assert("0+ [1] 0^2 ?");

    assert!(!tape.is_valid_step(false, 1));
    assert!(tape.is_spinout(true, 1));

    tape.push_indef(true).unwrap();

    tape.assert("0+ [1] 1.. 0^2 ?");

    assert!(!tape.is_spinout(false, 1));
    assert!(tape.is_spinout(true, 1));
}

#[test]
fn test_parse() {
    let tapes = [
        "? 2 1^2 [5] 3^3 0+",
        "0+ 2 1^2 [5] 3^3 ?",
        "0+ 2 1^2 [5] 3^3 0+",
        "? 2 3^11 4 1^11 [0] ?",
        "? 2 3^11 4 1^11 [0] 0+",
        "0+ 2 3^11 4 1^11 [0] ?",
        "? 4^118 [4] 5^2 2 4 5^7 1 0+",
        "? 4^118 [4] 5^2 2 4 5^7 1 0+",
        "0+ 4^118 [4] 5^2 2 4 5^7 1 0+",
    ];

    for tape in tapes {
        Into::<Tape>::into(tape).assert(tape);
    }
}

#[test]
fn test_backstep_indef() {
    let mut tape: Tape = "0+ [1] 1.. 0^2 ?".into();

    tape.backstep(false, 1).unwrap();

    tape.assert("0+ 1 [1] 1.. 0^2 ?");
}

#[test]
fn test_push_indef() {
    let mut tape: Tape = "0+ 1 [0] ?".into();

    tape.push_indef(false).unwrap();

    tape.assert("0+ 1 0.. [0] ?");

    tape.assert("0+ 1 0.. [0] ?");

    tape.scan = 1;
    tape.push_indef(false).unwrap();

    tape.assert("0+ 1 0.. 1.. [1] ?");

    tape.scan = 0;
    tape.push_indef(false).unwrap();

    tape.assert("0+ 1 0.. 1.. 0.. [0] ?");

    tape.backstep(false, 0).unwrap();

    tape.assert("0+ 1 0.. 1.. 0^2.. [0] ?");
}

#[test]
fn test_count_limit() {
    let mut exact: Tape = "? 1^255 [1] 0 ?".into();
    assert!(matches!(exact.backstep(false, 0), Err(CountLimit)));

    let config = Config::new(0, "? 1^255.. [1] ?".into());
    let diff = Entries::new();
    let same = Entries::new();
    assert!(matches!(
        get_indef(false, &config, &diff, &same),
        Err(CountLimit)
    ));
}

/**************************************/

use core::array::from_fn;
use std::collections::VecDeque;

type Adj<const S: usize> = [Vec<usize>; S];
type Preds<const S: usize> = [[Vec<usize>; 2]; S]; // preds[v][dir] -> u
type Writers<const C: usize> = [[Vec<usize>; 2]; C]; // writers[color][dir] -> v
type NextDir<const S: usize> = [[Vec<usize>; 2]; S]; // next[u][dir] -> v
type Indices<const S: usize, const C: usize> =
    (Adj<S>, Preds<S>, Writers<C>, NextDir<S>);

fn indices_new<const S: usize, const C: usize>() -> Indices<S, C> {
    (
        from_fn(|_| vec![]),
        from_fn(|_| from_fn(|_| vec![])),
        from_fn(|_| from_fn(|_| vec![])),
        from_fn(|_| from_fn(|_| vec![])),
    )
}

fn indices_add<const S: usize, const C: usize>(
    (adj, preds, writers, next): &mut Indices<S, C>,
    st: State,
    tr: State,
    sh: Shift,
    pr: Color,
) {
    let (st, tr, sh, pr) =
        (st as usize, tr as usize, usize::from(sh), pr as usize);

    adj[st].push(tr);
    preds[tr][sh].push(st);
    writers[pr][sh].push(tr);
    next[st][sh].push(tr);
}

fn indices_finalize<const S: usize, const C: usize>(
    (adj, preds, writers, next): &mut Indices<S, C>,
) {
    for u in 0..S {
        adj[u].sort_unstable();
        adj[u].dedup();
        for d in 0..2 {
            preds[u][d].sort_unstable();
            preds[u][d].dedup();
            next[u][d].sort_unstable();
            next[u][d].dedup();
        }
    }
    for co in 0..C {
        for d in 0..2 {
            writers[co][d].sort_unstable();
            writers[co][d].dedup();
        }
    }
}

const fn gcd_i32(mut a: i32, mut b: i32) -> i32 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}

fn reachability<const S: usize>(adj: &Adj<S>) -> [[bool; S]; S] {
    let mut reach = [[false; S]; S];

    for start in 0..S {
        let mut q = VecDeque::new();
        reach[start][start] = true;
        q.push_back(start);

        while let Some(u) = q.pop_front() {
            for &v in &adj[u] {
                if !reach[start][v] {
                    reach[start][v] = true;
                    q.push_back(v);
                }
            }
        }
    }

    reach
}

/// Color-aware one-sided excursion summary with exact parent/back color.
///
/// `ret[back][st][co]` is a bitmask of states that can be entered by a
/// balanced one-sided computation starting in exact `(st, co)` with immediate
/// parent/back color `back` and finishing by moving back across that boundary.
/// With `clean == true`, every matched pop writes 0, so all cells touched on
/// that side are restored to blank recursively.
///
/// A return has a direct recursive form, so we do not need the old all-pairs
/// `same` transitive closure.  If the first move pops, it returns immediately.
/// If the first move pushes, the child must itself return; after that return we
/// are back on the current cell in the returned state scanning exactly the
/// color printed by the push, and continue from there.  Saturating those
/// return-state masks computes the same grammar with far less work.
struct SideExcursions<const S: usize, const C: usize> {
    // Aggregate flattened [back][state][color]. Bit `tr` means some exact
    // front color admits a balanced return into state `tr`.
    ret: Vec<u64>,

    // Aggregate ordinary-excursion final boundary colors, flattened
    // [back][state][color][return_state] -> final pop-color mask.
    // Clean excursions do not need this aggregate outside their fixed point.
    pop: Option<Vec<u64>>,

    // Strong ordinary relation retaining the exact child/front color during
    // recursive composition. Flattened
    // [back][state][scan][front][return_state].
    //
    // For alphabets with C <= 8, each u64 is a bitset of
    // `(final pop color, final front color)` pairs, encoded as
    // `pop * C + final_front`.  This lets the crossing reduced product keep
    // the second cell on the excursion side exact.  For larger alphabets the
    // same storage falls back to the original final-pop-color mask.
    //
    // This is kept only for ordinary excursions.
    exact_pop: Option<Vec<u64>>,
}

impl<const S: usize, const C: usize> SideExcursions<S, C> {
    const fn node(st: usize, co: usize) -> usize {
        st * C + co
    }

    const fn decode(node: usize) -> (usize, usize) {
        (node / C, node % C)
    }

    const fn ret_index(back: usize, st: usize, co: usize) -> usize {
        (back * S + st) * C + co
    }

    const fn exact_node_index(
        back: usize,
        st: usize,
        co: usize,
        front: usize,
    ) -> usize {
        (((back * S + st) * C + co) * C) + front
    }

    fn ret_states(&self, back: usize, st: usize, co: usize) -> u64 {
        self.ret[Self::ret_index(back, st, co)]
    }

    fn ret_states_from_mask(
        &self,
        back: usize,
        st: usize,
        mut colors: u64,
    ) -> u64 {
        let mut out = 0;
        while colors != 0 {
            let co = colors.trailing_zeros() as usize;
            colors &= colors - 1;
            out |= self.ret_states(back, st, co);
        }
        out
    }

    fn ret_from_mask_possible(
        &self,
        back: usize,
        st: usize,
        colors: u64,
        tr: usize,
    ) -> bool {
        (self.ret_states_from_mask(back, st, colors) & (1_u64 << tr))
            != 0
    }

    fn pop_colors(
        &self,
        back: usize,
        st: usize,
        co: usize,
        tr: usize,
    ) -> u64 {
        let node = Self::ret_index(back, st, co);
        self.pop.as_ref().map_or(0, |pop| pop[node * S + tr])
    }

    fn exact_pop_colors(
        &self,
        back: usize,
        st: usize,
        co: usize,
        front: usize,
        tr: usize,
    ) -> u64 {
        let node = Self::exact_node_index(back, st, co, front);
        let outcomes =
            self.exact_pop.as_ref().map_or(0, |pop| pop[node * S + tr]);

        Self::outcome_pop_colors(outcomes)
    }

    #[expect(clippy::disallowed_names)]
    const fn outcome_pop_colors(outcomes: u64) -> u64 {
        if C > 8 {
            return outcomes;
        }

        // Pair bits are laid out in C-bit rows, one row per pop color.
        // Testing one row at a time is bounded by C <= 8 and is substantially
        // cheaper than walking every set `(pop, final_front)` bit when an
        // excursion outcome is dense.
        let row = (1_u64 << C) - 1;
        let mut colors = 0_u64;
        let mut pop = 0_usize;
        while pop < C {
            if outcomes & (row << (pop * C)) != 0 {
                colors |= 1_u64 << pop;
            }
            pop += 1;
        }
        colors
    }

    fn exact_pop_front_pairs(
        &self,
        back: usize,
        st: usize,
        co: usize,
        front: usize,
        tr: usize,
    ) -> u64 {
        if C > 8 {
            return 0;
        }

        let node = Self::exact_node_index(back, st, co, front);
        self.exact_pop.as_ref().map_or(0, |pop| pop[node * S + tr])
    }
}

/// Exact possible color mask of the child-side neighbor when the source's
/// parent/back neighbor is known.
fn window_child_mask<const S: usize, const C: usize>(
    state: usize,
    scan: usize,
    push: Shift,
    back: usize,
    possible: &WinPossible<S, C>,
) -> u64 {
    if push {
        possible.right[state][scan][back]
    } else {
        possible.left[state][scan][back]
    }
}

/// Saturate one-sided balanced returns while keeping the exact child/front
/// color correlated through every recursive composition.
///
/// An exact node is `(back, state, scan, front)`, where `back` is the parent
/// cell and `front` is the immediate neighbor deeper into the excursion side.
/// For ordinary (`clean == false`) excursions with C <= 8, an outcome
/// `(return_state, pop_color, final_front_color)` means the computation
/// eventually moves back onto the parent in
/// `return_state`, leaving `pop_color` on this node's scanned cell and
/// `final_front_color` on its immediate child/front cell.  When a nested child
/// returns, its `pop_color` becomes the exact `front` color of the same-level
/// continuation.  The nested child's farther result is deliberately joined
/// there; retaining the current node's final front is nevertheless enough for
/// the radius-2 same-cell crossing product.  Clean excursions and larger
/// alphabets keep the cheaper pop-color-only encoding.
fn side_excursions<const S: usize, const C: usize>(
    prog: &Prog<S, C>,
    windows: &WinPossible<S, C>,
    push: Shift,
    clean: bool,
) -> SideExcursions<S, C> {
    struct PushEq {
        source: usize,
        back: usize,
        print: usize,
    }

    fn add_outcome<const S: usize>(
        outcomes: &mut [u64],
        q: &mut VecDeque<(usize, usize, u64)>,
        node: usize,
        tr: usize,
        colors: u64,
    ) {
        let index = node * S + tr;
        let added = colors & !outcomes[index];
        if added != 0 {
            outcomes[index] |= added;
            q.push_back((node, tr, added));
        }
    }

    fn encode_outcome<const C: usize>(
        pop: usize,
        front: usize,
        keep_front: bool,
    ) -> u64 {
        if keep_front {
            1_u64 << (pop * C + front)
        } else {
            1_u64 << pop
        }
    }

    fn outcome_pop_colors<const S: usize, const C: usize>(
        outcomes: u64,
        keep_front: bool,
    ) -> u64 {
        if keep_front {
            SideExcursions::<S, C>::outcome_pop_colors(outcomes)
        } else {
            outcomes
        }
    }

    let keep_front = !clean && C <= 8;
    let pop_shift = !push;
    let exact_nodes = C * S * C * C;
    let mut outcomes = vec![0_u64; exact_nodes * S];
    let mut trans = [[None; C]; S];

    for ((st, co), &(print, shift, tr)) in prog.iter() {
        trans[st as usize][co as usize] =
            Some((print as usize, shift, tr as usize));
    }

    // For fixed `push`, this maps [state][scan][exact back] to the exact
    // possible front colors of the local window.
    let fronts = if push { &windows.right } else { &windows.left };

    let mut pushes = Vec::new();
    let mut child_users =
        (0..exact_nodes).map(|_| Vec::new()).collect::<Vec<_>>();
    let mut continuation_users =
        (0..exact_nodes).map(|_| Vec::new()).collect::<Vec<_>>();
    let mut registered_continuations = Vec::<Vec<usize>>::new();
    let mut q = VecDeque::new();

    for back in 0..C {
        for st in 0..S {
            for co in 0..C {
                let mut front_colors = fronts[st][co][back];
                while front_colors != 0 {
                    let front = front_colors.trailing_zeros() as usize;
                    front_colors &= front_colors - 1;

                    let source =
                        SideExcursions::<S, C>::exact_node_index(
                            back, st, co, front,
                        );
                    let Some((print, shift, tr)) = trans[st][co] else {
                        continue;
                    };

                    if shift == pop_shift {
                        if !clean || print == 0 {
                            add_outcome::<S>(
                                &mut outcomes,
                                &mut q,
                                source,
                                tr,
                                encode_outcome::<C>(
                                    print, front, keep_front,
                                ),
                            );
                        }
                        continue;
                    }

                    // Push deeper.  The old exact `front` becomes the child's
                    // scanned color; enumerate only exact farther-front colors
                    // admitted by the current window relation.
                    let child_fronts = fronts[tr][front][print];
                    if child_fronts == 0 {
                        continue;
                    }

                    let eq_i = pushes.len();
                    pushes.push(PushEq {
                        source,
                        back,
                        print,
                    });
                    registered_continuations.push(Vec::new());

                    let mut child_fronts = child_fronts;
                    while child_fronts != 0 {
                        let child_front =
                            child_fronts.trailing_zeros() as usize;
                        child_fronts &= child_fronts - 1;
                        let child =
                            SideExcursions::<S, C>::exact_node_index(
                                print,
                                tr,
                                front,
                                child_front,
                            );
                        child_users[child].push(eq_i);
                    }
                }
            }
        }
    }

    while let Some((node, return_st, added_colors)) = q.pop_front() {
        // Newly completed nested-child outcomes reveal exact same-level
        // continuation nodes: the child's final pop color is precisely the
        // continuation's new `front` color.
        for &eq_i in &child_users[node] {
            let source = pushes[eq_i].source;
            let back = pushes[eq_i].back;
            let print = pushes[eq_i].print;

            let mut colors =
                outcome_pop_colors::<S, C>(added_colors, keep_front);
            while colors != 0 {
                let child_pop = colors.trailing_zeros() as usize;
                colors &= colors - 1;
                let continuation =
                    SideExcursions::<S, C>::exact_node_index(
                        back, return_st, print, child_pop,
                    );

                // The same continuation can be discovered through several
                // exact child-front witnesses. Register it only once per
                // equation to keep the reverse worklist small.
                if registered_continuations[eq_i]
                    .contains(&continuation)
                {
                    continue;
                }
                registered_continuations[eq_i].push(continuation);
                continuation_users[continuation].push(eq_i);

                for tr in 0..S {
                    let current = outcomes[continuation * S + tr];
                    if current != 0 {
                        add_outcome::<S>(
                            &mut outcomes,
                            &mut q,
                            source,
                            tr,
                            current,
                        );
                    }
                }
            }
        }

        // If this exact node is already registered as a same-level
        // continuation, every new outcome is inherited by its outer source.
        for &eq_i in &continuation_users[node] {
            add_outcome::<S>(
                &mut outcomes,
                &mut q,
                pushes[eq_i].source,
                return_st,
                added_colors,
            );
        }
    }

    // Build the old aggregate API only after the stronger exact-front fixed
    // point is complete.  Existing halfblank/frontier/halt users therefore
    // get the stronger relation without needing to carry another index.
    let ret_len = C * S * C;
    let mut ret = vec![0_u64; ret_len];
    let mut pop = (!clean).then(|| vec![0_u64; ret_len * S]);

    for back in 0..C {
        for st in 0..S {
            for co in 0..C {
                let aggregate =
                    SideExcursions::<S, C>::ret_index(back, st, co);
                let mut front_colors = fronts[st][co][back];
                while front_colors != 0 {
                    let front = front_colors.trailing_zeros() as usize;
                    front_colors &= front_colors - 1;
                    let node = SideExcursions::<S, C>::exact_node_index(
                        back, st, co, front,
                    );

                    for tr in 0..S {
                        let outcomes = outcomes[node * S + tr];
                        if outcomes == 0 {
                            continue;
                        }
                        ret[aggregate] |= 1_u64 << tr;
                        if let Some(pop) = &mut pop {
                            pop[aggregate * S + tr] |=
                                outcome_pop_colors::<S, C>(
                                    outcomes, keep_front,
                                );
                        }
                    }
                }
            }
        }
    }

    SideExcursions {
        ret,
        pop,
        exact_pop: (!clean).then_some(outcomes),
    }
}

/// Sound over-approximation of reachable one-sided-blank configurations.
///
/// `blank_side == false` describes `0+ [color] ?`.
/// `blank_side == true`  describes `? [color] 0+`.
///
/// The worklist keeps the blank side clean at abstract checkpoints, but may
/// cross through dirty intermediate configurations via `clean` excursions.
/// The unconstrained side may use arbitrary balanced excursions.
fn halfblank_slots<const S: usize, const C: usize>(
    prog: &Prog<S, C>,
    windows: &WinPossible<S, C>,
    blank_side: Shift,
    clean: &SideExcursions<S, C>,
    away: &SideExcursions<S, C>,
) -> [[u64; C]; S] {
    debug_assert!(away.pop.is_some());

    let mut possible = [[0_u64; C]; S];
    let mut trans = [[None; C]; S];

    for ((st, co), &(print, shift, tr)) in prog.iter() {
        trans[st as usize][co as usize] =
            Some((print as usize, shift, tr as usize));
    }

    let mut q = VecDeque::new();
    let away_side = !blank_side;

    let push = |st: usize,
                co: usize,
                near: usize,
                possible: &mut [[u64; C]; S],
                q: &mut VecDeque<(usize, usize)>| {
        // Exact halfblank checkpoint:
        //   left blank:  0 [scan] near
        //   right blank: near [scan] 0
        // Keep only exact local windows admitted by the forward abstraction.
        let child_colors =
            window_child_mask(st, co, away_side, 0, windows);
        let bit = 1_u64 << near;
        if child_colors & bit != 0 && possible[st][co] & bit == 0 {
            possible[st][co] |= bit;
            q.push_back((SideExcursions::<S, C>::node(st, co), near));
        }
    };

    // The true blank initial configuration has exact zero on both sides.
    push(0, 0, 0, &mut possible, &mut q);

    while let Some((node, near)) = q.pop_front() {
        let (st, co) = SideExcursions::<S, C>::decode(node);

        let Some((print, shift, tr)) = trans[st][co] else {
            continue;
        };

        if shift == away_side {
            // A complete arbitrary excursion into the unconstrained side can
            // return to this same boundary.  Because `near` is exact, start
            // the excursion in that exact child color.  Its final pop color
            // is the new exact inward-neighbor color at the returned
            // halfblank checkpoint.
            let mut return_states = away.ret_states(print, tr, near);
            while return_states != 0 {
                let return_st = return_states.trailing_zeros() as usize;
                return_states &= return_states - 1;

                let mut pop_colors =
                    away.pop_colors(print, tr, near, return_st);
                while pop_colors != 0 {
                    let pop_color =
                        pop_colors.trailing_zeros() as usize;
                    pop_colors &= pop_colors - 1;
                    push(
                        return_st,
                        print,
                        pop_color,
                        &mut possible,
                        &mut q,
                    );
                }
            }
        }

        if shift == blank_side {
            // Move into the blank side.  The source checkpoint already proves
            // that exact neighbor is zero.  The old head joins the opposite
            // side, so the transition's print becomes the new exact inward
            // neighbor.
            push(tr, 0, print, &mut possible, &mut q);

            // Or make a complete clean excursion into the blank side and
            // return to the original boundary.  The unconstrained side is
            // untouched by that excursion, so its exact neighbor remains
            // `near`.
            let mut return_states = clean.ret_states(print, tr, 0);
            while return_states != 0 {
                let return_st = return_states.trailing_zeros() as usize;
                return_states &= return_states - 1;
                push(return_st, print, near, &mut possible, &mut q);
            }
        } else if print == 0 {
            // Move directly away from the blank side while leaving zero on
            // the old head cell.  The old exact `near` becomes the new scan;
            // enumerate the next outward neighbor from the exact target
            // window with blank back/parent color 0.
            let mut next_nears =
                window_child_mask(tr, near, away_side, 0, windows);
            while next_nears != 0 {
                let next_near = next_nears.trailing_zeros() as usize;
                next_nears &= next_nears - 1;
                push(tr, near, next_near, &mut possible, &mut q);
            }
        }
    }

    possible
}

/// Sound over-approximation of reachable fresh-frontier configurations.
///
/// `frontier_side == false` describes a head at the left edge of the visited
/// interval, with the immediate cell to the left still an unvisited blank.
/// `frontier_side == true` is the symmetric right edge.
///
/// From a frontier checkpoint the machine may make an arbitrary balanced
/// excursion inward and return to the same frontier cell, or it may move
/// outward onto the next fresh cell, whose scanned color is exactly 0.  A
/// direct inward move that does not return is not itself a frontier checkpoint.
fn frontier_slots<const S: usize, const C: usize>(
    prog: &Prog<S, C>,
    windows: &WinPossible<S, C>,
    frontier_side: Shift,
    inward: &SideExcursions<S, C>,
) -> [[u64; C]; S] {
    debug_assert!(inward.pop.is_some());

    let mut possible = [[0_u64; C]; S];
    let mut trans = [[None; C]; S];

    for ((st, co), &(print, shift, tr)) in prog.iter() {
        trans[st as usize][co as usize] =
            Some((print as usize, shift, tr as usize));
    }

    let mut q = VecDeque::new();

    let push = |st: usize,
                co: usize,
                near: usize,
                possible: &mut [[u64; C]; S],
                q: &mut VecDeque<(usize, usize)>| {
        // Exact frontier checkpoint:
        //   left frontier:  0 [scan] near
        //   right frontier: near [scan] 0
        // The outward zero is still unvisited, while `near` is the exact
        // immediate color on the already visited/inward side.
        let window_ok = if frontier_side {
            windows.right[st][co][near] & 1 != 0
        } else {
            windows.right[st][co][0] & (1_u64 << near) != 0
        };
        let bit = 1_u64 << near;

        if window_ok && possible[st][co] & bit == 0 {
            possible[st][co] |= bit;
            q.push_back((SideExcursions::<S, C>::node(st, co), near));
        }
    };

    // The initial blank configuration is simultaneously both frontiers, and
    // its inward neighbor is also exact blank.
    push(0, 0, 0, &mut possible, &mut q);

    let inward_side = !frontier_side;

    while let Some((node, near)) = q.pop_front() {
        let (st, co) = SideExcursions::<S, C>::decode(node);

        let Some((print, shift, tr)) = trans[st][co] else {
            continue;
        };

        if shift == frontier_side {
            // Advance onto a fresh blank.  The old frontier cell becomes the
            // new exact inward neighbor with the transition's printed color.
            push(tr, 0, print, &mut possible, &mut q);
            continue;
        }

        debug_assert_eq!(shift, inward_side);

        // Move onto the exact inward neighbor.  The old frontier cell is the
        // child's parent/back cell and contains exactly `print` after the
        // departure.  When the excursion returns, its final pop color is the
        // new exact inward-neighbor color at this frontier checkpoint.
        let mut return_states = inward.ret_states(print, tr, near);
        while return_states != 0 {
            let return_st = return_states.trailing_zeros() as usize;
            return_states &= return_states - 1;

            let mut pop_colors =
                inward.pop_colors(print, tr, near, return_st);
            while pop_colors != 0 {
                let pop_color = pop_colors.trailing_zeros() as usize;
                pop_colors &= pop_colors - 1;
                push(
                    return_st,
                    print,
                    pop_color,
                    &mut possible,
                    &mut q,
                );
            }
        }
    }

    possible
}

/// Exact-window closure generated from fresh-frontier checkpoints and complete
/// same-cell one-sided excursions.
///
/// Every tape cell is first visited while it is a visited-interval frontier.
/// Between two consecutive visits to that same cell, the head must stay
/// strictly on one side of it; otherwise it would have crossed the cell and
/// visited it earlier.  Therefore seeding all reachable frontier windows and
/// closing under the exact-front excursion relation is a sound
/// over-approximation of every local window reachable from blank.
fn same_cell_crossing_windows_radius1_from_blank<
    const S: usize,
    const C: usize,
>(
    prog: &Prog<S, C>,
    windows: &WinPossible<S, C>,
    left_any: &SideExcursions<S, C>,
    right_any: &SideExcursions<S, C>,
) -> [[[u64; C]; C]; S] {
    debug_assert!(left_any.exact_pop.is_some());
    debug_assert!(right_any.exact_pop.is_some());

    let left_frontier = frontier_slots(prog, windows, false, right_any);
    let right_frontier = frontier_slots(prog, windows, true, left_any);
    let mut possible = [[[0_u64; C]; C]; S];
    let mut q = VecDeque::new();
    let mut trans = [[None; C]; S];

    for ((st, co), &(print, shift, tr)) in prog.iter() {
        trans[st as usize][co as usize] =
            Some((print as usize, shift, tr as usize));
    }

    let push =
        |st: usize,
         scan: usize,
         left: usize,
         right: usize,
         possible: &mut [[[u64; C]; C]; S],
         q: &mut VecDeque<(usize, usize, usize, usize)>| {
            let bit = 1_u64 << right;
            if windows.right[st][scan][left] & bit != 0
                && possible[st][scan][left] & bit == 0
            {
                possible[st][scan][left] |= bit;
                q.push_back((st, scan, left, right));
            }
        };

    // Every frontier checkpoint is already an exact local window: the outward
    // neighbor is fresh zero and the inward neighbor is retained exactly.
    for st in 0..S {
        for scan in 0..C {
            let mut nears = left_frontier[st][scan];
            while nears != 0 {
                let near = nears.trailing_zeros() as usize;
                nears &= nears - 1;
                push(st, scan, 0, near, &mut possible, &mut q);
            }

            let mut nears = right_frontier[st][scan];
            while nears != 0 {
                let near = nears.trailing_zeros() as usize;
                nears &= nears - 1;
                push(st, scan, near, 0, &mut possible, &mut q);
            }
        }
    }

    // Keep the true initial window explicit even if later frontier tightening
    // becomes stronger than necessary for some intermediate window relation.
    push(0, 0, 0, 0, &mut possible, &mut q);

    while let Some((st, scan, left, right)) = q.pop_front() {
        let Some((print, shift, child_st)) = trans[st][scan] else {
            continue;
        };

        if shift {
            // Depart right.  The current cell becomes the child's exact back
            // color `print`; the old right neighbor is the child's scan.  Keep
            // its farther-front color exact through the complete excursion.
            let mut fronts = window_child_mask(
                child_st, right, true, print, windows,
            );
            while fronts != 0 {
                let front = fronts.trailing_zeros() as usize;
                fronts &= fronts - 1;

                for return_st in 0..S {
                    let mut pop_colors = right_any.exact_pop_colors(
                        print, child_st, right, front, return_st,
                    );
                    while pop_colors != 0 {
                        let pop_color =
                            pop_colors.trailing_zeros() as usize;
                        pop_colors &= pop_colors - 1;
                        push(
                            return_st,
                            print,
                            left,
                            pop_color,
                            &mut possible,
                            &mut q,
                        );
                    }
                }
            }
        } else {
            let mut fronts = window_child_mask(
                child_st, left, false, print, windows,
            );
            while fronts != 0 {
                let front = fronts.trailing_zeros() as usize;
                fronts &= fronts - 1;

                for return_st in 0..S {
                    let mut pop_colors = left_any.exact_pop_colors(
                        print, child_st, left, front, return_st,
                    );
                    while pop_colors != 0 {
                        let pop_color =
                            pop_colors.trailing_zeros() as usize;
                        pop_colors &= pop_colors - 1;
                        push(
                            return_st,
                            print,
                            pop_color,
                            right,
                            &mut possible,
                            &mut q,
                        );
                    }
                }
            }
        }
    }

    possible
}

/// Exact two-cell inward prefixes at fresh-frontier checkpoints.
///
/// Bit `near * C + far` in `possible[state][scan]` means the frontier can be
/// reached with the first two cells on the visited/inward side exactly
/// `(near, far)`.  Keeping this pair here avoids the old radius-2 seed step
/// taking a Cartesian product with every possible second-neighbor color.
fn frontier_pair_slots<const S: usize, const C: usize>(
    prog: &Prog<S, C>,
    windows: &WinPossible<S, C>,
    frontier_side: Shift,
    inward: &SideExcursions<S, C>,
) -> [[u64; C]; S] {
    debug_assert!(C <= 8);
    debug_assert!(inward.exact_pop.is_some());

    let mut possible = [[0_u64; C]; S];
    let mut trans = [[None; C]; S];
    for ((st, co), &(print, shift, tr)) in prog.iter() {
        trans[st as usize][co as usize] =
            Some((print as usize, shift, tr as usize));
    }

    let mut q = VecDeque::new();
    let push = |st: usize,
                co: usize,
                near: usize,
                far: usize,
                possible: &mut [[u64; C]; S],
                q: &mut VecDeque<(usize, usize)>| {
        let window_ok = if frontier_side {
            // Right frontier: `near [scan] 0`.
            windows.right[st][co][near] & 1 != 0
        } else {
            // Left frontier: `0 [scan] near`.
            windows.right[st][co][0] & (1_u64 << near) != 0
        };
        if !window_ok {
            return;
        }

        let pair = near * C + far;
        let bit = 1_u64 << pair;
        if possible[st][co] & bit == 0 {
            possible[st][co] |= bit;
            q.push_back((SideExcursions::<S, C>::node(st, co), pair));
        }
    };

    // The initial blank tape is both frontiers and has two exact inward zeros.
    push(0, 0, 0, 0, &mut possible, &mut q);

    let inward_side = !frontier_side;
    while let Some((node, pair)) = q.pop_front() {
        let (st, co) = SideExcursions::<S, C>::decode(node);
        let near = pair / C;
        let far = pair % C;
        let Some((print, shift, tr)) = trans[st][co] else {
            continue;
        };

        if shift == frontier_side {
            // Move onto a fresh blank.  The old frontier cell and its old
            // inward neighbor become the new first two inward cells.
            push(tr, 0, print, near, &mut possible, &mut q);
            continue;
        }

        debug_assert_eq!(shift, inward_side);

        // Move inward and make a complete balanced excursion.  The strong
        // excursion relation updates both retained inward cells jointly.
        for return_st in 0..S {
            let mut outcomes = inward
                .exact_pop_front_pairs(print, tr, near, far, return_st);
            while outcomes != 0 {
                let outcome = outcomes.trailing_zeros() as usize;
                outcomes &= outcomes - 1;
                let pop_color = outcome / C;
                let final_front = outcome % C;
                push(
                    return_st,
                    print,
                    pop_color,
                    final_front,
                    &mut possible,
                    &mut q,
                );
            }
        }
    }

    possible
}

/// Radius-2 same-cell crossing closure.  A checkpoint retains
/// `l2 l1 [scan] r1 r2` in one forward witness.  The ordinary radius-one
/// window remains the storage/reduced-product interface; the second-neighbor
/// pair is packed into one u64 per exact local window.
///
/// Radius-2 frontier seeds are themselves propagated with exact two-cell
/// inward prefixes.  This is both stronger and usually much smaller than the
/// previous `frontier_slots x all second colors` Cartesian seed set.
///
/// The worklist is bucketed by the ordinary radius-one window.  Newly learned
/// `(l2,r2)` bits for the same window are coalesced into one queue entry, which
/// avoids pushing a six-usize tuple for every individual radius-2 checkpoint.
fn same_cell_crossing_windows_radius2_from_blank<
    const S: usize,
    const C: usize,
>(
    prog: &Prog<S, C>,
    windows: &WinPossible<S, C>,
    left_any: &SideExcursions<S, C>,
    right_any: &SideExcursions<S, C>,
) -> Radius2Possible<S, C> {
    debug_assert!(C <= 8);
    debug_assert!(left_any.exact_pop.is_some());
    debug_assert!(right_any.exact_pop.is_some());

    let left_frontier =
        frontier_pair_slots(prog, windows, false, right_any);
    let right_frontier =
        frontier_pair_slots(prog, windows, true, left_any);
    let window_count = S * C * C * C;
    let mut possible2 = vec![0_u64; window_count];
    let mut pending = vec![0_u64; window_count];
    let mut q = VecDeque::<usize>::new();
    let mut trans = [[None; C]; S];

    for ((st, co), &(print, shift, tr)) in prog.iter() {
        trans[st as usize][co as usize] =
            Some((print as usize, shift, tr as usize));
    }

    let window_index =
        |st: usize, scan: usize, left: usize, right: usize| {
            (((st * C + scan) * C + left) * C) + right
        };

    let push = |st: usize,
                scan: usize,
                l2: usize,
                left: usize,
                right: usize,
                r2: usize,
                possible2: &mut [u64],
                pending: &mut [u64],
                q: &mut VecDeque<usize>| {
        if windows.right[st][scan][left] & (1_u64 << right) == 0 {
            return;
        }

        let pair = l2 * C + r2;
        let bit = 1_u64 << pair;
        let index = window_index(st, scan, left, right);
        if possible2[index] & bit != 0 {
            return;
        }

        possible2[index] |= bit;
        let was_empty = pending[index] == 0;
        pending[index] |= bit;
        if was_empty {
            q.push_back(index);
        }
    };

    // Left frontier: `0 0 [scan] r1 r2`.
    for st in 0..S {
        for scan in 0..C {
            let mut pairs = left_frontier[st][scan];
            while pairs != 0 {
                let pair = pairs.trailing_zeros() as usize;
                pairs &= pairs - 1;
                let r1 = pair / C;
                let r2 = pair % C;
                push(
                    st,
                    scan,
                    0,
                    0,
                    r1,
                    r2,
                    &mut possible2,
                    &mut pending,
                    &mut q,
                );
            }

            // Right frontier: `l2 l1 [scan] 0 0`.
            let mut pairs = right_frontier[st][scan];
            while pairs != 0 {
                let pair = pairs.trailing_zeros() as usize;
                pairs &= pairs - 1;
                let l1 = pair / C;
                let l2 = pair % C;
                push(
                    st,
                    scan,
                    l2,
                    l1,
                    0,
                    0,
                    &mut possible2,
                    &mut pending,
                    &mut q,
                );
            }
        }
    }

    // Preserve the concrete initial radius-2 checkpoint explicitly.
    push(0, 0, 0, 0, 0, 0, &mut possible2, &mut pending, &mut q);

    while let Some(index) = q.pop_front() {
        // Clear the pending bucket before propagating it.  If this propagation
        // discovers another pair for the same window, `push` simply queues the
        // window once more with only that new delta.
        let mut checkpoint_pairs = core::mem::take(&mut pending[index]);

        let right = index % C;
        let rest = index / C;
        let left = rest % C;
        let rest = rest / C;
        let scan = rest % C;
        let st = rest / C;

        let Some((print, shift, child_st)) = trans[st][scan] else {
            continue;
        };

        while checkpoint_pairs != 0 {
            let checkpoint = checkpoint_pairs.trailing_zeros() as usize;
            checkpoint_pairs &= checkpoint_pairs - 1;
            let l2 = checkpoint / C;
            let r2 = checkpoint % C;

            if shift {
                for return_st in 0..S {
                    let mut outcomes = right_any.exact_pop_front_pairs(
                        print, child_st, right, r2, return_st,
                    );
                    while outcomes != 0 {
                        let outcome =
                            outcomes.trailing_zeros() as usize;
                        outcomes &= outcomes - 1;
                        let pop_color = outcome / C;
                        let final_front = outcome % C;
                        push(
                            return_st,
                            print,
                            l2,
                            left,
                            pop_color,
                            final_front,
                            &mut possible2,
                            &mut pending,
                            &mut q,
                        );
                    }
                }
            } else {
                for return_st in 0..S {
                    let mut outcomes = left_any.exact_pop_front_pairs(
                        print, child_st, left, l2, return_st,
                    );
                    while outcomes != 0 {
                        let outcome =
                            outcomes.trailing_zeros() as usize;
                        outcomes &= outcomes - 1;
                        let pop_color = outcome / C;
                        let final_front = outcome % C;
                        push(
                            return_st,
                            print,
                            final_front,
                            pop_color,
                            right,
                            r2,
                            &mut possible2,
                            &mut pending,
                            &mut q,
                        );
                    }
                }
            }
        }
    }

    Radius2Possible::from_pairs(possible2)
}

/// Tighten the local-window relation with the same-cell crossing reduced
/// product.  The radius-one closure is intentionally run first on each round:
/// when that cheaper relation removes a window, rebuilding radius-2 immediately
/// would be wasted work because the excursion grammar must be recomputed on the
/// smaller graph anyway.  Radius-2 therefore runs only after radius-one is
/// stable for the current window graph.  Both refinements are monotone
/// intersections, so this scheduling preserves the same joint fixed point.
fn refine_windows_by_same_cell_crossings<
    const S: usize,
    const C: usize,
>(
    prog: &Prog<S, C>,
    windows: &mut WinPossible<S, C>,
) -> (
    SideExcursions<S, C>,
    SideExcursions<S, C>,
    Radius2Possible<S, C>,
) {
    loop {
        let left_any = side_excursions(prog, windows, false, false);
        let right_any = side_excursions(prog, windows, true, false);

        let crossing1 = same_cell_crossing_windows_radius1_from_blank(
            prog, windows, &left_any, &right_any,
        );
        if windows.refine_crossing_reachability_relation(&crossing1) {
            continue;
        }

        if C <= 8 {
            let radius2 = same_cell_crossing_windows_radius2_from_blank(
                prog, windows, &left_any, &right_any,
            );
            let crossing2 = radius2.radius1_projection();
            if windows.refine_crossing_reachability_relation(&crossing2)
            {
                continue;
            }
            return (left_any, right_any, radius2);
        }

        return (left_any, right_any, Radius2Possible::disabled());
    }
}

/// Close same-cell crossings and whole-side reachability together.  This is
/// used both for the initial forward relation and after an ordered-prefix
/// domain removes local windows, so radius-2/JointShort feedback reaches the
/// same reduced-product fixed point instead of using a stale radius-2 table.
fn refine_windows_by_crossings_and_sides<
    const S: usize,
    const C: usize,
>(
    prog: &Prog<S, C>,
    windows: &mut WinPossible<S, C>,
) -> (
    SidePossible<S, C>,
    SideExcursions<S, C>,
    SideExcursions<S, C>,
    Radius2Possible<S, C>,
) {
    loop {
        let (left_any, right_any, radius2) =
            refine_windows_by_same_cell_crossings(prog, windows);
        let sides = prog.side_possible_from_blank(windows);
        if !windows.refine_side_reachability_relation(&sides) {
            return (sides, left_any, right_any, radius2);
        }
    }
}

/// Independent per-color forward abstraction of capped counts strictly beyond
/// the immediate neighbors.
///
/// For one nonblank color at a time, each side count is `0`, `1`, or `2+`.
/// On an R move, old `left` enters the new left tail. The newly exposed
/// `new_right` is removed from the old right tail count; removing one tracked
/// color from `2+` leaves either `1` or `2+`. L moves are symmetric.
#[expect(clippy::cast_possible_truncation)]
fn color_tail_count_from_blank<const S: usize, const C: usize>(
    prog: &Prog<S, C>,
    windows: &WinPossible<S, C>,
) -> ColorTailCountPossible<S, C> {
    fn add_neighbor(count: u8, matches: bool) -> u8 {
        if matches { (count + 1).min(2) } else { count }
    }

    /// Bitset over residual capped counts after exposing one cell from a tail.
    ///
    /// A zero bitset means the exposed color contradicts the old capped count.
    const fn residual_mask(count: u8, exposed_matches: bool) -> u8 {
        #[expect(clippy::match_same_arms)]
        match (count, exposed_matches) {
            (0, false) => 0b001,
            (0, true) => 0,
            (1, false) => 0b010,
            (1, true) => 0b001,
            (2, false) => 0b100,
            (2, true) => 0b110,
            _ => 0,
        }
    }

    let mut trans = [[None; C]; S];
    for ((st, co), &(print, shift, tr)) in prog.iter() {
        trans[st as usize][co as usize] =
            Some((print as usize, shift, tr as usize));
    }

    let mut possible = ColorTailCountPossible::new();
    let mut q = VecDeque::new();

    let push = |st: usize,
                left: usize,
                scan: usize,
                right: usize,
                color: usize,
                status: u8,
                possible: &mut ColorTailCountPossible<S, C>,
                q: &mut VecDeque<(
        usize,
        usize,
        usize,
        usize,
        usize,
        u8,
    )>| {
        if windows.right[st][scan][left] & (1_u64 << right) == 0 {
            return;
        }

        let index = ColorTailCountPossible::<S, C>::exact_index(
            st, scan, left, right, color,
        );
        let bit = 1_u16 << status;
        if possible.exact[index] & bit != 0 {
            return;
        }

        possible.add(st, scan, left, right, color, status);
        q.push_back((st, left, scan, right, color, status));
    };

    for color in 1..C {
        push(0, 0, 0, 0, color, 0, &mut possible, &mut q);
    }

    while let Some((st, left, scan, right, color, status)) =
        q.pop_front()
    {
        let Some((print, shift, tr)) = trans[st][scan] else {
            continue;
        };

        let left_count = status % 3;
        let right_count = status / 3;

        if shift {
            let new_left = add_neighbor(left_count, left == color);

            let mut new_rights = windows.right[tr][right][print];
            while new_rights != 0 {
                let new_right = new_rights.trailing_zeros() as usize;
                new_rights &= new_rights - 1;

                let residual =
                    residual_mask(right_count, new_right == color);
                let mut residuals = residual;
                while residuals != 0 {
                    let new_right_count =
                        residuals.trailing_zeros() as u8;
                    residuals &= residuals - 1;

                    let new_status = new_left + 3 * new_right_count;
                    push(
                        tr,
                        print,
                        right,
                        new_right,
                        color,
                        new_status,
                        &mut possible,
                        &mut q,
                    );
                }
            }
        } else {
            let new_right = add_neighbor(right_count, right == color);

            let mut new_lefts = windows.left[tr][left][print];
            while new_lefts != 0 {
                let new_left = new_lefts.trailing_zeros() as usize;
                new_lefts &= new_lefts - 1;

                let residual =
                    residual_mask(left_count, new_left == color);
                let mut residuals = residual;
                while residuals != 0 {
                    let new_left_count =
                        residuals.trailing_zeros() as u8;
                    residuals &= residuals - 1;

                    let new_status = new_left_count + 3 * new_right;
                    push(
                        tr,
                        new_left,
                        left,
                        print,
                        color,
                        new_status,
                        &mut possible,
                        &mut q,
                    );
                }
            }
        }
    }

    possible
}

/// Pairwise same-run presence abstraction retained alongside the capped-count layer.
///
/// A newly exposed cell can equal at most one member of an unordered color
/// pair, so consuming it makes at most one pair component uncertain. This
/// keeps each transition to at most two residual-status branches in concrete
/// runs, while the stored 16-bit mask retains all same-run correlations.
#[expect(clippy::similar_names)]
fn pair_tail_presence_from_blank<const S: usize, const C: usize>(
    prog: &Prog<S, C>,
    windows: &WinPossible<S, C>,
) -> PairTailPresencePossible<S, C> {
    fn residual_mask(
        present: bool,
        exposed: usize,
        color: usize,
    ) -> u8 {
        if !present {
            u8::from(exposed != color)
        } else if exposed == color {
            0b11
        } else {
            0b10
        }
    }

    let mut trans = [[None; C]; S];
    for ((st, co), &(print, shift, tr)) in prog.iter() {
        trans[st as usize][co as usize] =
            Some((print as usize, shift, tr as usize));
    }

    let mut possible = PairTailPresencePossible::new();
    if C < 3 {
        return possible;
    }

    let mut q = VecDeque::new();

    let push = |st: usize,
                left: usize,
                scan: usize,
                right: usize,
                a: usize,
                b: usize,
                status: u8,
                possible: &mut PairTailPresencePossible<S, C>,
                q: &mut VecDeque<(
        usize,
        usize,
        usize,
        usize,
        usize,
        usize,
        u8,
    )>| {
        if windows.right[st][scan][left] & (1_u64 << right) == 0 {
            return;
        }

        let pair = PairTailPresencePossible::<S, C>::pair_index(a, b);
        let index = PairTailPresencePossible::<S, C>::exact_index(
            st, scan, left, right, pair,
        );
        let bit = 1_u16 << status;
        if possible.exact[index] & bit != 0 {
            return;
        }

        possible.add(st, scan, left, right, a, b, status);
        q.push_back((st, left, scan, right, a, b, status));
    };

    for a in 1..C {
        for b in (a + 1)..C {
            push(0, 0, 0, 0, a, b, 0, &mut possible, &mut q);
        }
    }

    while let Some((st, left, scan, right, a, b, status)) =
        q.pop_front()
    {
        let Some((print, shift, tr)) = trans[st][scan] else {
            continue;
        };

        let a_left = status & 1 != 0;
        let a_right = status & 2 != 0;
        let b_left = status & 4 != 0;
        let b_right = status & 8 != 0;

        if shift {
            let new_a_left = a_left || left == a;
            let new_b_left = b_left || left == b;

            let mut new_rights = windows.right[tr][right][print];
            while new_rights != 0 {
                let new_right = new_rights.trailing_zeros() as usize;
                new_rights &= new_rights - 1;

                let a_residual = residual_mask(a_right, new_right, a);
                let b_residual = residual_mask(b_right, new_right, b);

                for new_a_right in 0..2_u8 {
                    if a_residual & (1_u8 << new_a_right) == 0 {
                        continue;
                    }
                    for new_b_right in 0..2_u8 {
                        if b_residual & (1_u8 << new_b_right) == 0 {
                            continue;
                        }

                        let new_status = u8::from(new_a_left)
                            | (new_a_right << 1)
                            | (u8::from(new_b_left) << 2)
                            | (new_b_right << 3);
                        push(
                            tr,
                            print,
                            right,
                            new_right,
                            a,
                            b,
                            new_status,
                            &mut possible,
                            &mut q,
                        );
                    }
                }
            }
        } else {
            let new_a_right = a_right || right == a;
            let new_b_right = b_right || right == b;

            let mut new_lefts = windows.left[tr][left][print];
            while new_lefts != 0 {
                let new_left = new_lefts.trailing_zeros() as usize;
                new_lefts &= new_lefts - 1;

                let a_residual = residual_mask(a_left, new_left, a);
                let b_residual = residual_mask(b_left, new_left, b);

                for new_a_left in 0..2_u8 {
                    if a_residual & (1_u8 << new_a_left) == 0 {
                        continue;
                    }
                    for new_b_left in 0..2_u8 {
                        if b_residual & (1_u8 << new_b_left) == 0 {
                            continue;
                        }

                        let new_status = new_a_left
                            | (u8::from(new_a_right) << 1)
                            | (new_b_left << 2)
                            | (u8::from(new_b_right) << 3);
                        push(
                            tr,
                            new_left,
                            left,
                            print,
                            a,
                            b,
                            new_status,
                            &mut possible,
                            &mut q,
                        );
                    }
                }
            }
        }
    }

    possible
}

/// Joint forward abstraction of whether each whole side is exactly blank or
/// definitely dirty (contains at least one nonblank), conditioned on the exact
/// local window `(left, scan, right)`.
///
/// Unlike a state/scan-only table, the status pair and local neighbor colors
/// travel through one abstract run. When moving into a dirty side, consuming
/// its nearest nonblank may expose either an all-blank or still-dirty residual;
/// consuming a blank from a dirty side leaves the residual definitely dirty.
/// The global `WinPossible` relation is used only as a sound cap on newly
/// exposed neighbor colors.
fn joint_blank_status_from_blank<const S: usize, const C: usize>(
    prog: &Prog<S, C>,
    windows: &WinPossible<S, C>,
) -> JointBlankPossible<S, C> {
    let mut trans = [[None; C]; S];
    for ((st, co), &(print, shift, tr)) in prog.iter() {
        trans[st as usize][co as usize] =
            Some((print as usize, shift, tr as usize));
    }

    let mut possible = JointBlankPossible::new();
    let mut q = VecDeque::new();

    let push =
        |st: usize,
         left: usize,
         scan: usize,
         right: usize,
         flags: u8,
         possible: &mut JointBlankPossible<S, C>,
         q: &mut VecDeque<(usize, usize, usize, usize, u8)>| {
            // Exact blank-side facts force the corresponding immediate neighbor
            // to zero. Reject inconsistent abstract states rather than letting a
            // later join make them useful.
            if flags & LEFT_BLANK_FLAG != 0 && left != 0 {
                return;
            }
            if flags & RIGHT_BLANK_FLAG != 0 && right != 0 {
                return;
            }

            // Keep only globally reachable exact windows. This is conservative:
            // the status product may still join dirty-tail contents, but can never
            // invent a local window that the existing forward abstraction rejects.
            if windows.right[st][scan][left] & (1_u64 << right) == 0 {
                return;
            }

            let bit = 1_u8 << flags;
            let index = JointBlankPossible::<S, C>::index(
                st, scan, left, right,
            );
            if possible.windows[index] & bit != 0 {
                return;
            }

            possible.windows[index] |= bit;
            possible.any[st][scan] |= bit;
            q.push_back((st, left, scan, right, flags));
        };

    push(0, 0, 0, 0, BOTH_BLANK_FLAGS, &mut possible, &mut q);

    while let Some((st, left, scan, right, flags)) = q.pop_front() {
        let Some((print, shift, tr)) = trans[st][scan] else {
            continue;
        };

        let left_blank = flags & LEFT_BLANK_FLAG != 0;
        let right_blank = flags & RIGHT_BLANK_FLAG != 0;

        if shift {
            // Move R:
            //   (left, scan, right) -> (print, right, new_right)
            // The old head joins the left side. The old right neighbor is
            // consumed into the scan, so the new right-side status describes
            // the residual beyond that consumed cell.
            let new_left_blank = left_blank && print == 0;
            let mut new_rights = windows.right[tr][right][print];

            if right_blank {
                debug_assert_eq!(right, 0);
                new_rights &= 1; // residual of an all-blank side is blank
                while new_rights != 0 {
                    let new_right =
                        new_rights.trailing_zeros() as usize;
                    new_rights &= new_rights - 1;
                    let new_flags =
                        u8::from(new_left_blank) | RIGHT_BLANK_FLAG;
                    push(
                        tr,
                        print,
                        right,
                        new_right,
                        new_flags,
                        &mut possible,
                        &mut q,
                    );
                }
                continue;
            }

            if right == 0 {
                // The side was dirty and its nearest cell was blank, so some
                // nonblank remains farther out. The residual is definitely
                // dirty regardless of the newly exposed neighbor color.
                while new_rights != 0 {
                    let new_right =
                        new_rights.trailing_zeros() as usize;
                    new_rights &= new_rights - 1;
                    let new_flags = u8::from(new_left_blank);
                    push(
                        tr,
                        print,
                        right,
                        new_right,
                        new_flags,
                        &mut possible,
                        &mut q,
                    );
                }
                continue;
            }

            // Consuming a nonblank from a dirty side may have consumed its
            // last nonblank, or dirt may remain farther out. The blank branch
            // requires the newly exposed neighbor to be zero; the dirty branch
            // allows every target-window color.
            let mut dirty_rights = new_rights;
            while dirty_rights != 0 {
                let new_right = dirty_rights.trailing_zeros() as usize;
                dirty_rights &= dirty_rights - 1;
                let new_flags = u8::from(new_left_blank);
                push(
                    tr,
                    print,
                    right,
                    new_right,
                    new_flags,
                    &mut possible,
                    &mut q,
                );
            }

            if new_rights & 1 != 0 {
                let new_flags =
                    u8::from(new_left_blank) | RIGHT_BLANK_FLAG;
                push(
                    tr,
                    print,
                    right,
                    0,
                    new_flags,
                    &mut possible,
                    &mut q,
                );
            }
        } else {
            // Move L, symmetrically:
            //   (left, scan, right) -> (new_left, left, print)
            let new_right_blank = right_blank && print == 0;
            let mut new_lefts = windows.left[tr][left][print];

            if left_blank {
                debug_assert_eq!(left, 0);
                new_lefts &= 1;
                while new_lefts != 0 {
                    let new_left = new_lefts.trailing_zeros() as usize;
                    new_lefts &= new_lefts - 1;
                    let new_flags = LEFT_BLANK_FLAG
                        | (u8::from(new_right_blank) << 1);
                    push(
                        tr,
                        new_left,
                        left,
                        print,
                        new_flags,
                        &mut possible,
                        &mut q,
                    );
                }
                continue;
            }

            if left == 0 {
                while new_lefts != 0 {
                    let new_left = new_lefts.trailing_zeros() as usize;
                    new_lefts &= new_lefts - 1;
                    let new_flags = u8::from(new_right_blank) << 1;
                    push(
                        tr,
                        new_left,
                        left,
                        print,
                        new_flags,
                        &mut possible,
                        &mut q,
                    );
                }
                continue;
            }

            let mut dirty_lefts = new_lefts;
            while dirty_lefts != 0 {
                let new_left = dirty_lefts.trailing_zeros() as usize;
                dirty_lefts &= dirty_lefts - 1;
                let new_flags = u8::from(new_right_blank) << 1;
                push(
                    tr,
                    new_left,
                    left,
                    print,
                    new_flags,
                    &mut possible,
                    &mut q,
                );
            }

            if new_lefts & 1 != 0 {
                let new_flags =
                    LEFT_BLANK_FLAG | (u8::from(new_right_blank) << 1);
                push(
                    tr,
                    0,
                    left,
                    print,
                    new_flags,
                    &mut possible,
                    &mut q,
                );
            }
        }
    }

    possible
}

fn blank_side_possible_from_blank_with_any<
    const S: usize,
    const C: usize,
>(
    prog: &Prog<S, C>,
    windows: &WinPossible<S, C>,
    left_any: &SideExcursions<S, C>,
    right_any: &SideExcursions<S, C>,
) -> BlankSidePossible<S, C> {
    let left_clean = side_excursions(prog, windows, false, true);
    let right_clean = side_excursions(prog, windows, true, true);

    let left_half =
        halfblank_slots(prog, windows, false, &left_clean, right_any);
    let right_half =
        halfblank_slots(prog, windows, true, &right_clean, left_any);
    let joint = joint_blank_status_from_blank(prog, windows);

    BlankSidePossible {
        left_half,
        right_half,
        joint,
    }
}

fn blank_side_possible_from_blank<const S: usize, const C: usize>(
    prog: &Prog<S, C>,
    windows: &WinPossible<S, C>,
) -> BlankSidePossible<S, C> {
    let left_any = side_excursions(prog, windows, false, false);
    let right_any = side_excursions(prog, windows, true, false);
    blank_side_possible_from_blank_with_any(
        prog, windows, &left_any, &right_any,
    )
}

fn scc_from_reach<const S: usize>(
    reach: &[[bool; S]; S],
) -> ([usize; S], [u16; S], usize) {
    let mut comp = [usize::MAX; S];
    let mut masks = [0; S];
    let mut k = 0;

    for i in 0..S {
        if comp[i] != usize::MAX {
            continue;
        }
        let cid = k;
        k += 1;

        let mut mask: u16 = 0;
        for j in 0..S {
            if reach[i][j] && reach[j][i] {
                comp[j] = cid;
                mask |= 1 << j;
            }
        }
        masks[cid] = mask;
    }

    (comp, masks, k)
}

fn add_gen<const S: usize>(arr: &mut [i32; S], len: &mut u8, val: i32) {
    debug_assert!(val > 0);
    let n = *len as usize;
    for i in 0..n {
        if arr[i] == val {
            return;
        }
    }
    if n < S {
        arr[n] = val;
        *len += 1;
    }
}

/// DC meta + generators.
/// Returns:
/// - reach
/// - comp[state]
/// - masks[cid]
/// - k
/// - g_scc[cid]
/// - res[state]
/// - pos_gens[cid], pos_len[cid] : positive cycle displacements found
/// - neg_gens[cid], neg_len[cid] : absolute value of negative cycle displacements found
#[expect(clippy::excessive_nesting)]
fn dc_meta_with_gens<const S: usize>(
    adj: &Adj<S>,
    next: &NextDir<S>,
) -> (
    [[bool; S]; S],
    [usize; S],
    [u16; S],
    usize,
    [i32; S],
    [i32; S],
    [[i32; S]; S],
    [u8; S],
    [[i32; S]; S],
    [u8; S],
) {
    let reach = reachability::<S>(adj);
    let (comp, masks, k) = scc_from_reach::<S>(&reach);

    let mut g_scc = [0; S];
    let mut res = [0; S];

    let mut pos_gens = [[0; S]; S];
    let mut pos_len = [0; S];
    let mut neg_gens = [[0; S]; S];
    let mut neg_len = [0; S];

    for cid in 0..k {
        let mask = masks[cid];
        if mask == 0 {
            continue;
        }

        let Some(root) = (0..S).find(|&v| (mask >> v) & 1 != 0) else {
            continue;
        };

        let in_comp: [bool; S] = from_fn(|v| ((mask >> v) & 1) == 1);

        let mut dist: [Option<i32>; S] = [None; S];
        dist[root] = Some(0);

        let mut q = VecDeque::new();
        q.push_back(root);

        let mut g = 0;

        while let Some(u) = q.pop_front() {
            let du = dist[u].unwrap();

            for dir in 0..2 {
                let w = if dir == 1 { 1 } else { -1 }; // R:+1, L:-1

                for &v in &next[u][dir] {
                    if !in_comp[v] {
                        continue;
                    }

                    let dv_new = du + w;

                    match dist[v] {
                        None => {
                            dist[v] = Some(dv_new);
                            q.push_back(v);
                        },
                        Some(dv) => {
                            // discrepancy = closed-walk displacement
                            let diff = dv_new - dv;
                            if diff != 0 {
                                g = if g == 0 {
                                    diff.abs()
                                } else {
                                    gcd_i32(g, diff)
                                };

                                if diff > 0 {
                                    add_gen::<S>(
                                        &mut pos_gens[cid],
                                        &mut pos_len[cid],
                                        diff,
                                    );
                                } else {
                                    add_gen::<S>(
                                        &mut neg_gens[cid],
                                        &mut neg_len[cid],
                                        -diff,
                                    );
                                }
                            }
                        },
                    }
                }
            }
        }

        g_scc[cid] = g;

        // fill residues
        for v in 0..S {
            if !in_comp[v] {
                continue;
            }
            let dv = dist[v].unwrap_or(0);
            res[v] = if g == 0 {
                dv
            } else {
                let mut r = dv % g;
                if r < 0 {
                    r += g;
                }
                r
            };
        }
    }

    (
        reach, comp, masks, k, g_scc, res, pos_gens, pos_len, neg_gens,
        neg_len,
    )
}

/// Bellman-Ford negative-cycle detection inside SCC.
/// If `negate` is true, weights are negated => detects positive
/// cycles of original graph.
fn has_neg_cycle_in_scc<const S: usize>(
    mask: u16,
    next: &NextDir<S>,
    negate: bool,
) -> bool {
    let mut nodes = [0; S];
    let mut n = 0;
    for v in 0..S {
        if ((mask >> v) & 1) == 1 {
            nodes[n] = v;
            n += 1;
        }
    }
    if n == 0 {
        return false;
    }

    let mut dist = [0; S];

    for iter in 0..n {
        let mut changed = false;

        for i in 0..n {
            let u = nodes[i];
            let du = dist[u];

            for dir in 0..2 {
                let mut w = if dir == 1 { 1 } else { -1 };
                if negate {
                    w = -w;
                }

                for &v in &next[u][dir] {
                    if ((mask >> v) & 1) == 0 {
                        continue;
                    }
                    let nv = du + w;
                    if nv < dist[v] {
                        dist[v] = nv;
                        changed = true;
                    }
                }
            }
        }

        if !changed {
            return false;
        }
        if iter == n - 1 && changed {
            return true;
        }
    }

    false
}

type ColorMask = u64;

fn printed_mask<const S: usize, const C: usize>(
    prog: &Prog<S, C>,
) -> ColorMask {
    let mut m = 0;
    for ((_, _read), &(pr, _, _)) in prog.iter() {
        m |= 1 << pr;
    }
    m
}

fn color_closure<const S: usize, const C: usize>(
    prog: &Prog<S, C>,
) -> [ColorMask; C] {
    debug_assert!(C <= 64);

    let mut clo = [0; C];

    // direct edges: read -> print
    for ((_, read), &(pr, _, _)) in prog.iter() {
        clo[read as usize] |= 1 << pr;
    }

    // include self
    for a in 0..C {
        clo[a] |= 1 << a;
    }

    // transitive closure (bitset Floyd)
    for k in 0..C {
        let kset = clo[k];
        for a in 0..C {
            if ((clo[a] >> k) & 1) != 0 {
                clo[a] |= kset;
            }
        }
    }

    clo
}

fn unerasable_mask<const C: usize>(clo: &[ColorMask; C]) -> ColorMask {
    // bit i set => color i>0 cannot reach 0
    let mut m = 0;
    for a in 1..C {
        let can0 = (clo[a] & 1) != 0; // bit0 is color 0
        if !can0 {
            m |= 1 << a;
        }
    }
    m
}

impl<const S: usize, const C: usize> Prog<S, C> {
    fn entrypoints_and_indices(&self) -> (Entrypoints, Indices<S, C>) {
        let mut entrypoints = Entrypoints::new();
        let mut idx = indices_new::<S, C>();

        for (slot @ (st, _), &(pr, sh, tr)) in self.iter() {
            let (same, diff) = entrypoints.entry(tr).or_default();

            (if st == tr { same } else { diff }).push((slot, (pr, sh)));

            indices_add::<S, C>(&mut idx, st, tr, sh, pr);
        }

        indices_finalize::<S, C>(&mut idx);

        (entrypoints, idx)
    }

    /// Static halt-slot filter:
    /// - reachability + SCC residue gate (DC)
    /// - if SCC has both drift signs: keep conservative
    /// - if SCC is one-sided: do an *exact* “can we hit net displacement 0?” check
    ///   via a small bounded product-graph BFS (state × displacement window).
    #[expect(clippy::excessive_nesting)]
    pub fn halt_slots_disp_side(
        &self,
        idx: &Indices<S, C>,
    ) -> Set<Slot> {
        let (adj, preds, writers, next) = idx;

        let (
            reach,
            comp,
            masks,
            k,
            _g_scc,
            res,
            _pos_gens,
            _pos_len,
            _neg_gens,
            _neg_len,
        ) = dc_meta_with_gens::<S>(adj, next);

        // SCC drift classification (same as you already do)
        let mut has_neg = [false; S];
        let mut has_pos = [false; S];
        for cid in 0..k {
            let mask = masks[cid];
            has_neg[cid] = has_neg_cycle_in_scc::<S>(mask, next, false);
            has_pos[cid] = has_neg_cycle_in_scc::<S>(mask, next, true);
        }

        // NEW: exact 0-displacement reachability cache for one-sided SCCs.
        // zero_done[cid][src] indicates whether we computed zero_reach[cid][src].
        // zero_reach[cid][src] is bitmask of nodes reachable from src with net disp 0
        // (under an orientation where SCC has no negative cycles).
        let mut zero_done = [[false; S]; S];
        let mut zero_reach = [[0; S]; S];

        let (max_st, max_co) = self.max_reached();

        (0..=max_st)
            .flat_map(|st| (0..=max_co).map(move |co| (st, co)))
            .filter(|slot @ &(st, co)| {
                // only consider missing slots as "candidate halting slots"
                self.get(slot).is_none()
                    && (co == 0 || {
                        let h = st as usize;
                        let co = co as usize;

                        for w in 0..2 {
                            let need = w ^ 1;

                            for &p in &preds[h][need] {
                                for &s0 in &writers[co][w] {
                                    if !reach[s0][p] {
                                        continue;
                                    }

                                    // across SCCs: conservative keep
                                    if comp[s0] != comp[p] {
                                        return true;
                                    }

                                    // same SCC
                                    let cid = comp[p];

                                    // residue gate (necessary; conservative if weak)
                                    if res[s0] != res[p] {
                                        continue;
                                    }

                                    // If SCC has both signs, congruence is about all we can use cheaply;
                                    // keep witness.
                                    if has_pos[cid] && has_neg[cid] {
                                        return true;
                                    }

                                    // SCC is one-sided (or bounded). Do exact disp==0 reachability.
                                    // Choose an orientation with NO negative cycles:
                                    // - if SCC has no neg cycles, use normal weights (R=+1,L=-1)
                                    // - if SCC has neg cycles but no pos cycles, negate weights
                                    let negate = has_neg[cid] && !has_pos[cid];

                                    if !zero_done[cid][s0] {
                                        zero_reach[cid][s0] =
                                            zero_disp_reach_mask_one_sided_scc::<S>(
                                                masks[cid],
                                                next,
                                                s0,
                                                negate,
                                            );
                                        zero_done[cid][s0] = true;
                                    }

                                    // p reachable from s0 with net displacement 0?
                                    if ((zero_reach[cid][s0] >> p) & 1) == 0 {
                                        // No exact 0-displacement witness in this SCC => prune this witness
                                        continue;
                                    }

                                    // Exact witness exists => keep candidate halt slot
                                    return true;
                                }
                            }
                        }

                        false
                    })
            })
            .collect()
    }

    /// Strengthen candidate halt slots with the color-aware one-sided
    /// excursion relation.
    ///
    /// For a nonblank scanned color, take the last departure from the eventual
    /// halting cell.  That transition must write the halting color and move
    /// into one side; until the final return, the head stays strictly on that
    /// side, so an ordinary balanced excursion from the entered child state
    /// must be able to return into the halting state.
    ///
    /// A halt scanning 0 has one additional possibility: the cell may be a
    /// first visit to a fresh blank frontier.  The dedicated frontier
    /// abstraction tracks that stronger visited-interval boundary property.
    /// Previously visited zero cells are covered by the same
    /// last-departure rule, with a transition that writes 0.
    fn halt_slots_side_excursion(&self, slots: Set<Slot>) -> Set<Slot> {
        if slots.is_empty() {
            return slots;
        }

        let (forbid_left, forbid_right) = self.shift_side_forbidden();
        let mut windows =
            self.win_possible_from_blank(&forbid_left, &forbid_right);
        let (left_any, right_any, _) =
            refine_windows_by_same_cell_crossings(self, &mut windows);

        let frontiers =
            slots.iter().any(|&(_, color)| color == 0).then(|| {
                // At the left frontier, arbitrary balanced work is to the
                // right/inward side.  At the right frontier it is to the left.
                let left =
                    frontier_slots(self, &windows, false, &right_any);
                let right =
                    frontier_slots(self, &windows, true, &left_any);
                (left, right)
            });

        slots
            .into_iter()
            .filter(|&(state, color)| {
                let h = state as usize;
                let co = color as usize;

                if !windows.any[h][co] {
                    return false;
                }

                if color == 0
                    && let Some((left_frontier, right_frontier)) =
                        &frontiers
                    && (left_frontier[h][0] != 0
                        || right_frontier[h][0] != 0)
                {
                    return true;
                }

                self.iter().any(
                    |((st, read), &(print, shift, child_st))| {
                        if print != color {
                            return false;
                        }

                        let st = st as usize;
                        let read = read as usize;
                        let child_st = child_st as usize;
                        let child_colors = window_neighbor_mask(
                            st, read, shift, &windows,
                        );
                        let excursions =
                            if shift { &right_any } else { &left_any };

                        excursions.ret_from_mask_possible(
                            co,
                            child_st,
                            child_colors,
                            h,
                        )
                    },
                )
            })
            .collect()
    }

    /// Static target-shape filter for `0+ [color] 0+` blank predecessors.
    ///
    /// `halfblank_slots` tracks the stronger necessary condition that one
    /// whole side is blank in a reachable `(state, scanned color)` checkpoint,
    /// while retaining the exact immediate neighbor on the opposite side.
    /// Every exact blank target must therefore admit inward neighbor 0 in both
    /// the left-blank and right-blank abstractions.
    ///
    /// For a nonblank scanned color, also take the last departure from that
    /// target cell.  The untouched opposite side must already be blank at the
    /// departure, replacing the old weak `control state is reachable` gate.
    /// The departed side must then admit a clean return to the target state.
    fn blank_slots_side_clean(&self) -> Set<Slot> {
        let (forbid_left, forbid_right) = self.shift_side_forbidden();
        let windows =
            self.win_possible_from_blank(&forbid_left, &forbid_right);
        let left_clean = side_excursions(self, &windows, false, true);
        let right_clean = side_excursions(self, &windows, true, true);
        let left_any = side_excursions(self, &windows, false, false);
        let right_any = side_excursions(self, &windows, true, false);

        // false = left side blank:  0+ [color] ?
        // true  = right side blank: ? [color] 0+
        let left_half = halfblank_slots(
            self,
            &windows,
            false,
            &left_clean,
            &right_any,
        );
        let right_half = halfblank_slots(
            self,
            &windows,
            true,
            &right_clean,
            &left_any,
        );

        self.erase_slots()
            .into_iter()
            .filter(|&(state, color)| {
                let h = state as usize;
                let co = color as usize;

                // An exact `0+ [color] 0+` occurrence witnesses both
                // one-sided abstractions on the same concrete run.
                if left_half[h][co] & 1 == 0
                    || right_half[h][co] & 1 == 0
                {
                    return false;
                }

                // A scanned 0 may be a first visit to a fresh blank cell, so
                // there need not be an earlier departure from this cell.
                if color == 0 {
                    return true;
                }

                // For a nonzero scanned color, the cell was written earlier.
                // At its last departure before the target, the opposite side
                // is never touched again and therefore must already be blank.
                for ((st, read), &(print, shift, child_st)) in
                    self.iter()
                {
                    let st = st as usize;
                    let read = read as usize;
                    let child_st = child_st as usize;
                    if print != color {
                        continue;
                    }

                    let (opposite_half, clean) = if shift {
                        // Depart right: left side remains untouched.
                        (&left_half, &right_clean)
                    } else {
                        // Depart left: right side remains untouched.
                        (&right_half, &left_clean)
                    };

                    if opposite_half[st][read] == 0 {
                        continue;
                    }

                    // Do not narrow the clean-return child colors to the
                    // neighbor-aware halfblank mask here. `clean` is a
                    // deliberately restrictive recursive summary; the old
                    // window-level existential join is needed to keep this
                    // static last-departure test conservative. The refined
                    // halfblank mask remains useful for the dynamic tape
                    // filter and for the exact blank-neighbor target gate.
                    let child_colors =
                        window_child_mask(st, read, shift, 0, &windows);

                    if clean.ret_from_mask_possible(
                        co,
                        child_st,
                        child_colors,
                        h,
                    ) {
                        return true;
                    }
                }

                false
            })
            .collect()
    }

    /// Filter one-sided zero targets by reachable halfblank shape:
    ///
    ///  ? [0] 0+  (side = R)
    ///  0+ [0] ?  (side = L)
    fn shifts_side_clean(
        &self,
        shifts: Set<(State, Shift)>,
    ) -> Set<(State, Shift)> {
        let (forbid_left, forbid_right) = self.shift_side_forbidden();
        let windows =
            self.win_possible_from_blank(&forbid_left, &forbid_right);
        let left_clean = side_excursions(self, &windows, false, true);
        let right_clean = side_excursions(self, &windows, true, true);
        let left_any = side_excursions(self, &windows, false, false);
        let right_any = side_excursions(self, &windows, true, false);
        let left_half = halfblank_slots(
            self,
            &windows,
            false,
            &left_clean,
            &right_any,
        );
        let right_half = halfblank_slots(
            self,
            &windows,
            true,
            &right_clean,
            &left_any,
        );

        shifts
            .into_iter()
            .filter(|&(state, side)| {
                let h = state as usize;
                if side {
                    right_half[h][0] != 0
                } else {
                    left_half[h][0] != 0
                }
            })
            .collect()
    }

    fn spinout_shifts_side_clean(&self) -> Set<(State, Shift)> {
        let shifts = self.zr_shifts();
        if shifts.is_empty() {
            return shifts;
        }

        let (forbid_left, forbid_right) = self.shift_side_forbidden();
        let windows =
            self.win_possible_from_blank(&forbid_left, &forbid_right);
        let left_any = side_excursions(self, &windows, false, false);
        let right_any = side_excursions(self, &windows, true, false);
        let left_frontier =
            frontier_slots(self, &windows, false, &right_any);
        let right_frontier =
            frontier_slots(self, &windows, true, &left_any);

        shifts
            .into_iter()
            .filter(|&(state, side)| {
                let h = state as usize;
                if side {
                    right_frontier[h][0] != 0
                } else {
                    left_frontier[h][0] != 0
                }
            })
            .collect()
    }

    fn zloop_shifts_side_clean(&self) -> Set<(State, Shift)> {
        self.shifts_side_clean(self.blank_loops())
    }

    fn cant_blank_by_color_graph(&self) -> bool {
        let clo = color_closure::<S, C>(self);
        let bad = unerasable_mask::<C>(&clo);
        if bad == 0 {
            return false;
        }

        let pr = printed_mask::<S, C>(self);

        (pr & bad) != 0
    }
}

/// BF min distances inside SCC with optional weight negation.
/// If `negate=true`, weights are negated (R=-1, L=+1).
fn bf_min_row_in_scc_weight<const S: usize>(
    mask: u16,
    next: &NextDir<S>,
    src: usize,
    negate: bool,
    out: &mut [i32; S],
) {
    const INF: i32 = 1_000_000;

    *out = [INF; S];
    out[src] = 0;

    let mut nodes = [0; S];
    let mut n = 0;
    for v in 0..S {
        if ((mask >> v) & 1) == 1 {
            nodes[n] = v;
            n += 1;
        }
    }
    if n == 0 {
        return;
    }

    for _ in 0..(n.saturating_sub(1)) {
        let mut changed = false;

        for i in 0..n {
            let u = nodes[i];
            let du = out[u];
            if du == INF {
                continue;
            }

            for dir in 0..2 {
                let mut w = if dir == 1 { 1 } else { -1 };
                if negate {
                    w = -w;
                }

                for &v in &next[u][dir] {
                    if ((mask >> v) & 1) == 0 {
                        continue;
                    }
                    let nv = du + w;
                    if nv < out[v] {
                        out[v] = nv;
                        changed = true;
                    }
                }
            }
        }

        if !changed {
            break;
        }
    }
}

/// Exact check inside a one-sided SCC:
/// Return bitmask of states v in SCC such that there exists a path src -> v
/// with net displacement exactly 0, under weights:
/// - normal: R=+1, L=-1 if negate=false
/// - negated: R=-1, L=+1 if negate=true
///
/// Assumes: under the chosen weight system, SCC has no negative cycles
/// (so min distance is bounded and the explored displacement window is small).
fn zero_disp_reach_mask_one_sided_scc<const S: usize>(
    mask: u16,
    next: &NextDir<S>,
    src: usize,
    negate: bool,
) -> u16 {
    // Compute global lower bound on displacement reachable from src in SCC:
    // min over nodes of shortest path distance (no negative cycles => finite).
    let mut d = [0; S];
    bf_min_row_in_scc_weight::<S>(mask, next, src, negate, &mut d);

    let lo_opt = (0..S)
        .filter(|&v| (mask >> v) & 1 != 0)
        .map(|v| d[v])
        .filter(|&dv| dv < 900_000)
        .min();

    let Some(lo) = lo_opt else { return 0 };

    // We need to search displacements in [lo .. 0].
    // For S<=16 and no negative cycles, lo is typically >= -(S-1) (<= -15).
    // Keep a safe cap; if it somehow exceeds the cap, return
    // conservative "all nodes".
    const CAP: usize = 33; // supports lo down to -32
    let offset = -lo;
    #[expect(clippy::cast_sign_loss)]
    if offset < 0 || (offset as usize) >= CAP {
        // Too wide; don't prune.
        return mask;
    }
    #[expect(clippy::cast_sign_loss)]
    let zero_idx = offset as usize; // index representing displacement 0

    // visited[state][idx] where idx corresponds to disp = lo + idx
    let mut visited = [[false; CAP]; S];

    let mut q = VecDeque::new();
    visited[src][zero_idx] = true;
    q.push_back((src, 0)); // store actual displacement

    while let Some((u, disp)) = q.pop_front() {
        for dir in 0..2 {
            let mut w = if dir == 1 { 1 } else { -1 };
            if negate {
                w = -w;
            }

            for &v in &next[u][dir] {
                if ((mask >> v) & 1) == 0 {
                    continue;
                }
                let nd = disp + w;
                if nd < lo || nd > 0 {
                    continue;
                }
                #[expect(clippy::cast_sign_loss)]
                let idx = (nd - lo) as usize;
                if idx >= CAP || visited[v][idx] {
                    continue;
                }
                visited[v][idx] = true;
                q.push_back((v, nd));
            }
        }
    }

    // Collect targets reachable with displacement exactly 0
    let mut out = 0;
    for v in 0..S {
        if ((mask >> v) & 1) == 0 {
            continue;
        }
        if visited[v][zero_idx] {
            out |= 1 << v;
        }
    }
    out
}

/**************************************/

#[expect(clippy::multiple_inherent_impl)]
impl<const s: usize, const c: usize> Prog<s, c> {
    pub fn is_reversible(&self) -> bool {
        self.get_entrypoints().values().all(|(same, diff)| {
            let mut shift = None;
            let mut seen_print = [false; c];

            for &(_, (print, sh)) in same.iter().chain(diff) {
                match shift {
                    Some(prev) if prev != sh => return false,
                    Some(_) => {},
                    None => shift = Some(sh),
                }

                let print = print as usize;

                if seen_print[print] {
                    return false;
                }

                seen_print[print] = true;
            }

            true
        })
    }
}

#[test]
fn test_is_reversible() {
    assert!(Prog::<2, 2>::from("0RB ...  1LA 1RB").is_reversible());
    assert!(Prog::<2, 2>::from("0RB 0LA  1LA 1RB").is_reversible());
    assert!(!Prog::<2, 2>::from("0RB 1LA  1LA 1RB").is_reversible());

    assert!(
        Prog::<3, 2>::from("0RB ...  0LC 1RA  1RB 1LC").is_reversible()
    );
    assert!(
        Prog::<3, 2>::from("0RB 0RA  0LC 1RA  1RB 1LC").is_reversible()
    );
    assert!(
        !Prog::<3, 2>::from("0RB 0RB  0LC 1RA  1RB 1LC")
            .is_reversible()
    );

    assert!(
        Prog::<4, 2>::from("1RB 0LD  0LC 0RB  1LA 1LD  1LC ...")
            .is_reversible()
    );
    assert!(
        Prog::<4, 2>::from("1RB 0LD  0LC 0RB  1LA 1LD  1LC 0LA")
            .is_reversible()
    );
    assert!(
        !Prog::<4, 2>::from("1RB 0LD  0LC 0RB  1LA 1LD  1LC 1LA")
            .is_reversible()
    );

    assert!(
        Prog::<5, 2>::from(
            "1RB 0RD  1RC 0RB  1RD ...  1LE 1LA  0LE 0LA"
        )
        .is_reversible()
    );
    assert!(
        Prog::<5, 2>::from(
            "1RB 0RD  1RC 0RB  1RD 0RC  1LE 1LA  0LE 0LA"
        )
        .is_reversible()
    );
    assert!(
        !Prog::<5, 2>::from(
            "1RB 0RD  1RC 0RB  1RD 1RC  1LE 1LA  0LE 0LA"
        )
        .is_reversible()
    );

    assert!(
        Prog::<6, 2>::from(
            "1RB 1LD  1LC 1RE  0LD 0LC  0RE 0RF  0RA ...  1RF 1RA"
        )
        .is_reversible()
    );
    assert!(
        Prog::<6, 2>::from(
            "1RB 1LD  1LC 1RE  0LD 0LC  0RE 0RF  0RA 0RB  1RF 1RA"
        )
        .is_reversible()
    );
    assert!(
        !Prog::<6, 2>::from(
            "1RB 1LD  1LC 1RE  0LD 0LC  0RE 0RF  0RA 1RB  1RF 1RA"
        )
        .is_reversible()
    );

    assert!(Prog::<7, 2>::from("1RB 1LD  0LC 0LD  1LC 1LA  0LA 1RE  0RF 0RE  0RG 1RF  0RB ...").is_reversible());
    assert!(Prog::<7, 2>::from("1RB 1LD  0LC 0LD  1LC 1LA  0LA 1RE  0RF 0RE  0RG 1RF  0RB 1RG").is_reversible());
    assert!(!Prog::<7, 2>::from("1RB 1LD  0LC 0LD  1LC 1LA  0LA 1RE  0RF 0RE  0RG 1RF  0RB 1LG").is_reversible());
}
