use core::{
    fmt,
    hash::{Hash, Hasher as _},
    iter::once,
};

use ahash::{AHashMap as Dict, AHashSet as Set, AHasher};

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
    windows: Vec<WindowSideSummary<C>>,
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
        Self {
            windows: vec![WindowSideSummary::empty(); S * C * C * C],
        }
    }

    fn window(
        &self,
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
    ) -> &WindowSideSummary<C> {
        &self.windows[Self::index(st, scan, left, right)]
    }

    fn window_mut(
        &mut self,
        st: usize,
        scan: usize,
        left: usize,
        right: usize,
    ) -> &mut WindowSideSummary<C> {
        let index = Self::index(st, scan, left, right);
        &mut self.windows[index]
    }
}

/// Near-to-far run prefix of the tape strictly beyond one immediate neighbor.
///
/// Two complete runs keep the original 1/2/3/4+ count precision.  One extra
/// spill run remembers the next run color and only whether its length is 1 or
/// 2+.  This delays the old collapse to `dirty_unknown()` by one run boundary
/// without paying for a full third precise run.
///
/// The spill's `farther_dirty` bit is exact for each alternative: false means
/// everything after that spill run is blank; true means there is definitely a
/// nonblank cell farther out.  When that distinction is not known, the
/// forward worklist retains both alternatives.  `DirtyUnknown` is used only
/// after the spill itself has been consumed/lost.
const SIDE_PREFIX_MANY: u8 = 4;
const SIDE_PREFIX_SPILL_MANY: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct SidePrefixRun {
    color: Color,
    count: u8,
}

impl SidePrefixRun {
    const EMPTY: Self = Self { color: 0, count: 0 };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum SidePrefixSpill {
    Blank,
    Run {
        color: Color,
        // 1 is exact; 2 means two-or-more.
        count: u8,
        // Some nonblank cell exists strictly beyond this spill run.
        farther_dirty: bool,
    },
    // The forgotten remainder is definitely dirty, but its next run is lost.
    DirtyUnknown,
}

impl SidePrefixSpill {
    const fn definitely_dirty(self) -> bool {
        match self {
            Self::Blank => false,
            Self::Run {
                color,
                farther_dirty,
                ..
            } => color != 0 || farther_dirty,
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

    #[cfg(test)]
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

    /// Denotational subsumption used by the regression tests for the broad
    /// antichain state. No structural-prefix heuristic is used.
    #[cfg(test)]
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
                farther_dirty: false,
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

    fn spill_after_dropped(
        dropped: SidePrefixRun,
        old: SidePrefixSpill,
    ) -> SidePrefixSpill {
        let count = if dropped.count == 1 {
            1
        } else {
            SIDE_PREFIX_SPILL_MANY
        };

        SidePrefixSpill::Run {
            color: dropped.color,
            count,
            farther_dirty: old.definitely_dirty(),
        }
    }

    const fn suffix_after_spill(
        farther_dirty: bool,
    ) -> SidePrefixSpill {
        if farther_dirty {
            SidePrefixSpill::DirtyUnknown
        } else {
            SidePrefixSpill::Blank
        }
    }

    /// Prepend one exact cell at the near end of the represented tail.
    /// Successors are emitted directly to avoid allocating a temporary `Vec`
    /// for every abstract edge in the fixed point.
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
                    farther_dirty,
                } => {
                    if color == spill_color {
                        // The exact prepended cell merges into the known spill
                        // run.  1 becomes exact 2; 2+ becomes either exact 3
                        // or 4+, which is exactly the old full-run cap.
                        let suffix =
                            Self::suffix_after_spill(farther_dirty);
                        if count == 1 {
                            emit(Self {
                                runs: [
                                    SidePrefixRun { color, count: 2 },
                                    SidePrefixRun::EMPTY,
                                ],
                                len: 1,
                                spill: suffix,
                            });
                        } else {
                            debug_assert_eq!(
                                count,
                                SIDE_PREFIX_SPILL_MANY
                            );
                            for next_count in [3, SIDE_PREFIX_MANY] {
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
                        }
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
                    for count in 1..=SIDE_PREFIX_MANY {
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
        if out.runs[0].color == color {
            out.runs[0].count =
                (out.runs[0].count + 1).min(SIDE_PREFIX_MANY);
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
                SidePrefixSpill::Run {
                    color,
                    count,
                    farther_dirty,
                } => {
                    if count == 1 {
                        emit(
                            color,
                            Self {
                                runs: [SidePrefixRun::EMPTY; 2],
                                len: 0,
                                spill: Self::suffix_after_spill(
                                    farther_dirty,
                                ),
                            },
                        );
                    } else {
                        debug_assert_eq!(count, SIDE_PREFIX_SPILL_MANY);
                        // Removing one cell from 2+ leaves either exactly one
                        // or still 2+ cells of the same spill run.
                        for next_count in [1, SIDE_PREFIX_SPILL_MANY] {
                            let mut next = Self {
                                runs: [SidePrefixRun::EMPTY; 2],
                                len: 0,
                                spill: SidePrefixSpill::Run {
                                    color,
                                    count: next_count,
                                    farther_dirty,
                                },
                            };
                            next.canonicalize();
                            emit(color, next);
                        }
                    }
                },
                SidePrefixSpill::DirtyUnknown => {
                    // The forgotten remainder is dirty. Consuming a zero cannot
                    // remove that dirt. Consuming a nonzero may remove the last
                    // nonblank or may leave more dirt farther out.
                    for color in 0..C {
                        #[expect(clippy::cast_possible_truncation)]
                        let color = color as Color;
                        if color == 0 {
                            emit(color, Self::dirty_unknown());
                        } else {
                            emit(color, Self::blank());
                            emit(color, Self::dirty_unknown());
                        }
                    }
                },
            }
            return;
        }

        let color = self.runs[0].color;
        let count = self.runs[0].count;

        let residual = |new_count: Option<u8>| {
            let mut next = self;
            match new_count {
                Some(count) => next.runs[0].count = count,
                None => {
                    next.runs[0] = next.runs[1];
                    next.runs[1] = SidePrefixRun::EMPTY;
                    next.len -= 1;
                },
            }
            next.canonicalize();
            next
        };

        match count {
            1 => emit(color, residual(None)),
            2 | 3 => emit(color, residual(Some(count - 1))),
            SIDE_PREFIX_MANY => {
                emit(color, residual(Some(3)));
                emit(color, residual(Some(SIDE_PREFIX_MANY)));
            },
            _ => unreachable!(),
        }
    }
}

/// Same exact-window conditioning as `SidePossible`, but keeps alternatives
/// instead of unioning their ordered near-tail run structure.
const SIDE_PREFIX_HAS_BLANK: u8 = 1;
const SIDE_PREFIX_HAS_DIRTY_UNKNOWN: u8 = 2;
const SIDE_PREFIX_UNCONSTRAINED: u8 =
    SIDE_PREFIX_HAS_BLANK | SIDE_PREFIX_HAS_DIRTY_UNKNOWN;

struct SidePrefixPossible<const S: usize, const C: usize> {
    // Flattened [window][side] -> reachable prefix alternatives.
    windows: Vec<Vec<SidePrefix>>,

    // Cached membership of the two broad alternatives. Besides making query
    // time O(1), this avoids scanning an antichain on every forward insertion
    // just to discover that `dirty_unknown()` already subsumes the candidate.
    flags: Vec<u8>,
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

    fn new() -> Self {
        let len = S * C * C * C * 2;
        Self {
            windows: (0..len).map(|_| Vec::new()).collect(),
            flags: vec![0; len],
        }
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
    // expensive cycle pass. The retry keeps exact tape/count semantics and
    // records recurring predecessor edges. When one particular edge would
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
    let win_possible =
        prog.win_possible_from_blank(&forbid_left, &forbid_right);
    let side_possible = prog.side_possible_from_blank(&win_possible);
    let blank_side_possible =
        blank_side_possible_from_blank(prog, &win_possible);
    let color_tail_count =
        color_tail_count_from_blank(prog, &win_possible);
    let pair_tail_presence =
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

    // Side-prefix propagation is substantially more expensive than the older
    // summaries. Build it only after every pre-existing target filter has
    // failed to refute the query, so easy programs pay no prefix fixed-point
    // cost at all.
    let side_prefix_possible = prog
        .side_prefix_possible_from_blank(&win_possible, &side_possible);
    configs.retain(|Config { state, tape }| {
        tape.obeys_side_prefix_possible(*state, &side_prefix_possible)
    });

    if configs.is_empty() {
        return Refuted(0);
    }

    let mut blanks = get_blanks(&configs);

    // In cycle mode, exact repeats are ordinary graph cycles and can be
    // discarded immediately. Growing-count cycles are handled more narrowly:
    // the retry records exact predecessor *edges* and only cuts an edge when
    // that very edge would overflow a run after a long, stable recurrence.
    // No tape is widened and no whole-frontier periodicity is assumed.
    let mut cycle_seen: Option<Dict<(State, u64), Vec<Tape>>> =
        CYCLE_ANALYSIS.then(Dict::new);
    let mut overflow_cycle_history = OverflowCycleHistory::default();

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

        let stepped = match step_configs::<s, c, CYCLE_ANALYSIS>(
            valid_steps,
            step,
            &mut overflow_cycle_history,
            &mut blanks,
            &win_possible,
            &side_possible,
            &side_prefix_possible,
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
    side_possible: &SidePossible<s, c>,
    side_prefix_possible: &SidePrefixPossible<s, c>,
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
            || !tape.obeys_state_side(state, side_possible)
            || !tape
                .obeys_blank_side_possible(state, blank_side_possible)
            || !tape.obeys_tail_presence(
                state,
                color_tail_count,
                pair_tail_presence,
            )
            || !tape
                .obeys_side_prefix_possible(state, side_prefix_possible)
        {
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
    side_possible: &SidePossible<s, c>,
    side_prefix_possible: &SidePrefixPossible<s, c>,
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
                side_possible,
                side_prefix_possible,
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
                side_possible,
                side_prefix_possible,
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
            side_possible,
            side_prefix_possible,
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
        clippy::excessive_nesting
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
        // Four-bit mask per abstract window state.  Each bit is one exact
        // `(left-side parity, right-side parity)` combination.  Total support
        // parity is derived from those two bits plus the scanned color.
        let mut visited = vec![0_u8; total];
        let mut q = std::collections::VecDeque::new();

        // Start from true blank: both whole sides have even nonblank parity.
        q.push_back((0, 1, 0, 0, 0, 1, 0_u8));
        visited[idx::<c, s>(0, 1, 0, 0, 0, 1)] = 0b0001;

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

        while let Some((st, lb, l, sc, r, rb, side_parity)) =
            q.pop_front()
        {
            possible.right[st][sc][l] |= 1_u64 << r;
            possible.left[st][sc][r] |= 1_u64 << l;
            possible.any[st][sc] = true;

            let left_parity = side_parity & 1;
            let right_parity = (side_parity >> 1) & 1;
            let total_parity =
                left_parity ^ right_parity ^ u8::from(sc != 0);
            let parity_bit = 1_u8 << total_parity;
            let side_parity_bit = 1_u8 << side_parity;
            let parity_index =
                WinPossible::<s, c>::parity_index(st, sc, l, r);

            possible.parity[parity_index] |= parity_bit;
            possible.parity_right[st][sc][l] |= parity_bit;
            possible.parity_left[st][sc][r] |= parity_bit;
            possible.parity_any[st][sc] |= parity_bit;

            possible.side_parity[parity_index] |= side_parity_bit;
            possible.side_parity_right[st][sc][l] |= side_parity_bit;
            possible.side_parity_left[st][sc][r] |= side_parity_bit;
            possible.side_parity_any[st][sc] |= side_parity_bit;

            let st_state = st as State;
            let sc_color = sc as Color;

            let Some(&(print, shift, next_state)) =
                self.get(&(st_state, sc_color))
            else {
                // Missing transition: halting sink.
                continue;
            };

            let p = print as usize;
            let ns = next_state as usize;

            if shift {
                // Move Right.
                // New: left neighbor becomes printed symbol p, scanned becomes old r.
                // The new left tail starts at old l, so it remains known blank exactly
                // when old l is blank and the old farther-left tail was known blank.
                let new_lb = usize::from(lb == 1 && l == 0);
                let next_left_parity = left_parity ^ u8::from(p != 0);
                let next_right_parity = right_parity ^ u8::from(r != 0);
                let next_side_parity =
                    next_left_parity | (next_right_parity << 1);
                let next_side_parity_bit = 1_u8 << next_side_parity;

                if rb == 1 {
                    // The old right tail starts at the newly exposed cell, so both
                    // that cell and everything beyond it are known blank.  This does
                    // not depend on the old right neighbor r.
                    let n = (ns, new_lb, p, r, 0, 1);
                    let id = idx::<c, s>(n.0, n.1, n.2, n.3, n.4, n.5);
                    if visited[id] & next_side_parity_bit == 0 {
                        visited[id] |= next_side_parity_bit;
                        q.push_back((
                            n.0,
                            n.1,
                            n.2,
                            n.3,
                            n.4,
                            n.5,
                            next_side_parity,
                        ));
                    }
                } else {
                    // The newly exposed cell is unknown; conservatively allow any
                    // right-side color and drop right-tail certainty.
                    for new_r in 0..c {
                        if forbid_right[new_r] {
                            continue;
                        }
                        let n = (ns, new_lb, p, r, new_r, 0);
                        let id =
                            idx::<c, s>(n.0, n.1, n.2, n.3, n.4, n.5);
                        if visited[id] & next_side_parity_bit == 0 {
                            visited[id] |= next_side_parity_bit;
                            q.push_back((
                                n.0,
                                n.1,
                                n.2,
                                n.3,
                                n.4,
                                n.5,
                                next_side_parity,
                            ));
                        }
                    }
                }
            } else {
                // Move Left.  Symmetrically, the new right tail starts at old r.
                let new_rb = usize::from(rb == 1 && r == 0);
                let next_left_parity = left_parity ^ u8::from(l != 0);
                let next_right_parity = right_parity ^ u8::from(p != 0);
                let next_side_parity =
                    next_left_parity | (next_right_parity << 1);
                let next_side_parity_bit = 1_u8 << next_side_parity;

                if lb == 1 {
                    // The old left tail starts at the newly exposed cell, so that
                    // cell and everything beyond it are known blank.
                    let n = (ns, 1, 0, l, p, new_rb);
                    let id = idx::<c, s>(n.0, n.1, n.2, n.3, n.4, n.5);
                    if visited[id] & next_side_parity_bit == 0 {
                        visited[id] |= next_side_parity_bit;
                        q.push_back((
                            n.0,
                            n.1,
                            n.2,
                            n.3,
                            n.4,
                            n.5,
                            next_side_parity,
                        ));
                    }
                } else {
                    // The newly exposed cell is unknown; conservatively allow any
                    // left-side color and drop left-tail certainty.
                    for new_l in 0..c {
                        if forbid_left[new_l] {
                            continue;
                        }
                        let n = (ns, 0, new_l, l, p, new_rb);
                        let id =
                            idx::<c, s>(n.0, n.1, n.2, n.3, n.4, n.5);
                        if visited[id] & next_side_parity_bit == 0 {
                            visited[id] |= next_side_parity_bit;
                            q.push_back((
                                n.0,
                                n.1,
                                n.2,
                                n.3,
                                n.4,
                                n.5,
                                next_side_parity,
                            ));
                        }
                    }
                }
            }
        }

        // A second tiny worklist retains joint left/right nonblank counts
        // modulo 3.  It is intentionally separate from side parity: joining
        // the two abstractions independently costs 4 + 9 residue states per
        // abstract window instead of 36, while preserving every existing
        // parity rejection and adding the mod-3 rejection on top.
        let mut mod3_visited = vec![0_u16; total];
        let mut mod3_q = std::collections::VecDeque::new();
        mod3_q.push_back((0, 1, 0, 0, 0, 1, 0_u8));
        mod3_visited[idx::<c, s>(0, 1, 0, 0, 0, 1)] = 1;

        while let Some((st, lb, l, sc, r, rb, side_mod3)) =
            mod3_q.pop_front()
        {
            let left_residue = side_mod3 % 3;
            let right_residue = side_mod3 / 3;
            let residue_bit = 1_u16 << side_mod3;
            let residue_index =
                WinPossible::<s, c>::parity_index(st, sc, l, r);

            possible.side_mod3[residue_index] |= residue_bit;
            possible.side_mod3_right[st][sc][l] |= residue_bit;
            possible.side_mod3_left[st][sc][r] |= residue_bit;
            possible.side_mod3_any[st][sc] |= residue_bit;

            let st_state = st as State;
            let sc_color = sc as Color;
            let Some(&(print, shift, next_state)) =
                self.get(&(st_state, sc_color))
            else {
                continue;
            };

            let p = print as usize;
            let ns = next_state as usize;

            if shift {
                let new_lb = usize::from(lb == 1 && l == 0);
                let next_left = (left_residue + u8::from(p != 0)) % 3;
                let next_right =
                    (right_residue + 3 - u8::from(r != 0)) % 3;
                let next_code = next_left + 3 * next_right;
                let next_bit = 1_u16 << next_code;

                if rb == 1 {
                    let n = (ns, new_lb, p, r, 0, 1);
                    let id = idx::<c, s>(n.0, n.1, n.2, n.3, n.4, n.5);
                    if mod3_visited[id] & next_bit == 0 {
                        mod3_visited[id] |= next_bit;
                        mod3_q.push_back((
                            n.0, n.1, n.2, n.3, n.4, n.5, next_code,
                        ));
                    }
                } else {
                    for new_r in 0..c {
                        if forbid_right[new_r] {
                            continue;
                        }
                        let n = (ns, new_lb, p, r, new_r, 0);
                        let id =
                            idx::<c, s>(n.0, n.1, n.2, n.3, n.4, n.5);
                        if mod3_visited[id] & next_bit == 0 {
                            mod3_visited[id] |= next_bit;
                            mod3_q.push_back((
                                n.0, n.1, n.2, n.3, n.4, n.5, next_code,
                            ));
                        }
                    }
                }
            } else {
                let new_rb = usize::from(rb == 1 && r == 0);
                let next_left =
                    (left_residue + 3 - u8::from(l != 0)) % 3;
                let next_right = (right_residue + u8::from(p != 0)) % 3;
                let next_code = next_left + 3 * next_right;
                let next_bit = 1_u16 << next_code;

                if lb == 1 {
                    let n = (ns, 1, 0, l, p, new_rb);
                    let id = idx::<c, s>(n.0, n.1, n.2, n.3, n.4, n.5);
                    if mod3_visited[id] & next_bit == 0 {
                        mod3_visited[id] |= next_bit;
                        mod3_q.push_back((
                            n.0, n.1, n.2, n.3, n.4, n.5, next_code,
                        ));
                    }
                } else {
                    for new_l in 0..c {
                        if forbid_left[new_l] {
                            continue;
                        }
                        let n = (ns, 0, new_l, l, p, new_rb);
                        let id =
                            idx::<c, s>(n.0, n.1, n.2, n.3, n.4, n.5);
                        if mod3_visited[id] & next_bit == 0 {
                            mod3_visited[id] |= next_bit;
                            mod3_q.push_back((
                                n.0, n.1, n.2, n.3, n.4, n.5, next_code,
                            ));
                        }
                    }
                }
            }
        }

        // Global per-color parity is a separate product component.  Keeping it
        // independent from side parity/mod-3 avoids multiplying all residue
        // domains together.  A transition changes global tape composition only
        // by replacing the scanned color `sc` with `print`; head movement merely
        // changes which existing cell is scanned.
        if WinPossible::<s, c>::color_parity_enabled() {
            let mut color_visited = vec![0_u64; total];
            let mut color_q = std::collections::VecDeque::new();
            color_q.push_back((0, 1, 0, 0, 0, 1, 0_u8));
            color_visited[idx::<c, s>(0, 1, 0, 0, 0, 1)] = 1;

            while let Some((st, lb, l, sc, r, rb, color_parity)) =
                color_q.pop_front()
            {
                let vector_bit = 1_u64 << color_parity;
                let vector_index =
                    WinPossible::<s, c>::parity_index(st, sc, l, r);

                possible.color_parity[vector_index] |= vector_bit;
                possible.color_parity_right[st][sc][l] |= vector_bit;
                possible.color_parity_left[st][sc][r] |= vector_bit;
                possible.color_parity_any[st][sc] |= vector_bit;

                let st_state = st as State;
                let sc_color = sc as Color;
                let Some(&(print, shift, next_state)) =
                    self.get(&(st_state, sc_color))
                else {
                    continue;
                };

                let p = print as usize;
                let ns = next_state as usize;
                let mut next_color_parity = color_parity;
                if sc != 0 {
                    next_color_parity ^= 1_u8 << (sc - 1);
                }
                if p != 0 {
                    next_color_parity ^= 1_u8 << (p - 1);
                }
                let next_vector_bit = 1_u64 << next_color_parity;

                if shift {
                    let new_lb = usize::from(lb == 1 && l == 0);

                    if rb == 1 {
                        let n = (ns, new_lb, p, r, 0, 1);
                        let id =
                            idx::<c, s>(n.0, n.1, n.2, n.3, n.4, n.5);
                        if color_visited[id] & next_vector_bit == 0 {
                            color_visited[id] |= next_vector_bit;
                            color_q.push_back((
                                n.0,
                                n.1,
                                n.2,
                                n.3,
                                n.4,
                                n.5,
                                next_color_parity,
                            ));
                        }
                    } else {
                        for new_r in 0..c {
                            if forbid_right[new_r] {
                                continue;
                            }
                            let n = (ns, new_lb, p, r, new_r, 0);
                            let id = idx::<c, s>(
                                n.0, n.1, n.2, n.3, n.4, n.5,
                            );
                            if color_visited[id] & next_vector_bit == 0
                            {
                                color_visited[id] |= next_vector_bit;
                                color_q.push_back((
                                    n.0,
                                    n.1,
                                    n.2,
                                    n.3,
                                    n.4,
                                    n.5,
                                    next_color_parity,
                                ));
                            }
                        }
                    }
                } else {
                    let new_rb = usize::from(rb == 1 && r == 0);

                    if lb == 1 {
                        let n = (ns, 1, 0, l, p, new_rb);
                        let id =
                            idx::<c, s>(n.0, n.1, n.2, n.3, n.4, n.5);
                        if color_visited[id] & next_vector_bit == 0 {
                            color_visited[id] |= next_vector_bit;
                            color_q.push_back((
                                n.0,
                                n.1,
                                n.2,
                                n.3,
                                n.4,
                                n.5,
                                next_color_parity,
                            ));
                        }
                    } else {
                        for new_l in 0..c {
                            if forbid_left[new_l] {
                                continue;
                            }
                            let n = (ns, 0, new_l, l, p, new_rb);
                            let id = idx::<c, s>(
                                n.0, n.1, n.2, n.3, n.4, n.5,
                            );
                            if color_visited[id] & next_vector_bit == 0
                            {
                                color_visited[id] |= next_vector_bit;
                                color_q.push_back((
                                    n.0,
                                    n.1,
                                    n.2,
                                    n.3,
                                    n.4,
                                    n.5,
                                    next_color_parity,
                                ));
                            }
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
            let initial = possible.window_mut(0, 0, 0, 0);
            initial.reachable = true;
            for side in [LEFT_SIDE, RIGHT_SIDE] {
                initial.colors[side] = 1;
                initial.pairs[side][0] = 1;
            }
        }

        #[expect(clippy::useless_let_if_seq)]
        fn merge<const C: usize>(
            target: &mut WindowSideSummary<C>,
            source: WindowSideSummary<C>,
            push_side: usize,
            print: usize,
            old_neighbor: usize,
            target_left: usize,
            target_right: usize,
        ) -> bool {
            let mut changed = false;

            if !target.reachable {
                target.reachable = true;
                changed = true;
            }

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
            // sides.  Usually these bits are already inherited, but setting
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
        let mut queued = vec![false; possible.windows.len()];
        let mut q = VecDeque::new();
        queued[initial_index] = true;
        q.push_back((0_usize, 0_usize, 0_usize, 0_usize));

        while let Some((st, sc, left, right)) = q.pop_front() {
            let index =
                SidePossible::<s, c>::index(st, sc, left, right);
            queued[index] = false;

            let source = possible.windows[index];
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
                        &mut possible.windows[target_index],
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
                        &mut possible.windows[target_index],
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
    fn side_prefix_possible_from_blank(
        &self,
        windows: &WinPossible<s, c>,
        sides: &SidePossible<s, c>,
    ) -> SidePrefixPossible<s, c> {
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

        let mut possible = SidePrefixPossible::new();
        let mut q = VecDeque::new();

        #[expect(clippy::shadow_unrelated)]
        let push =
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
                        || possible.windows[index].contains(&prefix)
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

// Expensive growing-edge history used only after the ordinary pass has
// actually reached CountLimit. The retry never widens counts. Instead, it
// remembers exact predecessor edges that increase the near run on the push
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
    left: Vec<RunSig>,
    right: Vec<RunSig>,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct RunSig {
    color: Color,
    count: usize,
    indef: bool,
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

fn span_runs(span: &Span) -> Vec<RunSig> {
    span.span
        .iter()
        .map(|block| match block.count {
            BlockCount::Exact(count) => RunSig {
                color: block.color,
                count: usize::from(count),
                indef: false,
            },
            BlockCount::AtLeast(count) => RunSig {
                color: block.color,
                count: usize::from(count),
                indef: true,
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
    if block.color != tape.scan {
        return None;
    }

    let count = block.count.minimum();
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
    debug_assert_eq!(nearest.color, tape.scan);
    nearest.count = 0;

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

    const fn minimum(self) -> Count {
        match self {
            Self::Exact(count) | Self::AtLeast(count) => count,
        }
    }

    const fn is_single(self) -> bool {
        matches!(self, Self::Exact(1))
    }

    const fn is_indef(self) -> bool {
        matches!(self, Self::AtLeast(_))
    }

    const fn can_be_one(self) -> bool {
        matches!(self, Self::AtLeast(1))
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
        *self = Self::AtLeast(count);
        Ok(())
    }

    /// Remove one definitely present cell.
    ///
    /// `AtLeast(1)` is used only on the residual branch where the concrete
    /// run had length at least two, so its residual is again `AtLeast(1)`.
    const fn decrement_after_pull(&mut self) {
        *self = match *self {
            Self::Exact(count) => {
                debug_assert!(count > 1);
                Self::Exact(count - 1)
            },
            Self::AtLeast(1) => Self::AtLeast(1),
            Self::AtLeast(count) => Self::AtLeast(count - 1),
        };
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct Block {
    color: Color,
    count: BlockCount,
}

impl Block {
    const fn exact(color: Color, count: Count) -> Self {
        Self {
            color,
            count: BlockCount::exact(count),
        }
    }

    const fn at_least(color: Color, count: Count) -> Self {
        Self {
            color,
            count: BlockCount::at_least(count),
        }
    }

    const fn blank(&self) -> bool {
        self.color == 0
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.count {
            BlockCount::Exact(1) => write!(f, "{}", self.color),
            BlockCount::Exact(count) => {
                write!(f, "{}^{count}", self.color)
            },
            BlockCount::AtLeast(1) => write!(f, "{}..", self.color),
            BlockCount::AtLeast(count) => {
                write!(f, "{}^{count}..", self.color)
            },
        }
    }
}

/// Minimal near-head span implementation for BKW.  Storage matches the shared
/// tape span: farthest block first, nearest block last.
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

    fn str_iter(&self) -> impl DoubleEndedIterator<Item = String> + '_ {
        self.iter().map(ToString::to_string)
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
        if let Some(block) = self.first_mut()
            && block.color == color
        {
            return block.count.add_exact(count);
        }

        self.blocks.push(Block::exact(color, count));
        Ok(())
    }

    fn push_at_least(
        &mut self,
        color: Color,
        count: Count,
    ) -> Result<(), BackwardResult> {
        if let Some(block) = self.first_mut()
            && block.color == color
        {
            return block.count.add_at_least(count);
        }

        self.blocks.push(Block::at_least(color, count));
        Ok(())
    }

    fn push_block(
        &mut self,
        block: &Block,
    ) -> Result<(), BackwardResult> {
        match block.count {
            BlockCount::Exact(count) => {
                self.push_exact(block.color, count)
            },
            BlockCount::AtLeast(count) => {
                self.push_at_least(block.color, count)
            },
        }
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
            |block| block.color == print,
        )
    }

    fn pull(&mut self) {
        let Some(block) = self.span.first_mut() else {
            return;
        };

        if block.count.is_single() {
            self.span.pop_block();
        } else {
            block.count.decrement_after_pull();
        }
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

        self.span.push_exact(color, 1)
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
        debug_assert!(block.count.can_be_one());
        block.count = BlockCount::Exact(1);
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

        // Collect blocks (ordered near->far) and drop blanks from the far end.
        let mut blocks: Vec<Block> =
            self.span.iter().cloned().collect();
        while matches!(blocks.last(), Some(b) if b.color == 0) {
            blocks.pop();
        }

        if blocks.len() == self.span.len() {
            return;
        }

        // Rebuild span by pushing blocks from far->near (push_block is near-end).
        let mut new_span = SpanT::init_blank();
        for b in blocks.into_iter().rev() {
            new_span.push_block(&b).expect(
                "canonical span rebuild cannot increase counts",
            );
        }
        self.span = new_span;
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
                .str_iter()
                .rev()
                .chain(once(format!("[{}]", self.scan)))
                .chain(self.rspan.span.str_iter())
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
            if span.span.iter().any(|block| block.color != 0) {
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
        /// Possible capped counts of `color` strictly beyond the immediate
        /// neighbor represented by `span`.
        ///
        /// Bit 0 = zero, bit 1 = exactly one, bit 2 = at least two.
        fn side_count_masks<const C: usize>(span: &Span) -> [u8; C] {
            let mut minimum = [0_u8; C];
            let mut variable = [span.end == TapeEnd::Unknown; C];

            for (index, block) in span.span.iter().enumerate() {
                let color = block.color as usize;
                if color == 0 {
                    continue;
                }

                // The first explicit block contains the immediate neighbor.
                // Remove exactly that nearest cell from the tail count.
                let contribution = if index == 0 {
                    block.count.minimum().saturating_sub(1)
                } else {
                    block.count.minimum()
                };

                minimum[color] =
                    minimum[color].saturating_add(contribution).min(2);
                variable[color] |= block.count.is_indef();
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

        let left_counts = side_count_masks::<C>(&self.lspan);
        let right_counts = side_count_masks::<C>(&self.rspan);
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

    #[cfg(test)]
    fn obeys_color_tail_count<const S: usize, const C: usize>(
        &self,
        state: State,
        possible: &ColorTailCountPossible<S, C>,
    ) -> bool {
        fn side_count_masks<const C: usize>(span: &Span) -> [u8; C] {
            let mut minimum = [0_u8; C];
            let mut variable = [span.end == TapeEnd::Unknown; C];

            for (index, block) in span.span.iter().enumerate() {
                let color = block.color as usize;
                if color == 0 {
                    continue;
                }

                let contribution = if index == 0 {
                    block.count.minimum().saturating_sub(1)
                } else {
                    block.count.minimum()
                };
                minimum[color] =
                    minimum[color].saturating_add(contribution).min(2);
                variable[color] |= block.count.is_indef();
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

        const fn allowed_mask(left: u8, right: u8) -> u16 {
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

        let st = state as usize;
        let sc = self.scan as usize;
        let left_neighbor = self.left_neighbor_color().map(usize::from);
        let right_neighbor =
            self.right_neighbor_color().map(usize::from);

        let left_counts = side_count_masks::<C>(&self.lspan);
        let right_counts = side_count_masks::<C>(&self.rspan);

        for color in 1..C {
            let required =
                allowed_mask(left_counts[color], right_counts[color]);
            if required != 0x01ff
                && possible.mask(
                    st,
                    sc,
                    left_neighbor,
                    right_neighbor,
                    color,
                ) & required
                    == 0
            {
                return false;
            }
        }

        true
    }

    #[cfg(test)]
    fn obeys_pair_tail_presence<const S: usize, const C: usize>(
        &self,
        state: State,
        possible: &PairTailPresencePossible<S, C>,
    ) -> bool {
        #[derive(Clone, Copy)]
        enum RequiredPresence {
            Absent,
            Present,
            Unknown,
        }

        fn requirement(span: &Span, color: usize) -> RequiredPresence {
            let mut first = true;
            let mut nearest_may_extend = false;
            for block in span.span.iter() {
                if first {
                    first = false;
                    if block.color as usize != color {
                        continue;
                    }
                    if block.count.minimum() > 1 {
                        return RequiredPresence::Present;
                    }
                    nearest_may_extend = block.count.is_indef();
                    continue;
                }
                if block.color as usize == color {
                    return RequiredPresence::Present;
                }
            }
            if span.end == TapeEnd::Unknown || nearest_may_extend {
                RequiredPresence::Unknown
            } else {
                RequiredPresence::Absent
            }
        }

        const fn matches(
            required: RequiredPresence,
            present: bool,
        ) -> bool {
            match required {
                RequiredPresence::Absent => !present,
                RequiredPresence::Present => present,
                RequiredPresence::Unknown => true,
            }
        }

        if C < 3 {
            return true;
        }

        let st = state as usize;
        let sc = self.scan as usize;
        let left_neighbor = self.left_neighbor_color().map(usize::from);
        let right_neighbor =
            self.right_neighbor_color().map(usize::from);
        let mut left_required = [RequiredPresence::Unknown; C];
        let mut right_required = [RequiredPresence::Unknown; C];
        let mut constrained = [false; C];

        for color in 1..C {
            left_required[color] = requirement(&self.lspan, color);
            right_required[color] = requirement(&self.rspan, color);
            constrained[color] = !matches!(
                (left_required[color], right_required[color]),
                (RequiredPresence::Unknown, RequiredPresence::Unknown)
            );
        }

        for a in 1..C {
            if !constrained[a] {
                continue;
            }
            for b in (a + 1)..C {
                if !constrained[b] {
                    continue;
                }

                let mut allowed = 0_u16;
                for status in 0..16_u8 {
                    if matches(left_required[a], status & 1 != 0)
                        && matches(right_required[a], status & 2 != 0)
                        && matches(left_required[b], status & 4 != 0)
                        && matches(right_required[b], status & 8 != 0)
                    {
                        allowed |= 1_u16 << status;
                    }
                }

                if possible.mask(
                    st,
                    sc,
                    left_neighbor,
                    right_neighbor,
                    a,
                    b,
                ) & allowed
                    == 0
                {
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
            if span.end == TapeEnd::Unknown {
                return 0b11;
            }

            let mut parity = 0_u8;
            for block in span.span.iter() {
                if block.color == 0 {
                    continue;
                }
                if block.count.is_indef() {
                    return 0b11;
                }
                parity ^= u8::from(block.count.minimum() & 1 != 0);
            }

            1_u8 << parity
        }

        (span_mask(&self.lspan), span_mask(&self.rspan))
    }

    /// Return possible nonblank-count residues modulo 3 for the left and right
    /// spans separately. Bits 0..=2 correspond to residues 0..=2. Unknown
    /// ends and indefinite nonblank runs permit every residue.
    fn side_nonblank_mod3_masks(&self) -> (u8, u8) {
        fn span_mask(span: &Span) -> u8 {
            if span.end == TapeEnd::Unknown {
                return 0b111;
            }

            let mut residue = 0_u8;
            for block in span.span.iter() {
                if block.color == 0 {
                    continue;
                }
                if block.count.is_indef() {
                    return 0b111;
                }
                residue = (residue + block.count.minimum() % 3) % 3;
            }

            1_u8 << residue
        }

        (span_mask(&self.lspan), span_mask(&self.rspan))
    }

    /// Return the possible global per-color parity vectors.  Vector bit
    /// `color - 1` is the parity of the number of cells of that nonblank color;
    /// the returned u64 is a bitset over those vectors. Unknown tape ends make
    /// every vector possible. An indefinite run makes only its own color bit
    /// unknown, preserving exact parity information for the other colors.
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

        let mut vector = 0_u8;
        let mut unknown_bits = 0_u8;

        let mut add_color =
            |color: Color, odd: bool, indefinite: bool| {
                if color == 0 {
                    return;
                }
                let bit = 1_u8 << (color as usize - 1);
                if indefinite {
                    unknown_bits |= bit;
                } else if odd {
                    vector ^= bit;
                }
            };

        add_color(self.scan, true, false);
        for span in [&self.lspan, &self.rspan] {
            for block in span.span.iter() {
                add_color(
                    block.color,
                    block.count.minimum() & 1 != 0,
                    block.count.is_indef(),
                );
            }
        }

        let mut out = 0_u64;
        let mut subset = unknown_bits;
        loop {
            out |= 1_u64 << (vector ^ subset);
            if subset == 0 {
                break;
            }
            subset = (subset - 1) & unknown_bits;
        }
        out
    }

    /// Return the possible parities of the total number of nonblank cells.
    /// Bit 0 means even is possible; bit 1 means odd is possible.
    ///
    /// A fully bounded tape with fixed run counts has an exact parity. An
    /// unknown end or an indefinite nonblank run can realize either parity,
    /// so returning `0b11` is the sound conservative answer.
    fn nonblank_parity_mask(&self) -> u8 {
        let mut parity = u8::from(self.scan != 0);

        for span in [&self.lspan, &self.rspan] {
            if span.end == TapeEnd::Unknown {
                return 0b11;
            }

            for block in span.span.iter() {
                if block.color == 0 {
                    continue;
                }
                if block.count.is_indef() {
                    return 0b11;
                }

                parity ^= u8::from(block.count.minimum() & 1 != 0);
            }
        }

        1_u8 << parity
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
        self.lspan.span.first().map(|b| b.color).or_else(|| {
            matches!(self.lspan.end, TapeEnd::Blanks).then_some(0)
        })
    }

    /// Return the immediate right neighbor color if it is determined by
    /// this tape description. If the right side is completely unknown
    /// (`?`) and there are no explicit blocks, returns None.
    fn right_neighbor_color(&self) -> Option<Color> {
        self.rspan.span.first().map(|b| b.color).or_else(|| {
            matches!(self.rspan.end, TapeEnd::Blanks).then_some(0)
        })
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

        block.count.can_be_one()
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
            let mut seen_zero = false;

            for b in span.span.iter() {
                if seen_zero && b.color != 0 {
                    return false; // nonblank beyond an unvisited blank
                }
                if b.color == 0 {
                    seen_zero = true;
                }
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
            let left_nonblank =
                self.lspan.span.iter().any(|block| block.color != 0);
            let right_nonblank =
                self.rspan.span.iter().any(|block| block.color != 0);

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
        self.lspan
            .span
            .iter()
            .all(|block| !forbid_left[block.color as usize])
            && self
                .rspan
                .span
                .iter()
                .all(|block| !forbid_right[block.color as usize])
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

            for block in span.span.iter() {
                let color = block.color as usize;
                req.colors |= 1_u64 << color;

                if block.count.minimum() > 1 {
                    req.pairs[color] |= 1_u64 << color;
                    req.pair_nears |= 1_u64 << color;
                }

                if let Some(near) = previous {
                    req.pairs[near] |= 1_u64 << color;
                    req.pair_nears |= 1_u64 << near;
                }

                previous = Some(color);
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

    /// Match the ordered two-run-plus-spill forward prefixes on both sides
    /// against one compatible exact local window.
    fn obeys_side_prefix_possible<const S: usize, const C: usize>(
        &self,
        state: State,
        possible: &SidePrefixPossible<S, C>,
    ) -> bool {
        #[derive(Clone, Copy)]
        struct ReqRun {
            color: Color,
            min: u16,
            max: Option<u16>,
        }

        impl ReqRun {
            const EMPTY: Self = Self {
                color: 0,
                min: 0,
                max: Some(0),
            };
        }

        #[derive(Clone, Copy)]
        struct Requirement {
            // The two full runs plus the one spill run can all be compared
            // directly with the backward residual description.
            runs: [ReqRun; 3],

            // Number of explicit residual runs, capped at three. This is enough
            // to answer whether the known two-run-plus-spill prefix extends
            // past them.
            run_count: u8,

            // `suffix_nonblank[i]` says some explicit nonblank residual run
            // occurs at position i or farther, for i in 0..=3. Position 3
            // summarizes every run beyond the spill descriptor.
            suffix_nonblank: [bool; 4],
            end_unknown: bool,
        }

        impl Requirement {
            const EMPTY: Self = Self {
                runs: [ReqRun::EMPTY; 3],
                run_count: 0,
                suffix_nonblank: [false; 4],
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

        #[derive(Clone, Copy)]
        enum FirstResidual {
            Keep,
            Drop,
            Replace(BlockCount),
        }

        fn req_run(color: Color, count: BlockCount) -> ReqRun {
            match count {
                BlockCount::Exact(count) => ReqRun {
                    color,
                    min: u16::from(count),
                    max: Some(u16::from(count)),
                },
                BlockCount::AtLeast(count) => ReqRun {
                    color,
                    min: u16::from(count),
                    max: None,
                },
            }
        }

        fn requirement(
            span: &Span,
            first: FirstResidual,
        ) -> Requirement {
            let end_unknown = span.end == TapeEnd::Unknown;
            let block_count = span.span.len();
            let trim_far_zero = !end_unknown
                && span
                    .span
                    .iter()
                    .next_back()
                    .is_some_and(|block| block.color == 0);

            let mut out = Requirement {
                end_unknown,
                ..Requirement::EMPTY
            };
            let mut count = 0_usize;

            for (source_index, block) in span.span.iter().enumerate() {
                if trim_far_zero && source_index + 1 == block_count {
                    continue;
                }

                let residual = if source_index == 0 {
                    match first {
                        FirstResidual::Keep => Some(block.count),
                        FirstResidual::Drop => None,
                        FirstResidual::Replace(count) => Some(count),
                    }
                } else {
                    Some(block.count)
                };

                let Some(residual) = residual else {
                    continue;
                };

                if count < 3 {
                    out.runs[count] = req_run(block.color, residual);
                }

                if block.color != 0 {
                    let highest = count.min(3);
                    for from in 0..=highest {
                        out.suffix_nonblank[from] = true;
                    }
                }

                count += 1;
            }

            out.run_count = count.min(3) as u8;

            // With an unknown tape end, the final explicit run may continue
            // into that end with the same color. Preserve the old unbounded
            // upper-bound semantics without building a temporary Vec.
            if end_unknown && (1..=3).contains(&count) {
                out.runs[count - 1].max = None;
            }

            out
        }

        fn requirements(span: &Span) -> RequirementSet {
            let Some(first) = span.span.first() else {
                let req = requirement(span, FirstResidual::Keep);
                return RequirementSet {
                    reqs: [req, Requirement::EMPTY],
                    len: 1,
                    unconstrained: req.unconstrained(),
                };
            };

            let (first_mode, second_mode) = match first.count {
                BlockCount::Exact(1) => (FirstResidual::Drop, None),
                BlockCount::Exact(count) => (
                    FirstResidual::Replace(BlockCount::Exact(
                        count - 1,
                    )),
                    None,
                ),
                BlockCount::AtLeast(1) => (
                    FirstResidual::Drop,
                    Some(FirstResidual::Replace(BlockCount::AtLeast(
                        1,
                    ))),
                ),
                BlockCount::AtLeast(count) => (
                    FirstResidual::Replace(BlockCount::AtLeast(
                        count - 1,
                    )),
                    None,
                ),
            };

            let first = requirement(span, first_mode);
            let mut out = RequirementSet {
                reqs: [first, Requirement::EMPTY],
                len: 1,
                unconstrained: first.unconstrained(),
            };

            if let Some(second_mode) = second_mode {
                let second = requirement(span, second_mode);
                out.reqs[1] = second;
                out.len = 2;
                out.unconstrained |= second.unconstrained();
            }

            out
        }

        fn count_bounds(count: u8) -> (u16, Option<u16>) {
            match count {
                1..=3 => (u16::from(count), Some(u16::from(count))),
                SIDE_PREFIX_MANY => (u16::from(SIDE_PREFIX_MANY), None),
                _ => unreachable!(),
            }
        }

        fn spill_count_bounds(count: u8) -> (u16, Option<u16>) {
            match count {
                1 => (1, Some(1)),
                SIDE_PREFIX_SPILL_MANY => {
                    (u16::from(SIDE_PREFIX_SPILL_MANY), None)
                },
                _ => unreachable!(),
            }
        }

        const fn intervals_overlap(
            a_min: u16,
            a_max: Option<u16>,
            b_min: u16,
            b_max: Option<u16>,
        ) -> bool {
            let a_before_b = matches!(a_max, Some(max) if max < b_min);
            let b_before_a = matches!(b_max, Some(max) if max < a_min);
            !a_before_b && !b_before_a
        }

        fn run_matches(
            color: Color,
            min: u16,
            max: Option<u16>,
            req: ReqRun,
        ) -> bool {
            color == req.color
                && intervals_overlap(min, max, req.min, req.max)
        }

        fn matches(prefix: SidePrefix, req: Requirement) -> bool {
            let prefix_len = usize::from(prefix.len);
            let req_len = req.run_count as usize;
            let common = prefix_len.min(req_len);

            for index in 0..common {
                let run = prefix.runs[index];
                let (min, max) = count_bounds(run.count);
                if !run_matches(run.color, min, max, req.runs[index]) {
                    return false;
                }
            }

            if prefix_len > req_len {
                // The backward description can supply additional run
                // structure only when its explicit prefix ends in `?`.
                return req.end_unknown;
            }

            let (known_len, tail_dirty) = match prefix.spill {
                SidePrefixSpill::Blank => (prefix_len, false),
                SidePrefixSpill::DirtyUnknown => (prefix_len, true),
                SidePrefixSpill::Run {
                    color,
                    count,
                    farther_dirty,
                } => {
                    // The spill is the next exact run after the two precise
                    // descriptors (or sooner after pulls have shortened the
                    // precise prefix). Compare its color and 1/2+ count too.
                    if req_len <= prefix_len {
                        return req.end_unknown;
                    }
                    let (min, max) = spill_count_bounds(count);
                    if !run_matches(
                        color,
                        min,
                        max,
                        req.runs[prefix_len],
                    ) {
                        return false;
                    }
                    (prefix_len + 1, farther_dirty)
                },
            };

            if known_len > req_len {
                return req.end_unknown;
            }

            if tail_dirty {
                req.end_unknown || req.suffix_nonblank[known_len]
            } else {
                !req.suffix_nonblank[known_len]
            }
        }

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
                (0..usize::from(reqs.len))
                    .any(|index| matches(prefix, reqs.reqs[index]))
            })
        }

        let st = state as usize;
        let sc = self.scan as usize;
        let known_left = self.left_neighbor_color().map(usize::from);
        let known_right = self.right_neighbor_color().map(usize::from);

        let candidate_unconstrained = |left: usize, right: usize| {
            possible.side_unconstrained(st, sc, left, right, LEFT_SIDE)
                && possible
                    .side_unconstrained(st, sc, left, right, RIGHT_SIDE)
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

        // Fixed-size requirement compilation: no Vec construction, cloning,
        // or per-configuration heap allocation remains on this hot path.
        let left_req = requirements(&self.lspan);
        let right_req = requirements(&self.rspan);

        let matches_window = |left: usize, right: usize| {
            matches_side(
                possible, st, sc, left, right, LEFT_SIDE, left_req,
            ) && matches_side(
                possible, st, sc, left, right, RIGHT_SIDE, right_req,
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
            if span.span.iter().any(|block| block.color != 0) {
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
fn test_get_indef_skips_noop_blank_extension() {
    let config = Config::new(1, Tape::init_l_spinout());
    let diff = Entries::new();
    let same = Entries::new();

    // `init_l_spinout` is `0+ [0] ?`. Pushing an indefinite 0 run onto the
    // left `0+` tail changes nothing, so there is no distinct indefinite
    // branch to add.
    assert!(get_indef(false, &config, &diff, &same).unwrap().is_none());
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

#[test]
fn test_count_limit_triggers_overflow_edge_cycle_cut() {
    // Precise BKW reaches u8 CountLimit on the A/C growth ladder. The retry
    // must recognize the recurring overflowing edge and cut only that edge;
    // in particular it must not widen the run and invent a B-state entrance.
    let prog = Prog::<3, 2>::from("1RB 1LC  1LC 1RB  ... 1LA");
    assert!(prog.bkw_cant_halt(300).is_refuted());
}

#[test]
#[expect(clippy::shadow_unrelated)]
fn test_overflow_edge_history_requires_stable_recurrence() {
    let instr: Instr = (2, true, 2);
    let mut history = OverflowCycleHistory::default();

    for (step, count) in (20..=25).zip(249_u8..=254) {
        let tape_text = format!("0+ 3.. [2] 2^{count} 0 ?");
        let config = Config::new(2, tape_text.as_str().into());
        let (key, observed) =
            growth_edge_observation(&config, instr).unwrap();
        assert_eq!(observed, count);
        history.observe(key, step, count);
    }

    let overflow = Config::new(2, "0+ 3.. [2] 2^255 0 ?".into());
    let (key, count) =
        growth_edge_observation(&overflow, instr).unwrap();
    assert!(history.certifies(&key, 26, count));

    // Same tape family but irregular step spacing is not a certificate.
    let mut irregular = OverflowCycleHistory::default();
    for (step, count) in [
        (10, 249_u8),
        (11, 250),
        (13, 251),
        (14, 252),
        (17, 253),
        (18, 254),
    ] {
        let tape_text = format!("0+ 3.. [2] 2^{count} 0 ?");
        let config = Config::new(2, tape_text.as_str().into());
        let (key, _) = growth_edge_observation(&config, instr).unwrap();
        irregular.observe(key, step, count);
    }
    assert!(!irregular.certifies(&key, 26, count));
}

#[test]
fn test_lower_bounded_indefinite_runs() {
    let mut tape: Tape = "0+ [0] 1^3.. ?".into();

    assert!(!tape.pull_needs_count_one_split(false));
    tape.backstep(false, 0).unwrap();
    tape.assert("0+ [0] 1^2.. ?");

    // Definite and indefinite same-color pushes both raise the lower bound.
    let mut pushed: Tape = "0+ 1^2.. [1] ?".into();
    pushed.backstep(false, 0).unwrap();
    pushed.assert("0+ 1^3.. [0] ?");
    pushed.scan = 1;
    pushed.push_indef(false).unwrap();
    pushed.assert("0+ 1^4.. [1] ?");

    let merged: Tape = "0+ [0] 1 1^2.. ?".into();
    merged.assert("0+ [0] 1^3.. ?");
    assert!(!merged.pull_needs_count_one_split(false));

    let mut split: Tape = "0+ [0] 1.. ?".into();
    assert!(split.pull_needs_count_one_split(false));
    split.rspan.set_head_to_one();
    split.assert("0+ [0] 1 ?");
}

#[test]
fn test_nonblank_parity_pruning() {
    let prog =
        Prog::<3, 3>::from("1RB 2RA 1LC  2LC 1RB 2RB  ... 2LA 1LA");
    let parity = prog.nonblank_parity_from_blank();

    // For this machine, A and C always have even support while B always has
    // odd support.
    assert_eq!(parity.possible, [0b01, 0b10, 0b01]);

    let even: Tape = "0+ 2 1 [0] 0+".into();
    assert_eq!(even.nonblank_parity_mask(), 0b01);
    assert!(nonblank_parity_possible(0, &even, &parity));
    assert!(!nonblank_parity_possible(1, &even, &parity));
    assert!(nonblank_parity_possible(2, &even, &parity));

    let odd: Tape = "0+ 2 [0] 0+".into();
    assert_eq!(odd.nonblank_parity_mask(), 0b10);
    assert!(!nonblank_parity_possible(0, &odd, &parity));
    assert!(nonblank_parity_possible(1, &odd, &parity));

    // An unresolved tail or an indefinite nonblank run remains conservative.
    let unknown: Tape = "0+ 2 1 [0] ?".into();
    assert_eq!(unknown.nonblank_parity_mask(), 0b11);
    assert!(nonblank_parity_possible(1, &unknown, &parity));

    let indefinite: Tape = "0+ 1.. [0] 0+".into();
    assert_eq!(indefinite.nonblank_parity_mask(), 0b11);
}

#[test]
fn test_window_nonblank_parity_pruning() {
    // The window BFS starts with exact even support at the all-zero initial
    // window and propagates parity independently of local-window reachability.
    let prog = Prog::<2, 2>::from("1RB 1RB  1LA ...");
    let state_parity = prog.nonblank_parity_from_blank();
    assert_eq!(state_parity.possible[1], 0b11);

    let (forbid_left, forbid_right) = prog.shift_side_forbidden();
    let windows =
        prog.win_possible_from_blank(&forbid_left, &forbid_right);

    // State B admits both support parities overall, but an exact local window
    // can retain a single parity.
    let mut found_exact = false;
    for scan in 0..2 {
        for left in 0..2 {
            for right in 0..2 {
                let mask =
                    windows.exact_parity_mask(1, scan, left, right);
                if matches!(mask, 0b01 | 0b10) {
                    found_exact = true;
                }
            }
        }
    }
    assert!(found_exact);

    // Query-time matching existentially joins only unknown neighbors.  With
    // both neighbors fixed, an incompatible exact support parity is rejected.
    let mut synthetic = WinPossible::<1, 2> {
        right: [[[0; 2]; 2]; 1],
        left: [[[0; 2]; 2]; 1],
        any: [[false; 2]; 1],
        parity: vec![0; 2 * 2 * 2],
        parity_right: [[[0; 2]; 2]; 1],
        parity_left: [[[0; 2]; 2]; 1],
        parity_any: [[0; 2]; 1],
        side_parity: vec![0; 2 * 2 * 2],
        side_parity_right: [[[0; 2]; 2]; 1],
        side_parity_left: [[[0; 2]; 2]; 1],
        side_parity_any: [[0; 2]; 1],
        side_mod3: vec![0; 2 * 2 * 2],
        side_mod3_right: [[[0; 2]; 2]; 1],
        side_mod3_left: [[[0; 2]; 2]; 1],
        side_mod3_any: [[0; 2]; 1],
        color_parity: vec![0; 2 * 2 * 2],
        color_parity_right: [[[0; 2]; 2]; 1],
        color_parity_left: [[[0; 2]; 2]; 1],
        color_parity_any: [[0; 2]; 1],
    };
    synthetic.right[0][0][1] = 1 << 0;
    synthetic.left[0][0][0] = 1 << 1;
    synthetic.any[0][0] = true;
    let index = WinPossible::<1, 2>::parity_index(0, 0, 1, 0);
    synthetic.parity[index] = 0b10; // exact window allows odd only
    synthetic.parity_right[0][0][1] = 0b10;
    synthetic.parity_left[0][0][0] = 0b10;
    synthetic.parity_any[0][0] = 0b10;

    let odd: Tape = "0+ 1 [0] 0+".into();
    assert_eq!(odd.nonblank_parity_mask(), 0b10);
    assert!(window_nonblank_parity_possible(0, &odd, &synthetic));

    let even: Tape = "0+ 1^2 [0] 0+".into();
    assert_eq!(even.nonblank_parity_mask(), 0b01);
    assert!(!window_nonblank_parity_possible(0, &even, &synthetic));
}

#[test]
fn test_window_side_nonblank_parity_pruning() {
    // Same exact local window and same total odd parity, but only the
    // `(left odd, right even)` distribution is forward-reachable.
    let mut synthetic = WinPossible::<1, 2> {
        right: [[[0; 2]; 2]; 1],
        left: [[[0; 2]; 2]; 1],
        any: [[false; 2]; 1],
        parity: vec![0; 2 * 2 * 2],
        parity_right: [[[0; 2]; 2]; 1],
        parity_left: [[[0; 2]; 2]; 1],
        parity_any: [[0; 2]; 1],
        side_parity: vec![0; 2 * 2 * 2],
        side_parity_right: [[[0; 2]; 2]; 1],
        side_parity_left: [[[0; 2]; 2]; 1],
        side_parity_any: [[0; 2]; 1],
        side_mod3: vec![0; 2 * 2 * 2],
        side_mod3_right: [[[0; 2]; 2]; 1],
        side_mod3_left: [[[0; 2]; 2]; 1],
        side_mod3_any: [[0; 2]; 1],
        color_parity: vec![0; 2 * 2 * 2],
        color_parity_right: [[[0; 2]; 2]; 1],
        color_parity_left: [[[0; 2]; 2]; 1],
        color_parity_any: [[0; 2]; 1],
    };

    synthetic.right[0][0][1] = 1 << 1;
    synthetic.left[0][0][1] = 1 << 1;
    synthetic.any[0][0] = true;
    let index = WinPossible::<1, 2>::parity_index(0, 0, 1, 1);
    synthetic.parity[index] = 0b10;
    synthetic.parity_right[0][0][1] = 0b10;
    synthetic.parity_left[0][0][1] = 0b10;
    synthetic.parity_any[0][0] = 0b10;

    // side-parity code 1 = left odd, right even.
    synthetic.side_parity[index] = 1 << 1;
    synthetic.side_parity_right[0][0][1] = 1 << 1;
    synthetic.side_parity_left[0][0][1] = 1 << 1;
    synthetic.side_parity_any[0][0] = 1 << 1;

    let matching: Tape = "0+ 1 [0] 1^2 0+".into();
    assert_eq!(matching.side_nonblank_parity_masks(), (0b10, 0b01));
    assert!(window_side_nonblank_parity_possible(
        0, &matching, &synthetic,
    ));

    // Move the odd support to the right. Total parity is still odd, so the
    // total-parity filter accepts it, while the joint side-parity filter does
    // not.
    let wrong_distribution: Tape = "0+ 1^2 [0] 1 0+".into();
    assert_eq!(wrong_distribution.nonblank_parity_mask(), 0b10);
    assert!(window_nonblank_parity_possible(
        0,
        &wrong_distribution,
        &synthetic,
    ));
    assert!(!window_side_nonblank_parity_possible(
        0,
        &wrong_distribution,
        &synthetic,
    ));
}

#[test]
fn test_window_side_nonblank_mod3_pruning() {
    // Same exact local window and same side parities, but different left-side
    // nonblank count modulo 3.  The parity checks alone cannot distinguish
    // one nonblank from three nonblanks.
    let mut synthetic = WinPossible::<1, 2> {
        right: [[[0; 2]; 2]; 1],
        left: [[[0; 2]; 2]; 1],
        any: [[false; 2]; 1],
        parity: vec![0; 2 * 2 * 2],
        parity_right: [[[0; 2]; 2]; 1],
        parity_left: [[[0; 2]; 2]; 1],
        parity_any: [[0; 2]; 1],
        side_parity: vec![0; 2 * 2 * 2],
        side_parity_right: [[[0; 2]; 2]; 1],
        side_parity_left: [[[0; 2]; 2]; 1],
        side_parity_any: [[0; 2]; 1],
        side_mod3: vec![0; 2 * 2 * 2],
        side_mod3_right: [[[0; 2]; 2]; 1],
        side_mod3_left: [[[0; 2]; 2]; 1],
        side_mod3_any: [[0; 2]; 1],
        color_parity: vec![0; 2 * 2 * 2],
        color_parity_right: [[[0; 2]; 2]; 1],
        color_parity_left: [[[0; 2]; 2]; 1],
        color_parity_any: [[0; 2]; 1],
    };

    synthetic.right[0][0][1] = 1 << 0;
    synthetic.left[0][0][0] = 1 << 1;
    synthetic.any[0][0] = true;
    let index = WinPossible::<1, 2>::parity_index(0, 0, 1, 0);

    // Odd total, left odd/right even: both parity filters accept either tape.
    synthetic.parity[index] = 0b10;
    synthetic.parity_right[0][0][1] = 0b10;
    synthetic.parity_left[0][0][0] = 0b10;
    synthetic.parity_any[0][0] = 0b10;
    synthetic.side_parity[index] = 1 << 1;
    synthetic.side_parity_right[0][0][1] = 1 << 1;
    synthetic.side_parity_left[0][0][0] = 1 << 1;
    synthetic.side_parity_any[0][0] = 1 << 1;

    // Mod-3 code 1 = left residue 1, right residue 0.
    synthetic.side_mod3[index] = 1 << 1;
    synthetic.side_mod3_right[0][0][1] = 1 << 1;
    synthetic.side_mod3_left[0][0][0] = 1 << 1;
    synthetic.side_mod3_any[0][0] = 1 << 1;

    let one: Tape = "0+ 1 [0] 0+".into();
    assert_eq!(one.side_nonblank_mod3_masks(), (0b010, 0b001));
    assert!(window_side_nonblank_mod3_possible(0, &one, &synthetic));

    let three: Tape = "0+ 1^3 [0] 0+".into();
    assert_eq!(three.side_nonblank_parity_masks(), (0b10, 0b01));
    assert!(window_side_nonblank_parity_possible(
        0, &three, &synthetic
    ));
    assert_eq!(three.side_nonblank_mod3_masks(), (0b001, 0b001));
    assert!(!window_side_nonblank_mod3_possible(0, &three, &synthetic));
}

#[test]
fn test_window_per_color_parity_pruning() {
    // Three colors: vector bit 0 is parity of color 1, bit 1 of color 2.
    // The exact window admits only vector `01` (odd #1, even #2).  Both tapes
    // below have odd total nonblank parity, so total parity alone cannot
    // distinguish them.
    let mut synthetic = WinPossible::<1, 3> {
        right: [[[0; 3]; 3]; 1],
        left: [[[0; 3]; 3]; 1],
        any: [[false; 3]; 1],
        parity: vec![0; 3 * 3 * 3],
        parity_right: [[[0; 3]; 3]; 1],
        parity_left: [[[0; 3]; 3]; 1],
        parity_any: [[0; 3]; 1],
        side_parity: vec![0; 3 * 3 * 3],
        side_parity_right: [[[0; 3]; 3]; 1],
        side_parity_left: [[[0; 3]; 3]; 1],
        side_parity_any: [[0; 3]; 1],
        side_mod3: vec![0; 3 * 3 * 3],
        side_mod3_right: [[[0; 3]; 3]; 1],
        side_mod3_left: [[[0; 3]; 3]; 1],
        side_mod3_any: [[0; 3]; 1],
        color_parity: vec![0; 3 * 3 * 3],
        color_parity_right: [[[0; 3]; 3]; 1],
        color_parity_left: [[[0; 3]; 3]; 1],
        color_parity_any: [[0; 3]; 1],
    };

    synthetic.right[0][0][1] = 1 << 0;
    synthetic.left[0][0][0] = 1 << 1;
    synthetic.any[0][0] = true;
    let index = WinPossible::<1, 3>::parity_index(0, 0, 1, 0);
    let vector_odd_1_even_2 = 0b01_u8;
    let vector_bit = 1_u64 << vector_odd_1_even_2;
    synthetic.color_parity[index] = vector_bit;
    synthetic.color_parity_right[0][0][1] = vector_bit;
    synthetic.color_parity_left[0][0][0] = vector_bit;
    synthetic.color_parity_any[0][0] = vector_bit;

    let color_1: Tape = "0+ 1 [0] 0+".into();
    assert_eq!(color_1.color_parity_mask::<3>(), 1 << 0b01);
    assert!(window_color_parity_possible(0, &color_1, &synthetic));

    let color_2: Tape = "0+ 2 [0] 0+".into();
    assert_eq!(color_2.nonblank_parity_mask(), 0b10);
    assert_eq!(color_2.color_parity_mask::<3>(), 1 << 0b10);
    assert!(!window_color_parity_possible(0, &color_2, &synthetic));

    // An indefinite run of color 1 makes only the color-1 parity unknown; it
    // does not erase information about color 2.
    let indefinite: Tape = "0+ 1.. [0] 0+".into();
    assert_eq!(
        indefinite.color_parity_mask::<3>(),
        (1 << 0b00) | (1 << 0b01)
    );
}

#[test]
fn test_forward_per_color_parity_propagation() {
    let prog = Prog::<2, 3>::from("1RB ... ...  ... ... ...");
    let (forbid_left, forbid_right) = prog.shift_side_forbidden();
    let windows =
        prog.win_possible_from_blank(&forbid_left, &forbid_right);

    // A0 writes one 1 and moves right onto blank, so B0 has vector 01.
    let index = WinPossible::<2, 3>::parity_index(1, 0, 1, 0);
    assert_ne!(windows.color_parity[index] & (1_u64 << 0b01), 0);
    assert_eq!(windows.color_parity[index] & (1_u64 << 0b10), 0);
}

#[test]
fn test_scanned_fresh_zero_closes_opposite_tail() {
    let prog =
        Prog::<3, 3>::from("1RB 2RA 1LC  2LC 1RB 2RB  ... 2LA 1LA");
    let parity = prog.nonblank_parity_from_blank();

    // This is one of the backward B0 branches from the example. Since the
    // program never writes blank, the scanned 0 is fresh; the unknown tail to
    // its right is therefore all blank. Its support parity is then exactly
    // even, contradicting state B.
    let mut branch: Tape = "0+ 2 1 [0] ?".into();
    assert!(branch.enforce_fresh_zero_side_invariants(true, true));
    branch.assert("0+ 2 1 [0] 0+");
    assert!(!nonblank_parity_possible(1, &branch, &parity));

    // A fresh scanned blank cannot have previously visited/nonblank cells on
    // both sides.
    let mut impossible: Tape = "? 1 [0] 2 ?".into();
    assert!(!impossible.enforce_fresh_zero_side_invariants(true, true));
}

#[test]
fn test_state_side_colors_and_pairs() {
    let prog = Prog::<2, 2>::from("1RB ...  ... ...");
    let (forbid_left, forbid_right) = prog.shift_side_forbidden();
    let windows =
        prog.win_possible_from_blank(&forbid_left, &forbid_right);
    let sides = prog.side_possible_from_blank(&windows);

    // After the only step the exact local window is `1 [0] 0` in state B.
    // Its whole-left summary contains the newly created (1,0) boundary pair,
    // while the right side remains blank.
    let b = sides.window(1, 0, 1, 0);
    assert!(b.reachable);
    assert_eq!(b.colors[LEFT_SIDE], 0b11);
    assert_eq!(b.colors[RIGHT_SIDE], 0b01);
    assert_ne!(b.pairs[LEFT_SIDE][1] & 0b01, 0);
    assert_eq!(b.pairs[LEFT_SIDE][1] & 0b10, 0);

    // No state-B window scanning 1 is reachable.
    assert!((0..2).all(|left| {
        (0..2).all(|right| !sides.window(1, 1, left, right).reachable)
    }));

    let valid: Tape = "0+ 1 [0] 0+".into();
    assert!(valid.obeys_state_side(1, &sides));

    let impossible_right: Tape = "0+ [0] 1 0+".into();
    assert!(!impossible_right.obeys_state_side(1, &sides));

    // A fixed run of length two requires (1,1), which never occurs here.
    let impossible_pair: Tape = "0+ 1^2 [0] 0+".into();
    assert!(!impossible_pair.obeys_state_side(1, &sides));

    // An indefinite run denotes one-or-more cells, so it can still choose
    // length one and must not require the self-pair.
    let indefinite: Tape = "0+ 1.. [0] 0+".into();
    assert!(indefinite.obeys_state_side(1, &sides));

    // A lower bound of two guarantees an internal (1,1) adjacency.
    let at_least_two: Tape = "0+ 1^2.. [0] 0+".into();
    assert!(!at_least_two.obeys_state_side(1, &sides));
}

#[test]
fn test_state_side_window_conditioning() {
    // Two reachable summaries have the same `(state, scan, left)` but
    // different right neighbors.  Only the right=1 window admits left pair
    // (1,1).  A state+scan summary would union that pair into the right=0
    // case and incorrectly accept the length-two left run below.
    let mut sides = SidePossible::<1, 2>::new();

    {
        let summary = sides.window_mut(0, 0, 1, 0);
        summary.reachable = true;
        summary.colors[LEFT_SIDE] = 0b11;
        summary.colors[RIGHT_SIDE] = 0b01;
        summary.pairs[LEFT_SIDE][0] = 0b01;
        summary.pairs[LEFT_SIDE][1] = 0b01;
        summary.pairs[RIGHT_SIDE][0] = 0b01;
    }

    {
        let summary = sides.window_mut(0, 0, 1, 1);
        summary.reachable = true;
        summary.colors[LEFT_SIDE] = 0b11;
        summary.colors[RIGHT_SIDE] = 0b11;
        summary.pairs[LEFT_SIDE][0] = 0b01;
        summary.pairs[LEFT_SIDE][1] = 0b11;
        summary.pairs[RIGHT_SIDE][0] = 0b01;
        summary.pairs[RIGHT_SIDE][1] = 0b01;
    }

    let valid: Tape = "0+ 1 [0] 0+".into();
    assert!(valid.obeys_state_side(0, &sides));

    let cross_window_union_only: Tape = "0+ 1^2 [0] 0+".into();
    assert!(!cross_window_union_only.obeys_state_side(0, &sides));
}

#[test]
fn test_side_prefix_dirty_unknown_subsumption() {
    let dirty = SidePrefix::dirty_unknown();
    let blank = SidePrefix::blank();
    let one = SidePrefix {
        runs: [
            SidePrefixRun { color: 1, count: 1 },
            SidePrefixRun::EMPTY,
        ],
        len: 1,
        spill: SidePrefixSpill::Blank,
    };
    let dirty_zero = SidePrefix {
        runs: [
            SidePrefixRun { color: 0, count: 1 },
            SidePrefixRun::EMPTY,
        ],
        len: 1,
        spill: SidePrefixSpill::DirtyUnknown,
    };

    assert!(dirty.subsumes(dirty));
    assert!(dirty.subsumes(one));
    assert!(dirty.subsumes(dirty_zero));
    assert!(!dirty.subsumes(blank));
    assert!(!blank.subsumes(one));
}

#[test]
fn test_side_prefix_spill_survives_pulls() {
    // Near-to-far retained runs are 2,3. Prepending a new run 1 must keep the
    // dropped 3 as the spill rather than immediately degrading it to dirty.
    let base = SidePrefix {
        runs: [
            SidePrefixRun { color: 2, count: 1 },
            SidePrefixRun { color: 3, count: 1 },
        ],
        len: 2,
        spill: SidePrefixSpill::Blank,
    };

    let mut prepended = Vec::new();
    base.for_each_prepend(1, |prefix| prepended.push(prefix));
    assert_eq!(prepended.len(), 1);
    let prefix = prepended[0];
    assert_eq!(prefix.runs[0], SidePrefixRun { color: 1, count: 1 });
    assert_eq!(prefix.runs[1], SidePrefixRun { color: 2, count: 1 });
    assert_eq!(
        prefix.spill,
        SidePrefixSpill::Run {
            color: 3,
            count: 1,
            farther_dirty: false,
        }
    );

    let mut first = Vec::new();
    prefix.for_each_pull::<4>(|color, next| first.push((color, next)));
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].0, 1);

    let mut second = Vec::new();
    first[0]
        .1
        .for_each_pull::<4>(|color, next| second.push((color, next)));
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].0, 2);
    assert_eq!(second[0].1.len, 0);
    assert_eq!(
        second[0].1.spill,
        SidePrefixSpill::Run {
            color: 3,
            count: 1,
            farther_dirty: false,
        }
    );

    let mut third = Vec::new();
    second[0]
        .1
        .for_each_pull::<4>(|color, next| third.push((color, next)));
    assert_eq!(third.len(), 1);
    assert_eq!(third[0].0, 3);
    assert_eq!(third[0].1, SidePrefix::blank());
}

#[test]
fn test_side_prefix_spill_matches_third_run() {
    // Synthetic exact window with a left side whose tail beyond immediate 4
    // is exactly 3,2,1,blank. The old two-run prefix could see only 3,2 plus
    // "dirty"; the spill retains the distinguishing third color 1.
    let mut possible = SidePrefixPossible::<1, 5>::new();
    let left_index =
        SidePrefixPossible::<1, 5>::index(0, 0, 4, 0, LEFT_SIDE);
    possible.windows[left_index].push(SidePrefix {
        runs: [
            SidePrefixRun { color: 3, count: 1 },
            SidePrefixRun { color: 2, count: 1 },
        ],
        len: 2,
        spill: SidePrefixSpill::Run {
            color: 1,
            count: 1,
            farther_dirty: false,
        },
    });

    let right_index =
        SidePrefixPossible::<1, 5>::index(0, 0, 4, 0, RIGHT_SIDE);
    possible.windows[right_index].push(SidePrefix::blank());
    possible.flags[right_index] |= SIDE_PREFIX_HAS_BLANK;

    let actual: Tape = "0+ 1 2 3 4 [0] 0+".into();
    assert!(actual.obeys_side_prefix_possible(0, &possible));

    let wrong_third: Tape = "0+ 4 2 3 4 [0] 0+".into();
    assert!(!wrong_third.obeys_side_prefix_possible(0, &possible));
}

#[test]
fn test_side_prefix_prunes_blank_example_branch() {
    let prog = Prog::<2, 4>::from("1RB 3RB 0RB 0LA  2LB 3RA 3LA 1LA");
    let (forbid_left, forbid_right) = prog.shift_side_forbidden();
    let windows =
        prog.win_possible_from_blank(&forbid_left, &forbid_right);
    let sides = prog.side_possible_from_blank(&windows);
    let prefixes =
        prog.side_prefix_possible_from_blank(&windows, &sides);

    // This is the late B0 branch from the blank trace. The immediate window
    // `1 [0] 2` is locally reachable, and the whole-side summaries admit all
    // of its colors/pairs separately, but no compatible forward prefix has a
    // single 3 beyond the right-neighbor 2 followed immediately by blank.
    let branch: Tape = "0+ 1 [0] 2 3 0+".into();
    assert!(!branch.obeys_side_prefix_possible(1, &prefixes));
}

#[test]
fn test_ordered_side_prefix_pruning() {
    // Walk right while writing 1,2,3. At D0 the exact local window is
    // `3 [0] 0`; strictly beyond the immediate left neighbor the ordered runs
    // are exactly `2, 1`, followed by blank tape.
    let prog = Prog::<4, 4>::from(
        "1RB ... ... ...  2RC ... ... ...  3RD ... ... ...  ... ... ... ...",
    );
    let (forbid_left, forbid_right) = prog.shift_side_forbidden();
    let windows =
        prog.win_possible_from_blank(&forbid_left, &forbid_right);
    let sides = prog.side_possible_from_blank(&windows);
    let prefixes =
        prog.side_prefix_possible_from_blank(&windows, &sides);

    let actual: Tape = "0+ 1 2 3 [0] 0+".into();
    assert!(actual.obeys_side_prefix_possible(3, &prefixes));

    // Same colors and same immediate window, but the two farther runs are in
    // the wrong order. Whole-side color/pair unions can join this away; the
    // ordered prefix cannot.
    let wrong_order: Tape = "0+ 2 1 3 [0] 0+".into();
    assert!(!wrong_order.obeys_side_prefix_possible(3, &prefixes));

    // The forward prefix also knows that the second retained run reaches the
    // blank end; an additional nonblank farther out is impossible.
    let hidden_nonblank: Tape = "0+ 2 1 2 3 [0] 0+".into();
    assert!(!hidden_nonblank.obeys_side_prefix_possible(3, &prefixes));
}

#[test]
#[expect(clippy::shadow_unrelated)]
fn test_halfblank_direction() {
    // A0 writes 1 and moves right.  At B0 the right side is still all blank,
    // but the left side contains the written 1.
    let prog = Prog::<2, 2>::from("1RB ...  ... ...");
    let (forbid_left, forbid_right) = prog.shift_side_forbidden();
    let windows =
        prog.win_possible_from_blank(&forbid_left, &forbid_right);
    let left_clean = side_excursions(&prog, &windows, false, true);
    let right_clean = side_excursions(&prog, &windows, true, true);
    let left_any = side_excursions(&prog, &windows, false, false);
    let right_any = side_excursions(&prog, &windows, true, false);
    let left = halfblank_slots(
        &prog,
        &windows,
        false,
        &left_clean,
        &right_any,
    );
    let right =
        halfblank_slots(&prog, &windows, true, &right_clean, &left_any);

    assert_eq!(left[1][0], 0);
    assert_eq!(right[1][0], 1 << 1);

    // Leaving 0 behind preserves both one-sided-blank abstractions.
    let prog = Prog::<2, 2>::from("0RB ...  ... ...");
    let (forbid_left, forbid_right) = prog.shift_side_forbidden();
    let windows =
        prog.win_possible_from_blank(&forbid_left, &forbid_right);
    let left_clean = side_excursions(&prog, &windows, false, true);
    let right_clean = side_excursions(&prog, &windows, true, true);
    let left_any = side_excursions(&prog, &windows, false, false);
    let right_any = side_excursions(&prog, &windows, true, false);
    let left = halfblank_slots(
        &prog,
        &windows,
        false,
        &left_clean,
        &right_any,
    );
    let right =
        halfblank_slots(&prog, &windows, true, &right_clean, &left_any);

    assert_ne!(left[1][0] & 1, 0);
    assert_ne!(right[1][0] & 1, 0);
}

#[test]
#[expect(clippy::shadow_unrelated)]
fn test_same_run_joint_blank_dirty_flags() {
    // Writing a nonblank while moving onto fresh blank reaches B0 with a dirty
    // left side and blank right side, but not with both sides blank.
    let prog = Prog::<2, 2>::from("1RB ...  ... ...");
    let (forbid_left, forbid_right) = prog.shift_side_forbidden();
    let windows =
        prog.win_possible_from_blank(&forbid_left, &forbid_right);
    let joint = joint_blank_status_from_blank(&prog, &windows);
    let both_bit = 1_u8 << BOTH_BLANK_FLAGS;
    let dirty_left_blank_right = 1_u8 << RIGHT_BLANK_FLAG;
    assert_ne!(joint.any[0][0] & both_bit, 0);
    assert_eq!(joint.any[1][0] & both_bit, 0);
    assert_ne!(joint.any[1][0] & dirty_left_blank_right, 0);

    // Leaving zero behind preserves an exact blank tape as the head moves.
    let prog = Prog::<2, 2>::from("0RB ...  ... ...");
    let (forbid_left, forbid_right) = prog.shift_side_forbidden();
    let windows =
        prog.win_possible_from_blank(&forbid_left, &forbid_right);
    let joint = joint_blank_status_from_blank(&prog, &windows);
    assert_ne!(joint.any[1][0] & both_bit, 0);
}

#[test]
fn test_per_color_tail_count_filter() {
    let prog = Prog::<2, 3>::from("1RB ... ...  ... ... ...");
    let (forbid_left, forbid_right) = prog.shift_side_forbidden();
    let windows =
        prog.win_possible_from_blank(&forbid_left, &forbid_right);
    let count = color_tail_count_from_blank(&prog, &windows);

    // After the only move, B0 has immediate left neighbor 1 but nothing
    // farther left.  Tail presence therefore records color 1 as absent.
    assert_eq!(count.mask(1, 0, Some(1), Some(0), 1), 1_u16 << 0);

    let actual: Tape = "? 1 [0] 0+".into();
    assert!(actual.obeys_color_tail_count(1, &count));

    // Same immediate window, but requiring another 1 farther left is rejected.
    let hidden_one: Tape = "? 1^2 [0] 0+".into();
    assert!(!hidden_one.obeys_color_tail_count(1, &count));

    // A different hidden color is also rejected.
    let hidden_two: Tape = "? 2 1 [0] 0+".into();
    assert!(!hidden_two.obeys_color_tail_count(1, &count));
}

#[test]
fn test_per_color_tail_count_distinguishes_one_from_two_plus() {
    let mut possible = ColorTailCountPossible::<1, 2>::new();

    // Exact local window `0 [0] 0`; only one color-1 cell exists in the left
    // tail and none in the right tail. Status code 1 = (left=1, right=0).
    possible.add(0, 0, 0, 0, 1, 1);

    let one: Tape = "0+ 1 0 [0] 0+".into();
    assert!(one.obeys_color_tail_count(0, &possible));

    let two: Tape = "0+ 1^2 0 [0] 0+".into();
    assert!(!two.obeys_color_tail_count(0, &possible));

    // A lower-bounded run of two also proves the `2+` category.
    let at_least_two: Tape = "0+ 1^2.. 0 [0] 0+".into();
    assert!(!at_least_two.obeys_color_tail_count(0, &possible));
}

#[test]
fn test_pair_tail_presence_same_run_filter() {
    let mut possible = PairTailPresencePossible::<1, 3>::new();

    // Same exact local window. One abstract run supplies only color 1 in the
    // left tail; another supplies only color 2 in the right tail. Independent
    // per-color projections would accept both requirements, but no single run
    // supplies them together.
    possible.add(0, 0, 0, 0, 1, 2, 0b0001);
    possible.add(0, 0, 0, 0, 1, 2, 0b1000);

    let requires_both: Tape = "0+ 1 0 [0] 0 2 0+".into();
    assert!(!requires_both.obeys_pair_tail_presence(0, &possible));

    // Add the genuinely correlated status: color 1 left-tail present and
    // color 2 right-tail present.
    possible.add(0, 0, 0, 0, 1, 2, 0b1001);
    assert!(requires_both.obeys_pair_tail_presence(0, &possible));
}

#[test]
fn test_forward_pair_tail_presence_propagation() {
    // A0 writes 1 and moves right; B0 writes 2 and moves right. At C0 the
    // immediate left neighbor is 2, while the older 1 is strictly in the left
    // tail. For pair (1,2), only the color-1-left presence bit is therefore set.
    let prog =
        Prog::<3, 3>::from("1RB ... ...  2RC ... ...  ... ... ...");
    let (forbid_left, forbid_right) = prog.shift_side_forbidden();
    let windows =
        prog.win_possible_from_blank(&forbid_left, &forbid_right);
    let possible = pair_tail_presence_from_blank(&prog, &windows);

    let mask = possible.mask(2, 0, Some(2), Some(0), 1, 2);
    assert_ne!(mask & (1_u16 << 0b0001), 0);
}

#[test]
fn test_dynamic_blank_side_filter() {
    let prog = Prog::<2, 2>::from("1RB ...  ... ...");
    let (forbid_left, forbid_right) = prog.shift_side_forbidden();
    let windows =
        prog.win_possible_from_blank(&forbid_left, &forbid_right);
    let possible = blank_side_possible_from_blank(&prog, &windows);

    // The actual B0 shape has a blank right side and a dirty left side.
    let reachable: Tape = "? 1 [0] 0+".into();
    assert!(reachable.obeys_blank_side_possible(1, &possible));

    // Requiring both sides blank adds the same-run correlation and rejects B0.
    let impossible: Tape = "0+ [0] 0+".into();
    assert!(!impossible.obeys_blank_side_possible(1, &possible));
}

#[test]
fn test_halfblank_inward_neighbor_is_enforced_dynamically() {
    // The joint blank/dirty abstraction allows either local right neighbor,
    // but the stronger clean halfblank abstraction allows a blank left side
    // only when the exact inward/right neighbor is 0.
    let mut joint = JointBlankPossible::<1, 2>::new();
    let left_blank_bit = 1_u8 << LEFT_BLANK_FLAG;

    for right in 0..2 {
        let index = JointBlankPossible::<1, 2>::index(0, 0, 0, right);
        joint.windows[index] |= left_blank_bit;
        joint.any[0][0] |= left_blank_bit;
    }

    let possible = BlankSidePossible {
        left_half: [[1; 2]; 1], // only inward neighbor 0
        right_half: [[0b11; 2]; 1],
        joint,
    };

    let valid: Tape = "0+ [0] 0 ?".into();
    assert!(valid.obeys_blank_side_possible(0, &possible));

    let wrong_near: Tape = "0+ [0] 1 ?".into();
    assert!(!wrong_near.obeys_blank_side_possible(0, &possible));
}

#[test]
fn test_blank_dirty_status_is_window_conditioned() {
    // Synthetic same-state/scan join: left-blank is possible only with right
    // neighbor 0, while right neighbor 1 is possible only with a dirty left.
    // A state/scan-only status table would incorrectly accept `0+ [0] 1 ?`.
    let mut joint = JointBlankPossible::<1, 2>::new();
    let left_blank_flags = LEFT_BLANK_FLAG;
    let left_dirty_flags = 0_u8;
    let blank_bit = 1_u8 << left_blank_flags;
    let dirty_bit = 1_u8 << left_dirty_flags;

    let w0 = JointBlankPossible::<1, 2>::index(0, 0, 0, 0);
    joint.windows[w0] |= blank_bit;
    joint.any[0][0] |= blank_bit;

    let w1 = JointBlankPossible::<1, 2>::index(0, 0, 0, 1);
    joint.windows[w1] |= dirty_bit;
    joint.any[0][0] |= dirty_bit;

    let possible = BlankSidePossible {
        left_half: [[0b11; 2]; 1],
        right_half: [[0b11; 2]; 1],
        joint,
    };

    let impossible: Tape = "0+ [0] 1 ?".into();
    assert!(!impossible.obeys_blank_side_possible(0, &possible));

    let valid_blank: Tape = "0+ [0] 0 ?".into();
    assert!(valid_blank.obeys_blank_side_possible(0, &possible));

    let valid_dirty: Tape = "? [0] 1 ?".into();
    assert!(valid_dirty.obeys_blank_side_possible(0, &possible));
}

#[test]
fn test_parent_color_aware_excursions() {
    // A0 pushes right, writing 0 on the parent. B1 pops left into C, then C0
    // pops left across A's outer boundary into D.  The synthetic forward-window
    // abstraction permits child color 1 at A0 only when A0's own back/left
    // neighbor is 1, not when it is 0.  The old union-over-back-colors relation
    // would therefore invent an outer return for back color 0 as well.
    let prog = Prog::<4, 2>::from("0RB ...  ... 0LC  0LD ...  ... ...");
    let mut windows = WinPossible {
        right: [[[0; 2]; 2]; 4],
        left: [[[0; 2]; 2]; 4],
        any: [[false; 2]; 4],
        parity: vec![0; 4 * 2 * 2 * 2],
        parity_right: [[[0; 2]; 2]; 4],
        parity_left: [[[0; 2]; 2]; 4],
        parity_any: [[0; 2]; 4],
        side_parity: vec![0; 4 * 2 * 2 * 2],
        side_parity_right: [[[0; 2]; 2]; 4],
        side_parity_left: [[[0; 2]; 2]; 4],
        side_parity_any: [[0; 2]; 4],
        side_mod3: vec![0; 4 * 2 * 2 * 2],
        side_mod3_right: [[[0; 2]; 2]; 4],
        side_mod3_left: [[[0; 2]; 2]; 4],
        side_mod3_any: [[0; 2]; 4],
        color_parity: vec![0; 4 * 2 * 2 * 2],
        color_parity_right: [[[0; 2]; 2]; 4],
        color_parity_left: [[[0; 2]; 2]; 4],
        color_parity_any: [[0; 2]; 4],
    };

    // A0: with left/back 0, right child must be 0; with left/back 1, child 1.
    windows.right[0][0][0] = 1 << 0;
    windows.right[0][0][1] = 1 << 1;

    // B1 with parent/back 0 can pop back to A's cell in state C.
    windows.right[1][1][0] = 1 << 0;

    // C0 can then pop across the outer boundary for either outer back color.
    windows.right[2][0][0] = 1 << 0;
    windows.right[2][0][1] = 1 << 0;

    let ex = side_excursions(&prog, &windows, true, false);

    assert_eq!(ex.ret_states(0, 0, 0) & (1 << 3), 0);
    assert_ne!(ex.ret_states(1, 0, 0) & (1 << 3), 0);
}

#[test]
fn test_fresh_frontier_direction() {
    // A0 moves right onto a fresh blank.  B0 is therefore a right frontier,
    // but not a left frontier: the A cell has already been visited.
    let prog = Prog::<2, 2>::from("1RB ...  ... ...");
    let (forbid_left, forbid_right) = prog.shift_side_forbidden();
    let windows =
        prog.win_possible_from_blank(&forbid_left, &forbid_right);
    let left_any = side_excursions(&prog, &windows, false, false);
    let right_any = side_excursions(&prog, &windows, true, false);
    let left = frontier_slots(&prog, &windows, false, &right_any);
    let right = frontier_slots(&prog, &windows, true, &left_any);

    assert_eq!(left[1][0], 0);
    assert_eq!(right[1][0], 1 << 1);
}

#[test]
fn test_halfblank_neighbor_blank_target_regression() {
    // This machine really blanks. Its final erase is A2, whose actual last
    // departure is A3 -> 2RA with inward neighbor 1. The recursive `clean`
    // excursion abstraction finds a conservative witness only after joining
    // child colors at the source window, so the neighbor-aware halfblank mask
    // must not be used to narrow that static last-departure child set.
    let prog = Prog::<2, 4>::from("1RB 3RB 0LA 2RA  2LB 0RB 3LA 1LA");

    assert!(prog.blank_slots_side_clean().contains(&(0, 2)));
}

#[test]
fn test_spinout_fresh_frontier_filter() {
    // These trigger states can occur scanning erased blank territory, but not
    // at the fresh frontier required for the self-looping zero transition to
    // spin out forever.
    let left = Prog::<3, 2>::from("1RB 0LB  1LA 0RC  1LC 1RB");
    assert!(!left.spinout_shifts_side_clean().contains(&(2, false)));

    let right = Prog::<3, 2>::from("1RB 0LC  1LA 0RA  1RC 1LA");
    assert!(!right.spinout_shifts_side_clean().contains(&(2, true)));
}

#[test]
#[expect(clippy::shadow_unrelated, clippy::iter_on_single_items)]
fn test_halt_side_excursion_filter() {
    // Fresh zero at a right frontier: after A0 moves right, B0 is a valid
    // halting shape because the newly scanned cell is blank and the right
    // side is still globally blank.
    let prog = Prog::<2, 2>::from("1RB ...  ... ...");
    let slots: Set<Slot> = [(1, 0)].into_iter().collect();
    assert!(prog.halt_slots_side_excursion(slots).contains(&(1, 0)));

    // Nonzero halt after a genuine one-sided return.  A0 writes 1 and moves
    // right; B0 returns left into C, so C1 is a realizable halt slot.
    let prog = Prog::<3, 2>::from("1RB ...  0LC ...  ... ...");
    let slots: Set<Slot> = [(2, 1)].into_iter().collect();
    assert!(prog.halt_slots_side_excursion(slots).contains(&(2, 1)));

    // If the child side has no way to return across the parent boundary, the
    // same nonzero halt slot has no last-departure witness.
    let prog = Prog::<3, 2>::from("1RB ...  0RB ...  ... ...");
    #[expect(clippy::shadow_unrelated)]
    let slots: Set<Slot> = [(2, 1)].into_iter().collect();
    assert!(prog.halt_slots_side_excursion(slots).is_empty());
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
    // Flattened [back][state][color]. Bit `tr` means a balanced return into
    // state `tr` is possible.
    ret: Vec<u64>,

    // For ordinary excursions, flattened
    // [back][state][color][return_state] -> final pop-color mask.
    // Clean excursions do not need this extra summary.
    pop: Option<Vec<u64>>,
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

#[expect(clippy::shadow_unrelated)]
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
        child_st: usize,
        child_colors: u64,
    }

    fn add_returns(
        ret: &mut [u64],
        q: &mut VecDeque<(usize, u64)>,
        node: usize,
        bits: u64,
    ) {
        let added = bits & !ret[node];
        if added != 0 {
            ret[node] |= added;
            q.push_back((node, added));
        }
    }

    let pop = !push;
    let ret_len = C * S * C;
    let mut ret = vec![0_u64; ret_len];
    let mut trans = [[None; C]; S];

    for ((st, co), &(print, shift, tr)) in prog.iter() {
        trans[st as usize][co as usize] =
            Some((print as usize, shift, tr as usize));
    }

    // For fixed `push`, both window tables have exactly the indexing we need:
    // [state][scan][known back color] -> possible child colors.
    let child_masks = if push { &windows.right } else { &windows.left };

    let mut pushes = Vec::new();
    let mut child_users = vec![Vec::<usize>::new(); ret_len];
    let mut seeds = Vec::new();
    let mut pop_seeds = Vec::new();

    for back in 0..C {
        for st in 0..S {
            for co in 0..C {
                let child_colors = child_masks[st][co][back];
                if child_colors == 0 {
                    continue;
                }

                let Some((print, shift, tr)) = trans[st][co] else {
                    continue;
                };

                let source =
                    SideExcursions::<S, C>::ret_index(back, st, co);

                if shift == pop {
                    if !clean || print == 0 {
                        seeds.push((source, 1_u64 << tr));
                        if !clean {
                            pop_seeds.push((source, tr, print));
                        }
                    }
                    continue;
                }

                let eq = pushes.len();
                pushes.push(PushEq {
                    source,
                    back,
                    print,
                    child_st: tr,
                    child_colors,
                });

                // This push equation depends on the return relation of every
                // child color allowed by the exact forward window.
                let mut colors = child_colors;
                while colors != 0 {
                    let child_co = colors.trailing_zeros() as usize;
                    colors &= colors - 1;
                    let child = SideExcursions::<S, C>::ret_index(
                        print, tr, child_co,
                    );
                    child_users[child].push(eq);
                }
            }
        }
    }

    // Once child return state `r` becomes possible for a push equation, that
    // equation depends on the continuation `(back, r, print)`.  These reverse
    // dependencies are discovered lazily, so each return bit is propagated
    // only to equations that can actually use it.
    let mut continuation_users = vec![Vec::<usize>::new(); ret_len];
    let mut child_returns = vec![0_u64; pushes.len()];
    let mut q = VecDeque::new();

    for (node, bits) in seeds {
        add_returns(&mut ret, &mut q, node, bits);
    }

    while let Some((node, added_states)) = q.pop_front() {
        // `node` is used as a nested child by these equations.  Newly returned
        // states expose newly relevant same-level continuation nodes.
        for &eq_i in &child_users[node] {
            let fresh = added_states & !child_returns[eq_i];
            if fresh == 0 {
                continue;
            }
            child_returns[eq_i] |= fresh;

            let eq = &pushes[eq_i];
            let mut states = fresh;
            while states != 0 {
                let return_st = states.trailing_zeros() as usize;
                states &= states - 1;

                let continuation = SideExcursions::<S, C>::ret_index(
                    eq.back, return_st, eq.print,
                );
                continuation_users[continuation].push(eq_i);

                // The continuation may already have returns from earlier
                // events; consume its full current value when registering.
                let current = ret[continuation];
                add_returns(&mut ret, &mut q, eq.source, current);
            }
        }

        // `node` is a same-level continuation for these equations.  Every new
        // outer return state immediately becomes a return of their sources.
        for &eq_i in &continuation_users[node] {
            let source = pushes[eq_i].source;
            add_returns(&mut ret, &mut q, source, added_states);
        }
    }

    let pop = if clean {
        None
    } else {
        // With the return-state relation saturated, build the same-level
        // continuation graph induced by push/child-return pairs.  Final pop
        // colors then propagate backwards through that graph.  This keeps the
        // ordinary excursion relation small while retaining exactly the one
        // extra fact needed by fresh-frontier analysis.
        let mut continuation_users = vec![Vec::<usize>::new(); ret_len];

        for eq in &pushes {
            let mut return_states = 0;
            let mut colors = eq.child_colors;
            while colors != 0 {
                let child_co = colors.trailing_zeros() as usize;
                colors &= colors - 1;
                let child = SideExcursions::<S, C>::ret_index(
                    eq.print,
                    eq.child_st,
                    child_co,
                );
                return_states |= ret[child];
            }

            while return_states != 0 {
                let return_st = return_states.trailing_zeros() as usize;
                return_states &= return_states - 1;
                let continuation = SideExcursions::<S, C>::ret_index(
                    eq.back, return_st, eq.print,
                );
                continuation_users[continuation].push(eq.source);
            }
        }

        let mut pop = vec![0_u64; ret_len * S];
        let mut q = VecDeque::new();

        for (source, tr, color) in pop_seeds {
            let index = source * S + tr;
            let bit = 1_u64 << color;
            if pop[index] & bit == 0 {
                pop[index] |= bit;
                q.push_back((source, tr, bit));
            }
        }

        while let Some((node, tr, colors)) = q.pop_front() {
            for &source in &continuation_users[node] {
                let index = source * S + tr;
                let added = colors & !pop[index];
                if added != 0 {
                    pop[index] |= added;
                    q.push_back((source, tr, added));
                }
            }
        }

        Some(pop)
    };

    SideExcursions { ret, pop }
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

    #[expect(clippy::shadow_unrelated)]
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

    #[expect(clippy::shadow_unrelated)]
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

    #[expect(clippy::shadow_unrelated)]
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

    #[expect(clippy::shadow_unrelated)]
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

    #[expect(clippy::shadow_unrelated)]
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

fn blank_side_possible_from_blank<const S: usize, const C: usize>(
    prog: &Prog<S, C>,
    windows: &WinPossible<S, C>,
) -> BlankSidePossible<S, C> {
    let left_clean = side_excursions(prog, windows, false, true);
    let right_clean = side_excursions(prog, windows, true, true);
    let left_any = side_excursions(prog, windows, false, false);
    let right_any = side_excursions(prog, windows, true, false);

    let left_half =
        halfblank_slots(prog, windows, false, &left_clean, &right_any);
    let right_half =
        halfblank_slots(prog, windows, true, &right_clean, &left_any);
    let joint = joint_blank_status_from_blank(prog, windows);

    BlankSidePossible {
        left_half,
        right_half,
        joint,
    }
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
        let windows =
            self.win_possible_from_blank(&forbid_left, &forbid_right);
        let left_any = side_excursions(self, &windows, false, false);
        let right_any = side_excursions(self, &windows, true, false);

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
