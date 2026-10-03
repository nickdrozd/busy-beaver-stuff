//! FAR/MITM non-halting prover
//!
//! Self-contained FAR decider with MITMWFAR folded into `far_cant_halt`, implemented as methods on `Prog`.
//!
//! Works for **any number of tape colors** (`COLORS >= 2`) and typical Busy Beaver
//! sizes (e.g. `states <= 8`, `colors <= 8`).
//!
//! # Public API
//! ```ignore
//! // The argument is the largest FAR block length to try.
//! // Higher values include every smaller block length.
//! let proved = prog.far_cant_halt(16);
//! ```
//!
//! Behavior:
//! - Returns `true` only when FAR or MITM/WFAR proves that the machine cannot halt.
//! - Returns `false` otherwise (it may halt, or FAR ran out of budgets).
//!
//! # How the argument works
//! `block` controls only the cumulative FAR block-length sweep:
//! block lengths `1..=block` are tried, subject to the alphabet-size
//! and hard safety caps.  Per-run budgets are fixed functions of `block_len`.

use core::{
    cmp::Ordering,
    hash::{Hash, Hasher},
};

use ahash::{AHashMap as Map, AHashSet as Set};

use crate::{Color, Goal, Prog, Slot, State, macros::GetInstr};

// -----------------------------------------------------------------------------
// Top-level tuning constants
// -----------------------------------------------------------------------------

/// Base exploration budget per block length unit.
///
/// With `block_len = 16`, this gives:
/// `max_work ≈ 12_500 * 16 = 200_000` (matching the previous single-run default).
const FAR_WORK_PER_LEN: usize = 12_500;

/// Base per-block raw-step budget per block length unit.
///
/// With `block_len = 16`, this gives:
/// `block_step_limit ≈ 200 * 16 = 3_200`.
const FAR_STEP_PER_LEN: usize = 200;

/// Hard cap on block lengths we will try.
/// (Even if `block` is bigger.)
const FAR_BLOCK_LEN_HARD_CAP: usize = 256;

/// Practical caps by alphabet size (keeps the parameter sweep sane).
///
/// You can loosen these if you want, but for `COLORS > 2` huge block lengths are
/// rarely helpful and tend to explode state-space.
const FAR_BLOCK_LEN_CAP_COLORS_2: usize = 256;
const FAR_BLOCK_LEN_CAP_COLORS_3_4: usize = 128;
const FAR_BLOCK_LEN_CAP_COLORS_5_8: usize = 64;

// Summary parameters ----------------------------------------------------------

/// C++ FAR::NG1 default parameters.
const FAR_NG1_N: usize = 3;

/// C++ FAR::NG parameters plus a few fixed internal variants.
///
/// The C++ default is `NG_n = 3, tH = 0, pos_mod = 1`, which is
/// equivalent to `NG1` for a fixed block size.  The extra Rust variants use
/// non-zero tail history and position modulo without changing the public API.
const FAR_NG_N: usize = 3;
const FAR_NG_TAIL_H_SMALL: usize = 1;
const FAR_NG_TAIL_H_MED: usize = 2;
const FAR_NG_POS_MOD_2: usize = 2;
const FAR_NG_POS_MOD_3: usize = 3;

/// C++ MitM_CTL::NGset defaults, usable as a FAR summary as well.
const FAR_NGSET_NG_N: usize = 3;
const FAR_NGSET_LEN_H: usize = 64;

/// C++ MitM_CTL::LRUpair-style bounded recent-pair memory.
///
/// The published default has `len_h_tail = 0`, which degenerates to a short
/// recent-block queue.  This internal FAR variant keeps one protected tail
/// block so the pair LRU actually carries extra information.
const FAR_LRU_PAIR_LEN_H: usize = 8;
const FAR_LRU_PAIR_LEN_H_NO_LRU: usize = 2;
const FAR_LRU_PAIR_LEN_H_TAIL: usize = 1;

/// C++ MitM_CTL::set_pair defaults, usable as a FAR summary as well.
const FAR_SET_PAIR_LEN_H: usize = 16;
const FAR_SET_PAIR_LEN_H_NO_LRU: usize = 2;
const FAR_SET_PAIR_LEN_H_TAIL: usize = 1;

/// Faithful BusyCoq RWL_mod default profile.
///
/// RWL_mod has four independent transform parameters:
/// `(mnc, mod_, len1, len2)`.  `len1` protects the newest prefix exactly;
/// when the total retained list exceeds `len1 + len2`, the element immediately
/// after that protected prefix is discarded.  This is intentionally different
/// from a plain fixed-length FIFO/LRU queue.
const FAR_RWL_DEFAULT_MNC: u8 = 2;
const FAR_RWL_DEFAULT_LEN1: u8 = 8;
const FAR_RWL_DEFAULT_LEN2: u8 = 0;
const FAR_RWL_DEFAULT_MODS: &[u8] = &[1, 2, 3];

/// Additional exact RWL_mod profiles.  Every tuple is
/// `(mnc, mod_, len1, len2)` and is crossed independently with every FAR block
/// length in the late generalized sweep.
const FAR_RWL_GENERAL_PROFILES: &[(u8, u8, u8, u8)] = &[
    (2, 0, 8, 0),
    (2, 0, 4, 4),
    (2, 1, 4, 4),
    (2, 2, 4, 4),
    (2, 3, 4, 4),
    (2, 1, 2, 6),
    (2, 2, 2, 6),
    (2, 3, 2, 6),
    (2, 2, 1, 7),
    (2, 2, 0, 8),
    (1, 2, 8, 0),
    (3, 2, 8, 0),
    (1, 2, 4, 4),
    (3, 2, 4, 4),
    (2, 4, 8, 0),
    (2, 4, 4, 4),
    (2, 2, 2, 2),
];

const FAR_RWL_GENERAL_BLOCK_LEN_CAP: usize = 31;

/// C++ FAR::CPS_LRU defaults.
const FAR_CPS_LRU_LEN_H: usize = 8;
const FAR_CPS_LRU_LEN_H_NO_LRU: usize = 2;

/// Upstream FAR sweeps commonly take `n` through roughly 1..=31 while varying
/// CPS_LRU parameters separately.  Keep this as a late holdout pass instead of
/// multiplying the hot raw-FAR path at very large block sizes.
const FAR_CPS_LRU_EXACT_BLOCK_LEN_CAP: usize = 31;

/// Exact upstream CPS_LRU parameter triples `(len1, len2, len3)`.
///
/// `len1` is the protected exact prefix of x12, `len2` is the LRU-updated
/// suffix capacity, and `len3` is the exact x3 stack filled before x12 is
/// touched.  These profiles subsume the removed experimental `(LRUH,H,tH)`
/// portfolio at `LRU_n = 0` under `(len1,len2,len3) = (H,LRUH,tH)`.
const FAR_CPS_LRU_EXACT_PROFILES: &[(u8, u8, u8)] = &[
    (0, 2, 0),
    (0, 1, 0),
    (1, 3, 0),
    (0, 3, 0),
    (0, 4, 0),
    (0, 2, 1),
    (1, 2, 1),
    (0, 3, 1),
    (1, 4, 1),
    (0, 4, 2),
];

/// Upstream `LRU_n` is the zero-based matching duplicate removed from the
/// LRU suffix.  Sweep it independently of both the history-size triple and
/// FAR block/DFA size.  Values above two are easy to add if this axis pays.
const FAR_CPS_LRU_EXACT_LRU_NS: &[u8] = &[0, 1, 2];

/// FAR over tape symbols augmented with finite per-cell execution history.
///
/// The full LRU history macro is finite: for a base machine with K slots, each
/// cell history is a duplicate-free recency ordering of at most K slots.  It is
/// nevertheless a much larger alphabet than the raw machine, so keep this late
/// pass on the same conservative block-length cap previously used by macro FAR.
const FAR_HISTORY_BLOCK_LEN_CAP: usize = 64;

/// Exact finite tail-signature channels borrowed from standalone CPS.
///
/// Each channel stores the polynomial residue of the entire pushed FAR-block
/// stack, including blocks that have fallen out of the bounded CPS-LRU list.
/// For a tape sequence c0,c1,... from nearest to farthest, the residue is
/// c0 + base*c1 + base^2*c2 + ... (mod modulus).  Prepending a block is thus
/// an exact finite-state update.  Collisions only merge histories, so this is
/// a sound refinement of the ordinary CPS-LRU summary.
const FAR_CPS_SIG_REFINEMENTS: [(u16, u16); 2] = [(2, 3), (3, 4)];

/// Full per-macro-color supply summary borrowed from standalone CPS.
/// `Color` is u8, so four u64 words cover every possible raw or macro color.
const FAR_CPS_COLOR_WORDS: usize = 4;

/// Faithful BusyCoq RNGS_mod default profile.
///
/// The full parameterization is `(mnc, mod_, NG_n, len_h, bs_n)`.
/// `NG_n` is a count of FAR *block symbols*, not a count of raw tape cells.
/// `bs_n` is the exact staging-buffer length before an n-gram is promoted into
/// the repeated-history list.
const FAR_RNGS_DEFAULT_MNC: u8 = 2;
const FAR_RNGS_DEFAULT_MOD: u8 = 1;
const FAR_RNGS_DEFAULT_NG_N: u8 = 4;
const FAR_RNGS_DEFAULT_LEN_H: u8 = 8;
const FAR_RNGS_DEFAULT_BS_N: u8 = 0;

/// Additional exact RNGS_mod profiles.  Every tuple is
/// `(mnc, mod_, NG_n, len_h, bs_n)` and is crossed independently with FAR size.
const FAR_RNGS_GENERAL_PROFILES: &[(u8, u8, u8, u8, u8)] = &[
    (2, 2, 4, 8, 0),
    (2, 3, 4, 8, 0),
    (2, 1, 4, 8, 1),
    (2, 2, 4, 8, 1),
    (2, 3, 4, 8, 1),
    (2, 1, 4, 8, 2),
    (2, 2, 4, 8, 2),
    (2, 3, 4, 8, 2),
    (2, 2, 4, 8, 3),
    (2, 2, 3, 8, 1),
    (2, 2, 2, 8, 1),
    (2, 2, 1, 8, 1),
    (2, 2, 4, 4, 1),
    (2, 2, 4, 16, 1),
    (1, 2, 4, 8, 1),
    (3, 2, 4, 8, 1),
    (2, 0, 4, 8, 1),
    (2, 2, 0, 8, 1),
    (2, 2, 4, 0, 1),
];

const FAR_RNGS_GENERAL_BLOCK_LEN_CAP: usize = 31;
const FAR_RNGS_INLINE_NG_CAP: usize = 4;

/// C++ FAR::RS_mod defaults.
const FAR_RS_NG_N: usize = 4;
const FAR_RS_LEN_H: usize = 8;
const FAR_RS_MNC: u8 = 2;
const FAR_RS_STRICT: bool = true;

// MITM parameters -------------------------------------------------------------

const MITM_DEAD: usize = 1;
const MITM_MAX_FINITE_INTERVAL: i32 = 100;
const MITM_MAX_TRANSITIONS: usize = 10;
const MITM_MAX_WEIGHT_PAIRS: usize = 1;

// Rejection paths are local to one closed MITM skeleton and memory profile.
// They are only replayed as exact concrete abstract paths; a replay hit rejects
// a candidate but is never used to certify a non-target proof.
const MITM_MAX_REJECT_PATHS: usize = 4;
const MITM_MAX_REJECT_PATH_LEN: usize = 256;
const MITM_MAX_REJECT_PARENTS: usize = 4096;

/// MITM finite-memory refinements tried for every closed DFA skeleton and
/// every weight assignment.  Keeping these in one shared portfolio avoids
/// re-enumerating the same DFA skeleton separately for each memory profile.
const MITM_MEMORY_PROFILES: &[MitmMemory] = &[
    MitmMemory::new(0, 0),
    MitmMemory::new(1, 0),
    MitmMemory::new(0, 1),
    MitmMemory::new(1, 1),
    MitmMemory::new(2, 0),
    MitmMemory::new(0, 2),
];

// Direct FAR parameters -------------------------------------------------------

/// Late direct-DFA FAR pass.
///
/// This searches arbitrary small one-cell DFAs, unlike the summary-based FAR
/// passes above.  The search is intentionally bounded and runs only after the
/// existing FAR/MITM portfolio fails.
const DIRECT_FAR_MAX_DFA_STATES: usize = 7;
const DIRECT_FAR_MAX_NFA_STATES: usize = 128;
const DIRECT_FAR_TARGET_STATES: usize = 2;
const DIRECT_FAR_MAX_DFA_ENTRIES: usize = 18;
const DIRECT_FAR_MAX_WORK: usize = 350_000;
const DIRECT_FAR_MAX_REJECTS: usize = 64;

const fn direct_far_bit(idx: usize) -> u128 {
    1_u128 << idx
}

const fn direct_far_idx(
    dfa_state: usize,
    ctrl_state: usize,
    ctrl_states: usize,
) -> usize {
    dfa_state * ctrl_states + ctrl_state
}

const fn direct_far_move_code(shift_right: bool) -> u8 {
    // TonyGuil's direct FAR uses 0 = right, 1 = left.
    if shift_right { 0 } else { 1 }
}

const fn direct_far_vec_times_matrix(
    mut v: u128,
    matrix: &[u128],
) -> u128 {
    let mut out = 0_u128;
    while v != 0 {
        let idx = v.trailing_zeros() as usize;
        out |= matrix[idx];
        v &= v - 1;
    }
    out
}

fn direct_far_matrix_times_vec(
    matrix: &[u128],
    v: u128,
    nfa_states: usize,
) -> u128 {
    let mut out = 0_u128;
    #[expect(clippy::disallowed_names)]
    for (idx, row) in matrix.iter().take(nfa_states).enumerate() {
        if row & v != 0 {
            out |= direct_far_bit(idx);
        }
    }
    out
}

/// Extend zero-suffix acceptance using the current NFA lower bound. Target
/// sinks have zero self-loops, so acceptance only grows as NFA edges are added.
/// Once the initial configuration is accepted, no DFA completion can rescue
/// this branch; the rest of its left-rule saturation can be skipped.
fn direct_far_extend_accept(
    zero_matrix: &[u128],
    a: &mut u128,
    nfa_states: usize,
    deps: &mut DirectFarDeps,
) -> bool {
    // direct_far_idx(0, 0, ctrl_states) is always zero.
    loop {
        let accepted = zero_matrix[0] & *a;
        if accepted != 0 {
            deps.reject = deps.rows[0][0]
                | deps.accept[accepted.trailing_zeros() as usize];
            return false;
        }
        let next_accept =
            direct_far_matrix_times_vec(zero_matrix, *a, nfa_states);
        if next_accept == *a {
            return true;
        }
        let mut added = next_accept & !*a;
        while added != 0 {
            let src = added.trailing_zeros() as usize;
            let dst = (zero_matrix[src] & *a).trailing_zeros() as usize;
            deps.accept[src] = deps.rows[0][src] | deps.accept[dst];
            added &= added - 1;
        }
        *a = next_accept;
    }
}

// A row mask supports every currently present edge in that row. This is a
// conservative union of dependencies, not a minimal witness. Existing proofs
// remain valid when later edges acquire additional dependencies.
#[derive(Clone)]
struct DirectFarDeps {
    rows: Vec<Vec<u32>>,
    accept: Vec<u32>,
    reject: u32,
}

struct DirectFarReject {
    entries: u32,
    value_mask: u128,
    values: u128,
}

#[derive(Default)]
struct DirectFarRejectCache {
    witnesses: Vec<DirectFarReject>,
    next_replace: usize,
}

impl DirectFarRejectCache {
    fn rejects(&self, assignments: u128, fixed: u32) -> bool {
        self.witnesses.iter().any(|witness| {
            witness.entries & !fixed == 0
                && assignments & witness.value_mask == witness.values
        })
    }

    fn remember(
        &mut self,
        entries: u32,
        assignments: u128,
        fixed: u32,
    ) {
        debug_assert_eq!(entries & !fixed, 0);
        // Entry zero is fixed to zero in every candidate. A witness depending
        // on every other prefix entry cannot recur in this depth-first search.
        let entries = entries & !1;
        if entries == (fixed & !1) {
            return;
        }
        if self.rejects(assignments, entries) {
            return;
        }
        let mut remaining = entries;
        let mut value_mask = 0_u128;
        while remaining != 0 {
            value_mask |= 15_u128 << (4 * remaining.trailing_zeros());
            remaining &= remaining - 1;
        }
        let witness = DirectFarReject {
            entries,
            value_mask,
            values: assignments & value_mask,
        };
        if self.witnesses.len() < DIRECT_FAR_MAX_REJECTS {
            self.witnesses.push(witness);
        } else {
            self.witnesses[self.next_replace] = witness;
            self.next_replace =
                (self.next_replace + 1) % DIRECT_FAR_MAX_REJECTS;
        }
    }
}

// -----------------------------------------------------------------------------
// Public method on Prog
// -----------------------------------------------------------------------------

impl<const STATES: usize, const COLORS: usize> Prog<STATES, COLORS> {
    /// FAR non-halting prover (Finite Automaton Reduction).
    ///
    /// `block` is the only user-facing parameter.
    /// Higher values try every block length attempted by lower values, plus
    /// additional larger block lengths (up to the internal safety caps).
    ///
    /// Returns:
    /// - `true` iff FAR or MITM/WFAR proved the machine cannot halt.
    /// - `false` otherwise.
    pub fn far_cant_halt(&self, block: usize) -> bool {
        self.far_cant_target(block, Goal::Halt)
    }

    /// FAR blank-tape prover.
    ///
    /// Returns `true` iff FAR proved that the machine can never blank the tape
    /// after time 0.  The initial all-zero tape is ignored.  Every blanking event
    /// after time 0 has a first occurrence, and that occurrence must be a concrete
    /// transition that reads a nonzero symbol, writes zero, and leaves every other
    /// tape cell zero.  FAR, MITM, and direct FAR all use that last-erasing
    /// transition as the target rather than a history-tagged all-zero endpoint.
    /// For summary FAR, Blank uses a product DFA whose state contains both the
    /// configured history summary and an exact all-semantically-blank bit.  This
    /// keeps zero-context provenance correlated with the same DFA path even when
    /// LRU/RWL/RNGS summaries merge different concrete stacks.
    pub fn far_cant_blank(&self, block: usize) -> bool {
        self.far_cant_target(block, Goal::Blank)
    }

    /// FAR spinout prover.
    ///
    /// Returns `true` iff FAR proves that the machine can never enter a
    /// one-sided all-zero same-state drift.
    pub fn far_cant_spinout(&self, block: usize) -> bool {
        self.far_cant_target(block, Goal::Spinout)
    }
}

// -----------------------------------------------------------------------------
// Internal implementation
// -----------------------------------------------------------------------------

/// A block-word: a length-`len` vector of tape symbols.
///
/// `Word` uses a `Vec<Color>` so it supports any number of colors.  The cached
/// fingerprint makes hash-table probes O(1) in the block length.  Equality still
/// compares the full cells, so fingerprint collisions affect performance only.
#[derive(Clone, Debug)]
struct Word {
    cells: Vec<Color>,
    fingerprint: u64,
}

impl Word {
    #[inline]
    const fn fingerprint_base(len: usize) -> u64 {
        (len as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93)
    }

    #[inline]
    fn cell_fingerprint(idx: usize, color: Color) -> u64 {
        // Canonical zero contributes nothing.  This makes construction of the
        // overwhelmingly common all-zero block O(1), while the position is mixed
        // into every nonzero contribution so permutations normally hash apart.
        if color == 0 {
            return 0;
        }
        let x = (((idx as u64) << 8) | u64::from(color))
            .wrapping_mul(0x9E37_79B1_85EB_CA87);
        x ^ x.rotate_left(29) ^ (x >> 23)
    }

    fn recompute_fingerprint(&mut self) {
        let mut fingerprint = Self::fingerprint_base(self.cells.len());
        for (idx, &color) in self.cells.iter().enumerate() {
            fingerprint ^= Self::cell_fingerprint(idx, color);
        }
        self.fingerprint = fingerprint;
    }

    fn zero(len: usize) -> Self {
        Self {
            cells: vec![0; len],
            fingerprint: Self::fingerprint_base(len),
        }
    }

    const fn len(&self) -> usize {
        self.cells.len()
    }

    fn is_zero(&self) -> bool {
        self.cells.iter().all(|&x| x == 0)
    }

    fn get(&self, idx: usize) -> Color {
        self.cells[idx]
    }

    #[inline]
    fn set(&mut self, idx: usize, v: Color) {
        let old = self.cells[idx];
        if old == v {
            return;
        }
        self.fingerprint ^= Self::cell_fingerprint(idx, old)
            ^ Self::cell_fingerprint(idx, v);
        self.cells[idx] = v;
    }

    fn zero_to_left_of(&self, idx: usize) -> bool {
        self.cells[..idx].iter().all(|&x| x == 0)
    }

    fn zero_to_right_of(&self, idx: usize) -> bool {
        self.cells[idx + 1..].iter().all(|&x| x == 0)
    }

    /// Reverse the cell order in the word (length-preserving).
    fn reverse(mut self) -> Self {
        self.cells.reverse();
        self.recompute_fingerprint();
        self
    }
}

impl PartialEq for Word {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.fingerprint == other.fingerprint
            && self.cells == other.cells
    }
}

impl Eq for Word {}

impl Hash for Word {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.fingerprint);
    }
}

impl Ord for Word {
    fn cmp(&self, other: &Self) -> Ordering {
        self.cells.cmp(&other.cells)
    }
}

impl PartialOrd for Word {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(
    Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug, Hash, Default,
)]
struct WordId(u32);

#[derive(Clone, Debug)]
struct WordInterner {
    // Store each full Word only once, in `words`. The primary table uses the
    // cached fingerprint as its compact key; collision storage is allocated
    // only if two unequal Words actually share the same u64 fingerprint.
    ids: Map<u64, WordId>,
    collisions: Map<u64, Vec<WordId>>,
    words: Vec<Word>,
}

impl WordInterner {
    fn new() -> Self {
        Self {
            ids: Map::new(),
            collisions: Map::new(),
            words: Vec::new(),
        }
    }

    fn intern(&mut self, w: Word) -> WordId {
        let fingerprint = w.fingerprint;
        if let Some(&id) = self.ids.get(&fingerprint) {
            if self.words[id.0 as usize] == w {
                return id;
            }
            if let Some(collisions) = self.collisions.get(&fingerprint)
            {
                for &candidate in collisions {
                    if self.words[candidate.0 as usize] == w {
                        return candidate;
                    }
                }
            }

            let id =
                WordId(u32::try_from(self.words.len()).expect(
                    "FAR word interner exceeded u32::MAX entries",
                ));
            self.words.push(w);
            self.collisions.entry(fingerprint).or_default().push(id);
            return id;
        }

        let id = WordId(
            u32::try_from(self.words.len())
                .expect("FAR word interner exceeded u32::MAX entries"),
        );
        self.words.push(w);
        self.ids.insert(fingerprint, id);
        id
    }

    fn get(&self, id: WordId) -> &Word {
        &self.words[id.0 as usize]
    }

    fn clone_word(&self, id: WordId) -> Word {
        self.get(id).clone()
    }
}

/// Result of simulating the TM within one block until it exits or halts.
#[derive(Clone, Copy, Debug)]
struct WordUpdateLemma {
    w1: WordId,
    s1: Option<State>,
    is_back: bool,
    hit_blank: bool,
}

/// Cached result of an exact simulation within one FAR block.  `hit_blank` on
/// a local loop is used only by the deferred-Blank closure; ordinary FAR stops
/// at the first target and therefore never returns a target-bearing local loop.
#[derive(Clone, Copy, Debug)]
enum WordUpdateOutcome {
    Exit(WordUpdateLemma),
    /// The exact local configuration repeated before leaving the block.
    /// Determinism then guarantees that this branch stays in the block forever.
    LocalLoop {
        hit_blank: bool,
    },
    /// The simulation reached its step bound.
    Incomplete,
}

/// Whether an exact block simulation should stop at a target immediately or
/// merely remember Blank targets while continuing to the ordinary exit/loop
/// boundary.  Keeping this policy explicit lets both FAR paths share one exact
/// simulator without weakening the conditional-Blank closure.
#[derive(Clone, Copy)]
enum BlockTargetMode {
    Immediate(Goal),
    DeferredBlank,
}

impl BlockTargetMode {
    const fn goal(self) -> Goal {
        match self {
            Self::Immediate(goal) => goal,
            Self::DeferredBlank => Goal::Blank,
        }
    }

    const fn defers_blank(self) -> bool {
        matches!(self, Self::DeferredBlank)
    }
}

/// Non-interned result used only while computing a cache miss.
#[derive(Clone, Debug)]
struct RawWordUpdateLemma {
    w1: Word,
    s1: Option<State>,
    is_back: bool,
    hit_blank: bool,
}

#[derive(Clone, Debug)]
enum RawWordUpdateOutcome {
    Exit(RawWordUpdateLemma),
    LocalLoop { hit_blank: bool },
    Incomplete,
}

impl RawWordUpdateLemma {
    fn exit_oriented(
        w1: Word,
        s1: Option<State>,
        is_back: bool,
        hit_blank: bool,
    ) -> Self {
        let mut res = Self {
            w1,
            s1,
            is_back,
            hit_blank,
        };
        if res.s1.is_some() && !res.is_back {
            res.w1 = res.w1.reverse();
        }
        res
    }
}

/// History summarizer used by the FAR DFA.
trait Summary: Clone + Eq + Hash {
    type Config: Copy;

    fn new(config: Self::Config) -> Self;
    fn push(
        &mut self,
        w: WordId,
        words: &mut WordInterner,
        config: Self::Config,
    ) -> Result<(), SummaryOverflow>;

    /// Conservative summary-level all-zero compatibility.  Blank target
    /// detection does not trust this lossy predicate by itself: Blank's DFA key
    /// is producted with an exact semantic all-blank bit.  The predicate remains
    /// useful for summaries and for consistency checks on canonical-zero goals.
    fn may_be_all_zero_context(&self, words: &WordInterner) -> bool;
}

#[derive(Clone, Copy, Debug)]
struct SummaryOverflow;

/// Fixed-capacity vector for summary fragments with a genuinely tiny hard cap.
/// It hashes and compares exactly like the live slice, but needs no allocation.
#[derive(Clone, Copy)]
struct InlineVec<T: Copy + Default, const N: usize> {
    data: [T; N],
    len: u8,
}

impl<T: Copy + Default, const N: usize> InlineVec<T, N> {
    fn new() -> Self {
        debug_assert!(u8::try_from(N).is_ok());
        Self {
            data: [T::default(); N],
            len: 0,
        }
    }
    #[inline]
    fn push(&mut self, value: T) {
        let len = self.len as usize;
        debug_assert!(len < N, "InlineVec capacity exceeded");
        self.data[len] = value;
        self.len += 1;
    }
    #[inline]
    fn insert(&mut self, pos: usize, value: T) {
        let len = self.len as usize;
        debug_assert!(
            pos <= len && len < N,
            "InlineVec capacity exceeded"
        );
        for i in (pos..len).rev() {
            self.data[i + 1] = self.data[i];
        }
        self.data[pos] = value;
        self.len += 1;
    }
    #[inline]
    fn remove(&mut self, pos: usize) -> T {
        let len = self.len as usize;
        debug_assert!(pos < len);
        let out = self.data[pos];
        for i in pos + 1..len {
            self.data[i - 1] = self.data[i];
        }
        self.len -= 1;
        self.data[self.len as usize] = T::default();
        out
    }
    #[inline]
    fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        let idx = self.len as usize;
        let out = self.data[idx];
        self.data[idx] = T::default();
        Some(out)
    }
    #[inline]
    fn truncate(&mut self, len: usize) {
        while self.len as usize > len {
            let _ = self.pop();
        }
    }
    #[inline]
    fn as_slice(&self) -> &[T] {
        &self.data[..self.len as usize]
    }
}
impl<T: Copy + Default, const N: usize> Default for InlineVec<T, N> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T: Copy + Default, const N: usize> core::ops::Deref
    for InlineVec<T, N>
{
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}
impl<T: Copy + Default, const N: usize> core::ops::DerefMut
    for InlineVec<T, N>
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        let len = self.len as usize;
        &mut self.data[..len]
    }
}
impl<T: Copy + Default + PartialEq, const N: usize> PartialEq
    for InlineVec<T, N>
{
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}
impl<T: Copy + Default + Eq, const N: usize> Eq for InlineVec<T, N> {}
impl<T: Copy + Default + PartialOrd, const N: usize> PartialOrd
    for InlineVec<T, N>
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.as_slice().partial_cmp(other.as_slice())
    }
}
impl<T: Copy + Default + Ord, const N: usize> Ord for InlineVec<T, N> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_slice().cmp(other.as_slice())
    }
}
impl<T: Copy + Default + Hash, const N: usize> Hash
    for InlineVec<T, N>
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.as_slice().hash(state);
    }
}
impl<T: Copy + Default + core::fmt::Debug, const N: usize>
    core::fmt::Debug for InlineVec<T, N>
{
    fn fmt(
        &self,
        f: &mut core::fmt::Formatter<'_>,
    ) -> core::fmt::Result {
        self.as_slice().fmt(f)
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct RepeatWord {
    w: WordId,
    n: u8,
}

impl RepeatWord {
    const fn new(w: WordId, n: u8) -> Self {
        Self { w, n }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct Ng1Summary {
    q: InlineVec<WordId, FAR_NG1_N>,
}

impl Summary for Ng1Summary {
    type Config = ();

    fn new(_config: ()) -> Self {
        Self {
            q: InlineVec::new(),
        }
    }

    fn push(
        &mut self,
        w: WordId,
        words: &mut WordInterner,
        _config: (),
    ) -> Result<(), SummaryOverflow> {
        if self.q.is_empty() && words.get(w).is_zero() {
            return Ok(());
        }
        if self.q.len() == FAR_NG1_N {
            self.q.remove(0);
        }
        self.q.push(w);
        Ok(())
    }

    fn may_be_all_zero_context(&self, words: &WordInterner) -> bool {
        self.q.iter().all(|&w| words.get(w).is_zero())
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct NgSummary<const TAIL_H: usize, const POS_MOD: usize> {
    q: InlineVec<WordId, FAR_NG_N>,
    q0: InlineVec<WordId, TAIL_H>,
    mod_pos: u8,
}

impl<const TAIL_H: usize, const POS_MOD: usize> Summary
    for NgSummary<TAIL_H, POS_MOD>
{
    type Config = ();

    fn new(_config: ()) -> Self {
        Self {
            q: InlineVec::new(),
            q0: InlineVec::new(),
            mod_pos: 0,
        }
    }

    #[expect(clippy::unwrap_in_result)]
    fn push(
        &mut self,
        w: WordId,
        words: &mut WordInterner,
        _config: (),
    ) -> Result<(), SummaryOverflow> {
        if self.q.is_empty() && words.get(w).is_zero() {
            return Ok(());
        }

        if self.q.len() == FAR_NG_N {
            self.q.remove(0);
        }
        self.q.push(w);

        if self.q0.len() < TAIL_H {
            self.q0.push(w);
        }

        self.mod_pos =
            u8::try_from((usize::from(self.mod_pos) + 1) % POS_MOD)
                .expect("NG position modulus must fit u8");
        Ok(())
    }

    fn may_be_all_zero_context(&self, words: &WordInterner) -> bool {
        self.q.iter().all(|&w| words.get(w).is_zero())
            && self.q0.iter().all(|&w| words.get(w).is_zero())
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct NgSetSummary {
    q: InlineVec<WordId, FAR_NGSET_NG_N>,
    lru: Vec<InlineVec<WordId, FAR_NGSET_NG_N>>,
}

impl Summary for NgSetSummary {
    type Config = ();

    fn new(_config: ()) -> Self {
        Self {
            q: InlineVec::new(),
            lru: Vec::new(),
        }
    }

    fn push(
        &mut self,
        w: WordId,
        words: &mut WordInterner,
        _config: (),
    ) -> Result<(), SummaryOverflow> {
        if self.q.is_empty() && words.get(w).is_zero() {
            return Ok(());
        }

        let old_ngram = self.q;
        if self.q.len() == FAR_NGSET_NG_N {
            let _ = self.q.pop();
        }
        self.q.insert(0, w);

        if old_ngram.len() == FAR_NGSET_NG_N {
            match self.lru.binary_search(&old_ngram) {
                Ok(_) => {},
                Err(pos) => {
                    self.lru.insert(pos, old_ngram);
                    if self.lru.len() > FAR_NGSET_LEN_H {
                        return Err(SummaryOverflow);
                    }
                },
            }
        }

        Ok(())
    }

    fn may_be_all_zero_context(&self, words: &WordInterner) -> bool {
        self.q.iter().all(|&w| words.get(w).is_zero())
            && self.lru.iter().all(|ngram| {
                ngram.iter().all(|&w| words.get(w).is_zero())
            })
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct LruPairSummary {
    q: InlineVec<
        WordId,
        { FAR_LRU_PAIR_LEN_H_NO_LRU + FAR_LRU_PAIR_LEN_H_TAIL },
    >,
    lru: Vec<(WordId, WordId)>,
}

impl Summary for LruPairSummary {
    type Config = ();

    fn new(_config: ()) -> Self {
        Self {
            q: InlineVec::new(),
            lru: Vec::new(),
        }
    }

    fn push(
        &mut self,
        w: WordId,
        words: &mut WordInterner,
        _config: (),
    ) -> Result<(), SummaryOverflow> {
        if self.q.is_empty() && words.get(w).is_zero() {
            return Ok(());
        }

        if self.q.len()
            < FAR_LRU_PAIR_LEN_H_NO_LRU + FAR_LRU_PAIR_LEN_H_TAIL
        {
            self.q.push(w);
            return Ok(());
        }

        let i = FAR_LRU_PAIR_LEN_H_TAIL;
        let pair = (self.q[i - 1], self.q[i]);
        if let Some(pos) = self.lru.iter().position(|&p| p == pair) {
            self.lru.remove(pos);
        } else {
            let max_lru = FAR_LRU_PAIR_LEN_H
                - FAR_LRU_PAIR_LEN_H_NO_LRU
                - FAR_LRU_PAIR_LEN_H_TAIL;
            if self.lru.len() >= max_lru {
                self.lru.pop();
            }
        }
        self.lru.insert(0, pair);

        self.q.remove(FAR_LRU_PAIR_LEN_H_TAIL);
        self.q.push(w);
        Ok(())
    }

    fn may_be_all_zero_context(&self, words: &WordInterner) -> bool {
        self.q.iter().all(|&w| words.get(w).is_zero())
            && self.lru.iter().all(|&(a, b)| {
                words.get(a).is_zero() && words.get(b).is_zero()
            })
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct SetPairSummary {
    q: InlineVec<
        WordId,
        { FAR_SET_PAIR_LEN_H_NO_LRU + FAR_SET_PAIR_LEN_H_TAIL },
    >,
    lru: Vec<(WordId, WordId)>,
}

impl Summary for SetPairSummary {
    type Config = ();

    fn new(_config: ()) -> Self {
        Self {
            q: InlineVec::new(),
            lru: Vec::new(),
        }
    }

    fn push(
        &mut self,
        w: WordId,
        words: &mut WordInterner,
        _config: (),
    ) -> Result<(), SummaryOverflow> {
        if self.q.is_empty() && words.get(w).is_zero() {
            return Ok(());
        }

        if self.q.len()
            < FAR_SET_PAIR_LEN_H_NO_LRU + FAR_SET_PAIR_LEN_H_TAIL
        {
            self.q.push(w);
            return Ok(());
        }

        let i = FAR_SET_PAIR_LEN_H_TAIL;
        let pair = (self.q[i - 1], self.q[i]);
        match self.lru.binary_search(&pair) {
            Ok(_) => {},
            Err(pos) => {
                self.lru.insert(pos, pair);
                if self.lru.len() > FAR_SET_PAIR_LEN_H {
                    return Err(SummaryOverflow);
                }
            },
        }

        self.q.remove(FAR_SET_PAIR_LEN_H_TAIL);
        self.q.push(w);
        Ok(())
    }

    fn may_be_all_zero_context(&self, words: &WordInterner) -> bool {
        self.q.iter().all(|&w| words.get(w).is_zero())
            && self.lru.iter().all(|&(a, b)| {
                words.get(a).is_zero() && words.get(b).is_zero()
            })
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct RwlModWord {
    w: WordId,
    n: u8,
    phase: u32,
}

impl RwlModWord {
    const fn new(w: WordId) -> Self {
        // BusyCoq RWL_mod starts a fresh run at (word, 1, 1) without first
        // reducing the phase modulo `mod_`.
        Self { w, n: 1, phase: 1 }
    }
}

/// Exact BusyCoq RWL_mod summary.
///
/// State is a newest-first list of `(word, saturated_count, modular_phase)`.
/// The four parameters are independent:
/// - `mnc`: saturation threshold for the exact repetition count,
/// - `modulus`: phase modulus,
/// - `len1`: protected newest-prefix length,
/// - `len2`: retained suffix length after the protected prefix.
///
/// BusyCoq's `limit_length len1 len2` does *not* simply drop the oldest entry:
/// if the suffix is too long it drops the first entry immediately after the
/// protected prefix.  Thus `(len1=8,len2=0)` is the ordinary "keep newest 8"
/// case, while `(len1=0,len2=8)` preferentially keeps older history.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct RwlModSummary {
    q: Vec<RwlModWord>,
}

impl RwlModSummary {
    fn limit_length(&mut self, len1: u8, len2: u8) {
        if self.q.len() <= usize::from(len1) {
            return;
        }

        let len1 = usize::from(len1);
        let suffix_len = self.q.len() - len1;
        if suffix_len > usize::from(len2) {
            self.q.remove(len1);
        }
    }
}

impl Summary for RwlModSummary {
    type Config = (u8, u8, u8, u8);

    fn new(_config: Self::Config) -> Self {
        Self { q: Vec::new() }
    }

    fn push(
        &mut self,
        w: WordId,
        words: &mut WordInterner,
        config: Self::Config,
    ) -> Result<(), SummaryOverflow> {
        let (mnc, modulus, len1, len2) = config;

        // Upstream's distinguished state is the empty list, with a self-loop
        // on an all-zero FAR block.
        if self.q.is_empty() {
            if !words.get(w).is_zero() {
                self.q.push(RwlModWord::new(w));
            }
            return Ok(());
        }

        if self.q[0].w == w {
            let head = &mut self.q[0];
            head.n = head.n.saturating_add(1).min(mnc);
            let phase = head.phase + 1;
            // Rocq's Uint63 remainder returns its dividend on divisor zero.
            head.phase = if modulus == 0 {
                phase
            } else {
                phase % u32::from(modulus)
            };
        } else {
            self.q.insert(0, RwlModWord::new(w));
        }

        self.limit_length(len1, len2);
        Ok(())
    }

    fn may_be_all_zero_context(&self, words: &WordInterner) -> bool {
        self.q.iter().all(|rw| words.get(rw.w).is_zero())
    }
}

/// Exact BB6/BusyCoq CPS_LRU FAR summary.
///
/// Upstream state is `(x12, x3)`.  While `x3.len() < len3`, pushes prepend to
/// `x3`.  Afterwards a push applies
/// `upd_skipn_LRU len1 len2 LRU_n (w :: x12)`: the first `len1` entries are
/// protected exactly, and the remaining suffix keeps at most `len2` entries
/// after removing the `LRU_n`-th duplicate of its new head.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct UpstreamCpsLruSummary {
    x12: Vec<WordId>,
    x3: InlineVec<WordId, 2>,
}

impl UpstreamCpsLruSummary {
    fn is_initial(&self) -> bool {
        self.x12.is_empty() && self.x3.is_empty()
    }

    /// Exact specialization of BusyCoq's
    /// `upd_skipn len1 (upd_LRU len2 LRU_n)` to `WordId`.
    fn update_x12(&mut self, w: WordId, len1: u8, len2: u8, lru_n: u8) {
        self.x12.insert(0, w);

        // `upd_skipn` preserves a list shorter than the protected prefix.
        let len1 = usize::from(len1);
        let len2 = usize::from(len2);
        if self.x12.len() <= len1 {
            return;
        }

        // `upd_LRU 0 ...` drops the entire suffix after the protected prefix.
        if len2 == 0 {
            self.x12.truncate(len1);
            return;
        }

        let head = self.x12[len1];
        let mut out = Vec::with_capacity(len1.saturating_add(len2));
        out.extend_from_slice(&self.x12[..len1]);
        out.push(head);

        let mut matching_seen = 0_u8;
        let mut removed = false;
        for &item in &self.x12[len1 + 1..] {
            if !removed && item == head {
                if matching_seen == lru_n {
                    removed = true;
                    continue;
                }
                matching_seen += 1;
            }

            if out.len() - len1 == len2 {
                break;
            }
            out.push(item);
        }

        self.x12 = out;
    }
}

impl Summary for UpstreamCpsLruSummary {
    type Config = (u8, u8, u8, u8);

    fn new(config: Self::Config) -> Self {
        debug_assert!(usize::from(config.2) <= 2);
        Self {
            x12: Vec::new(),
            x3: InlineVec::new(),
        }
    }

    fn push(
        &mut self,
        w: WordId,
        words: &mut WordInterner,
        config: Self::Config,
    ) -> Result<(), SummaryOverflow> {
        let (len1, len2, len3, lru_n) = config;

        // Match upstream `is_s0`: the empty summary has a self-loop on the
        // canonical all-zero block.
        if self.is_initial() && words.get(w).is_zero() {
            return Ok(());
        }

        if self.x3.len() < usize::from(len3) {
            self.x3.insert(0, w);
        } else {
            self.update_x12(w, len1, len2, lru_n);
        }
        Ok(())
    }

    fn may_be_all_zero_context(&self, words: &WordInterner) -> bool {
        self.x12.iter().all(|&w| words.get(w).is_zero())
            && self.x3.iter().all(|&w| words.get(w).is_zero())
    }
}

#[derive(
    Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash, Default,
)]
struct CpsLruCore {
    ls: Vec<WordId>,
}

impl CpsLruCore {
    fn push(&mut self, w: WordId, is_zero: bool) {
        if self.ls.is_empty() && is_zero {
            return;
        }

        self.ls.insert(0, w);
        if self.ls.len() <= FAR_CPS_LRU_LEN_H_NO_LRU {
            return;
        }

        let key = self.ls[FAR_CPS_LRU_LEN_H_NO_LRU];
        let start = FAR_CPS_LRU_LEN_H_NO_LRU + 1;
        let remove_idx = self.ls[start..]
            .iter()
            .position(|&old| old == key)
            .map(|i| start + i)
            .or_else(|| {
                (self.ls.len() > FAR_CPS_LRU_LEN_H)
                    .then_some(self.ls.len() - 1)
            });

        if let Some(i) = remove_idx {
            self.ls.remove(i);
        }
    }

    fn may_be_all_zero_context(&self, words: &WordInterner) -> bool {
        self.ls.iter().all(|&w| words.get(w).is_zero())
    }
}

fn cps_lru_update_signature(
    sig: &mut [u16; FAR_CPS_SIG_REFINEMENTS.len()],
    word: &Word,
) {
    // The block is prepended to the represented hidden stack.  Iterate the
    // block from farthest to nearest so repeated single-cell prepend updates
    // produce the polynomial residue of [word || old_tail].
    for &color in word.cells.iter().rev() {
        for (slot, &(base, modulus)) in
            sig.iter_mut().zip(FAR_CPS_SIG_REFINEMENTS.iter())
        {
            let value =
                u64::from(color) + u64::from(base) * u64::from(*slot);
            *slot = u16::try_from(value % u64::from(modulus)).expect(
                "CPS-LRU tail-signature residue must fit in u16",
            );
        }
    }
}

#[derive(
    Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash, Default,
)]
struct CpsLruSummary {
    core: CpsLruCore,
}

impl Summary for CpsLruSummary {
    type Config = ();

    fn new(_config: ()) -> Self {
        Self::default()
    }

    fn push(
        &mut self,
        w: WordId,
        words: &mut WordInterner,
        _config: (),
    ) -> Result<(), SummaryOverflow> {
        self.core.push(w, words.get(w).is_zero());
        Ok(())
    }

    fn may_be_all_zero_context(&self, words: &WordInterner) -> bool {
        self.core.may_be_all_zero_context(words)
    }
}

/// CPS-LRU plus permanent modular information about the entire pushed block
/// stack.  The bounded LRU retains strong recent ordering information; the
/// residues retain weak information about arbitrarily old blocks after LRU
/// eviction.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct CpsLruSigSummary {
    core: CpsLruCore,
    sig: [u16; FAR_CPS_SIG_REFINEMENTS.len()],
}

impl Summary for CpsLruSigSummary {
    type Config = ();

    fn new(_config: ()) -> Self {
        Self {
            core: CpsLruCore::default(),
            sig: [0; FAR_CPS_SIG_REFINEMENTS.len()],
        }
    }

    fn push(
        &mut self,
        w: WordId,
        words: &mut WordInterner,
        _config: (),
    ) -> Result<(), SummaryOverflow> {
        let word = words.get(w);
        cps_lru_update_signature(&mut self.sig, word);

        // Preserve the ordinary CPS-LRU convention that an arbitrarily long
        // leading all-zero tail is represented by the initial state.  Zero
        // blocks also leave the polynomial signature at zero in that state.
        let initial_zero = self.core.ls.is_empty() && word.is_zero();
        if initial_zero {
            debug_assert!(self.sig.iter().all(|&x| x == 0));
        }
        self.core.push(w, word.is_zero());
        Ok(())
    }

    fn may_be_all_zero_context(&self, words: &WordInterner) -> bool {
        // Every genuinely all-zero stack has zero residues.  Non-zero stacks may
        // collide to zero, which is harmless: the exact zero_context DFA
        // reachability remains the authoritative blank-context test.
        self.sig.iter().all(|&x| x == 0)
            && self.core.may_be_all_zero_context(words)
    }
}

/// Per-color saturated lower bounds plus exact parity, as in standalone CPS.
/// For each non-canonical-zero color independently, `one` means count >= 1,
/// `two` means count >= 2, and `odd` is exact parity.
#[derive(
    Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug, Hash, Default,
)]
struct FarCpsColorSummary {
    one: [u64; FAR_CPS_COLOR_WORDS],
    two: [u64; FAR_CPS_COLOR_WORDS],
    odd: [u64; FAR_CPS_COLOR_WORDS],
}

impl FarCpsColorSummary {
    fn bit(color: Color) -> (usize, u64) {
        let idx = usize::from(color);
        debug_assert!(idx < FAR_CPS_COLOR_WORDS * 64);
        (idx / 64, 1_u64 << (idx % 64))
    }

    const fn normalize_word(&mut self, word: usize) {
        self.one[word] |= self.two[word] | self.odd[word];
        self.two[word] |= self.one[word] & !self.odd[word];
        self.one[word] |= self.two[word];
    }

    fn add_nonzero(&mut self, color: Color) {
        debug_assert_ne!(color, 0);
        let (word, bit) = Self::bit(color);
        if self.one[word] & bit != 0 {
            self.two[word] |= bit;
        } else {
            self.one[word] |= bit;
        }
        self.odd[word] ^= bit;
        self.normalize_word(word);
    }

    fn add_word(&mut self, word: &Word) {
        for &color in &word.cells {
            if color != 0 {
                self.add_nonzero(color);
            }
        }
    }

    fn is_empty(&self) -> bool {
        self.one.iter().all(|&x| x == 0)
            && self.two.iter().all(|&x| x == 0)
            && self.odd.iter().all(|&x| x == 0)
    }
}

/// The useful full-color FAR-CPS variant: bounded CPS-LRU + permanent
/// polynomial tail signatures + the standalone CPS per-color supply/parity
/// component. This is kept as a separate late quotient because it is more
/// expensive than signature-only CPS-LRU but has produced an additional Halt
/// proof on the current holdout set.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct CpsLruSigColorSummary {
    core: CpsLruCore,
    sig: [u16; FAR_CPS_SIG_REFINEMENTS.len()],
    colors: FarCpsColorSummary,
}

impl Summary for CpsLruSigColorSummary {
    type Config = ();

    fn new(_config: ()) -> Self {
        Self {
            core: CpsLruCore::default(),
            sig: [0; FAR_CPS_SIG_REFINEMENTS.len()],
            colors: FarCpsColorSummary::default(),
        }
    }

    fn push(
        &mut self,
        w: WordId,
        words: &mut WordInterner,
        _config: (),
    ) -> Result<(), SummaryOverflow> {
        let word = words.get(w);
        cps_lru_update_signature(&mut self.sig, word);
        self.colors.add_word(word);

        let initial_zero = self.core.ls.is_empty() && word.is_zero();
        if initial_zero {
            debug_assert!(self.sig.iter().all(|&x| x == 0));
            debug_assert!(self.colors.is_empty());
        }
        self.core.push(w, word.is_zero());
        Ok(())
    }

    fn may_be_all_zero_context(&self, words: &WordInterner) -> bool {
        self.colors.is_empty()
            && self.sig.iter().all(|&x| x == 0)
            && self.core.may_be_all_zero_context(words)
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct RngsModWord {
    /// One upstream n-gram symbol.  This is a sequence of FAR block IDs, not a
    /// concatenation of the raw cells inside those blocks.
    w: InlineVec<WordId, FAR_RNGS_INLINE_NG_CAP>,
    n: u8,
    phase: u32,
}

impl RngsModWord {
    const fn new(w: InlineVec<WordId, FAR_RNGS_INLINE_NG_CAP>) -> Self {
        Self { w, n: 1, phase: 1 }
    }
}

/// Exact BusyCoq RNGS_mod summary.
///
/// Upstream state is `(x0, x2, x1)`:
/// - `x0`: newest-first n-gram of the last `NG_n` FAR block symbols,
/// - `x2`: exact staging queue of n-gram symbols, capacity `bs_n`,
/// - `x1`: bounded repeated-ngram history, capacity `len_h`.
///
/// Once `x2` is full, its oldest n-gram is promoted into `x1`.  A repeated
/// promoted n-gram is removed from its old position, its saturated count and
/// modular phase are updated, and it is moved to the front.  This is materially
/// different from the old Rust approximation, which concatenated raw block
/// cells and had no staging queue or modular phase.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct RngsModSummary {
    x0: InlineVec<WordId, FAR_RNGS_INLINE_NG_CAP>,
    x2: Vec<InlineVec<WordId, FAR_RNGS_INLINE_NG_CAP>>,
    x1: Vec<RngsModWord>,
}

impl RngsModSummary {
    fn ngram_is_all_zero(
        ngram: &[WordId],
        words: &WordInterner,
    ) -> bool {
        ngram.iter().all(|&wid| words.get(wid).is_zero())
    }

    fn promote_ngram(
        &mut self,
        w: InlineVec<WordId, FAR_RNGS_INLINE_NG_CAP>,
        mnc: u8,
        modulus: u8,
        len_h: u8,
    ) {
        if let Some(pos) = self
            .x1
            .iter()
            .position(|old| old.w.as_slice() == w.as_slice())
        {
            let mut old = self.x1.remove(pos);
            old.n = old.n.saturating_add(1).min(mnc);
            if modulus != 0 {
                old.phase = (old.phase + 1) % u32::from(modulus);
            }
            self.x1.insert(0, old);
        } else {
            self.x1.insert(0, RngsModWord::new(w));
        }

        self.x1.truncate(usize::from(len_h));
    }
}

impl Summary for RngsModSummary {
    type Config = (u8, u8, u8, u8, u8);

    fn new(config: Self::Config) -> Self {
        debug_assert!(usize::from(config.2) <= FAR_RNGS_INLINE_NG_CAP);
        Self {
            x0: InlineVec::new(),
            x2: Vec::new(),
            x1: Vec::new(),
        }
    }

    fn push(
        &mut self,
        w: WordId,
        words: &mut WordInterner,
        config: Self::Config,
    ) -> Result<(), SummaryOverflow> {
        let (mnc, modulus, ng_n, len_h, bs_n) = config;

        // Match upstream exactly: the distinguished zero self-loop tests only
        // whether x0 is empty.  In particular NG_n=0 intentionally leaves x0
        // empty forever, so later zero pushes are no-ops even if x1/x2 are live.
        if self.x0.is_empty() && words.get(w).is_zero() {
            return Ok(());
        }

        let ng_n = usize::from(ng_n);
        if ng_n != 0 {
            if self.x0.len() == ng_n {
                let _ = self.x0.pop();
            }
            self.x0.insert(0, w);
        } else {
            self.x0.truncate(0);
        }
        let current_ngram = self.x0;

        if self.x2.len() == usize::from(bs_n) {
            // `removelast (y::x2)` / `last (y::x2) y`: prepend the current
            // n-gram, keep exactly bs_n staging entries, and promote the oldest.
            self.x2.insert(0, current_ngram);
            let promoted = self.x2.pop().expect(
                "RNGS_mod staging queue must contain current n-gram",
            );
            self.promote_ngram(promoted, mnc, modulus, len_h);
        } else {
            debug_assert!(self.x2.len() < usize::from(bs_n));
            self.x2.insert(0, current_ngram);
        }

        Ok(())
    }

    fn may_be_all_zero_context(&self, words: &WordInterner) -> bool {
        self.x0.iter().all(|&wid| words.get(wid).is_zero())
            && self
                .x2
                .iter()
                .all(|ngram| Self::ngram_is_all_zero(ngram, words))
            && self
                .x1
                .iter()
                .all(|rw| Self::ngram_is_all_zero(&rw.w, words))
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct RsModSummary {
    q: Vec<RepeatWord>,
    q0: InlineVec<WordId, FAR_RS_NG_N>,
}

impl Summary for RsModSummary {
    type Config = ();

    fn new(_config: ()) -> Self {
        Self {
            q: Vec::new(),
            q0: InlineVec::new(),
        }
    }

    fn push(
        &mut self,
        w: WordId,
        words: &mut WordInterner,
        _config: (),
    ) -> Result<(), SummaryOverflow> {
        if self.q0.is_empty() && words.get(w).is_zero() {
            return Ok(());
        }

        let key = if self.q0.len() == FAR_RS_NG_N {
            let key = self.q0.remove(0);
            self.q0.push(w);
            key
        } else {
            self.q0.push(w);
            return Ok(());
        };

        promote_repeat_word(
            &mut self.q,
            key,
            FAR_RS_LEN_H,
            FAR_RS_MNC,
            FAR_RS_STRICT,
        )
    }

    fn may_be_all_zero_context(&self, words: &WordInterner) -> bool {
        self.q0.iter().all(|&w| words.get(w).is_zero())
            && self.q.iter().all(|rw| words.get(rw.w).is_zero())
    }
}

fn promote_repeat_word(
    q: &mut Vec<RepeatWord>,
    key: WordId,
    len_h: usize,
    mnc: u8,
    strict: bool,
) -> Result<(), SummaryOverflow> {
    for i in 0..q.len() {
        if q[i].w == key {
            let mut rep = q.remove(i);
            rep.n = rep.n.saturating_add(1).min(mnc);
            q.push(rep);
            return Ok(());
        }
    }

    q.push(RepeatWord::new(key, 1));
    if q.len() > len_h {
        if strict {
            return Err(SummaryOverflow);
        }
        q.remove(0);
    }
    Ok(())
}

/// FAR DFA state.  For Blank, `blank_all_zero` is an exact product-state
/// component for the regular language of stacks whose every represented cell is
/// semantically blank.  Keeping it inside the DFA state prevents a lossy history
/// summary from merging a zero-only stack with a nonzero stack and then borrowing
/// the zero-context witness from the other representative.  Other goals keep the
/// bit false so their DFA state space and behavior are unchanged.
#[derive(Clone, Eq, PartialEq, Hash)]
struct DfaSummaryState<S: Summary> {
    summary: S,
    blank_all_zero: bool,
}

type DfaId = u32;

// H3 is the hottest/widest relation key.  FAR's configured per-run work cap is
// at most 12_500 * 256 = 3_200_000, so a 24-bit DFA id has ample headroom.
// WordId remains a full u32.  Packing also preserves the old derived ordering:
// H2/H2b fit exactly in u32 as (state:8, dfa:24). H3 stores that same
// suffix below a full u32 WordId. Numeric order therefore preserves the old
// derived ordering: H2/H2b compare (state, dfa), H3 compares (word, state, dfa),
// and DfaEdge
// compares (word, predecessor).
const FAR_PACKED_DFA_ID_MAX: DfaId = 0x00ff_ffff;

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct H2(u32);

impl H2 {
    #[inline]
    const fn new(s: State, r: DfaId) -> Self {
        Self(((s as u32) << 24) | r)
    }

    #[inline]
    const fn s(self) -> State {
        (self.0 >> 24) as State
    }

    #[inline]
    const fn r(self) -> DfaId {
        self.0 & FAR_PACKED_DFA_ID_MAX
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct H2b(u32);

impl H2b {
    #[inline]
    const fn new(s: State, r: DfaId) -> Self {
        Self(((s as u32) << 24) | r)
    }

    #[inline]
    const fn s(self) -> State {
        (self.0 >> 24) as State
    }

    #[inline]
    const fn r(self) -> DfaId {
        self.0 & FAR_PACKED_DFA_ID_MAX
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct H3(u64);

impl H3 {
    #[inline]
    const fn new(w: WordId, s: State, r: DfaId) -> Self {
        Self(((w.0 as u64) << 32) | ((s as u64) << 24) | r as u64)
    }

    #[inline]
    const fn w(self) -> WordId {
        WordId((self.0 >> 32) as u32)
    }

    #[inline]
    const fn s(self) -> State {
        ((self.0 >> 24) & 0xff) as State
    }

    #[inline]
    #[expect(clippy::cast_possible_truncation)]
    const fn r(self) -> DfaId {
        (self.0 & FAR_PACKED_DFA_ID_MAX as u64) as DfaId
    }
}

/// Dense id for an interned H3 node.  H3 itself is eight bytes and appears in
/// several mutually recursive FAR relations; carrying this four-byte id through
/// those relations cuts both hash-table width and propagation traffic.
#[derive(
    Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug, Hash, Default,
)]
struct H3Id(u32);

impl H3Id {
    #[inline]
    const fn idx(self) -> usize {
        self.0 as usize
    }
}

/// Intern H3 nodes once and retain the historical LIFO todo order.  H3 is
/// already a packed u64, so its bits are the exact interner key; relation tables
/// carry only H3Id afterwards.
#[derive(Clone, Debug)]
struct H3Interner {
    ids: Map<u64, H3Id>,
    values: Vec<H3>,
    todo: Vec<H3Id>,
}

impl H3Interner {
    fn new() -> Self {
        Self {
            ids: Map::new(),
            values: Vec::new(),
            todo: Vec::new(),
        }
    }

    fn intern(&mut self, value: H3) -> H3Id {
        if let Some(&id) = self.ids.get(&value.0) {
            return id;
        }

        let id = H3Id(
            u32::try_from(self.values.len())
                .expect("FAR H3 interner exceeded u32::MAX entries"),
        );
        self.values.push(value);
        self.ids.insert(value.0, id);
        self.todo.push(id);
        id
    }

    #[inline]
    fn get(&self, id: H3Id) -> H3 {
        self.values[id.idx()]
    }

    fn pop_todo(&mut self) -> Option<H3Id> {
        self.todo.pop()
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct DfaEdge(u64);

impl DfaEdge {
    #[inline]
    const fn new(w: WordId, prev: DfaId) -> Self {
        Self(((w.0 as u64) << 32) | prev as u64)
    }

    #[inline]
    const fn w(self) -> WordId {
        WordId((self.0 >> 32) as u32)
    }

    #[inline]
    #[expect(clippy::cast_possible_truncation)]
    const fn prev(self) -> DfaId {
        self.0 as DfaId
    }
}

#[inline]
const fn dfa_push_key(w: WordId, from: DfaId) -> u64 {
    ((w.0 as u64) << 32) | from as u64
}

/// Compact sorted fan-out for FAR relations. Most relation keys have only one
/// target, so keep that first value inline and allocate a Vec only on the second
/// distinct target. Multi-target values stay sorted, which also removes the
/// repeated collect+sort work from relation propagation.
#[derive(Clone, Debug)]
enum RelationValues<V> {
    Empty,
    One(V),
    Many(Vec<V>),
}

impl<V: Copy> RelationValues<V> {
    fn insert_by(
        &mut self,
        value: V,
        mut cmp: impl FnMut(&V, &V) -> Ordering,
    ) -> bool {
        match self {
            Self::Empty => {
                *self = Self::One(value);
                true
            },
            Self::One(old) => match cmp(&value, old) {
                Ordering::Equal => false,
                Ordering::Less => {
                    let old = *old;
                    *self = Self::Many(vec![value, old]);
                    true
                },
                Ordering::Greater => {
                    let old = *old;
                    *self = Self::Many(vec![old, value]);
                    true
                },
            },
            Self::Many(values) => {
                match values.binary_search_by(|old| cmp(old, &value)) {
                    Ok(_) => false,
                    Err(pos) => {
                        values.insert(pos, value);
                        true
                    },
                }
            },
        }
    }

    fn as_slice(&self) -> &[V] {
        match self {
            Self::Empty => &[],
            Self::One(value) => core::slice::from_ref(value),
            Self::Many(values) => values,
        }
    }
}

impl<V: Ord + Copy> RelationValues<V> {
    fn insert(&mut self, value: V) -> bool {
        self.insert_by(value, Ord::cmp)
    }
}

/// A map K -> compact sorted set(V) with a todo stack of inserted pairs.
#[derive(Clone, Debug)]
struct TodoMap<K: Eq + Hash + Copy, V: Ord + Copy> {
    mp: Map<K, RelationValues<V>>,
    todo: Vec<(K, V)>,
}

impl<K: Eq + Hash + Copy, V: Ord + Copy> TodoMap<K, V> {
    fn new() -> Self {
        Self {
            mp: Map::new(),
            todo: Vec::new(),
        }
    }

    fn insert(&mut self, k: K, v: V) -> bool {
        let values = self.mp.entry(k).or_insert(RelationValues::Empty);
        if values.insert(v) {
            self.todo.push((k, v));
            true
        } else {
            false
        }
    }

    fn insert_by(
        &mut self,
        k: K,
        v: V,
        cmp: impl FnMut(&V, &V) -> Ordering,
    ) -> bool {
        let values = self.mp.entry(k).or_insert(RelationValues::Empty);
        if values.insert_by(v, cmp) {
            self.todo.push((k, v));
            true
        } else {
            false
        }
    }

    fn values<'a>(&'a self, k: &K) -> impl Iterator<Item = &'a V> {
        self.mp
            .get(k)
            .map(RelationValues::as_slice)
            .into_iter()
            .flatten()
    }

    fn pop_todo(&mut self) -> Option<(K, V)> {
        self.todo.pop()
    }
}

/// Dense set keyed by H3Id with a LIFO todo stack.  Membership costs one bit per
/// interned H3 node instead of another H3 hash-table entry.
#[derive(Clone, Debug)]
struct DenseH3TodoSet {
    bits: Vec<u64>,
    todo: Vec<H3Id>,
}

impl DenseH3TodoSet {
    const fn new() -> Self {
        Self {
            bits: Vec::new(),
            todo: Vec::new(),
        }
    }

    fn insert(&mut self, id: H3Id) -> bool {
        let idx = id.idx();
        let word = idx / u64::BITS as usize;
        let bit = 1_u64 << (idx % u64::BITS as usize);
        if self.bits.len() <= word {
            self.bits.resize(word + 1, 0);
        }
        if self.bits[word] & bit != 0 {
            return false;
        }
        self.bits[word] |= bit;
        self.todo.push(id);
        true
    }

    fn contains(&self, id: H3Id) -> bool {
        let idx = id.idx();
        let word = idx / u64::BITS as usize;
        let bit = 1_u64 << (idx % u64::BITS as usize);
        self.bits.get(word).is_some_and(|&bits| bits & bit != 0)
    }

    fn pop_todo(&mut self) -> Option<H3Id> {
        self.todo.pop()
    }
}

/// Dense H3-keyed relation.  H3Id gives direct row indexing, so ret3/pre32/pre33
/// no longer hash the same H3 key on every lookup and insertion.
#[derive(Clone, Debug)]
struct DenseH3TodoMap<V: Ord + Copy> {
    rows: Vec<RelationValues<V>>,
    todo: Vec<(H3Id, V)>,
}

impl<V: Ord + Copy> DenseH3TodoMap<V> {
    const fn new() -> Self {
        Self {
            rows: Vec::new(),
            todo: Vec::new(),
        }
    }

    fn ensure(&mut self, id: H3Id) {
        if self.rows.len() <= id.idx() {
            self.rows
                .resize_with(id.idx() + 1, || RelationValues::Empty);
        }
    }

    fn insert(&mut self, id: H3Id, value: V) -> bool {
        self.ensure(id);
        if self.rows[id.idx()].insert(value) {
            self.todo.push((id, value));
            true
        } else {
            false
        }
    }

    fn insert_by(
        &mut self,
        id: H3Id,
        value: V,
        cmp: impl FnMut(&V, &V) -> Ordering,
    ) -> bool {
        self.ensure(id);
        if self.rows[id.idx()].insert_by(value, cmp) {
            self.todo.push((id, value));
            true
        } else {
            false
        }
    }

    fn values(&self, id: H3Id) -> &[V] {
        match self.rows.get(id.idx()) {
            Some(values) => values.as_slice(),
            None => &[],
        }
    }

    fn pop_todo(&mut self) -> Option<(H3Id, V)> {
        self.todo.pop()
    }
}

#[derive(Clone, Debug)]
enum StopReason {
    WorkLimit,
    BlockTimeout,
    MayTarget,
    SummaryOverflow,
}

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug, Default)]
enum StepContext {
    #[default]
    Normal,
    Blank {
        context_may_be_all_zero: bool,
    },
    Spinout {
        back_context_may_be_all_zero: bool,
        forward_context_may_be_all_zero: bool,
    },
}

impl StepContext {
    #[inline]
    const fn packed_code(self) -> u8 {
        match self {
            Self::Normal => 0,
            Self::Blank {
                context_may_be_all_zero: false,
            } => 1,
            Self::Blank {
                context_may_be_all_zero: true,
            } => 2,
            Self::Spinout {
                back_context_may_be_all_zero: false,
                forward_context_may_be_all_zero: false,
            } => 3,
            Self::Spinout {
                back_context_may_be_all_zero: false,
                forward_context_may_be_all_zero: true,
            } => 4,
            Self::Spinout {
                back_context_may_be_all_zero: true,
                forward_context_may_be_all_zero: false,
            } => 5,
            Self::Spinout {
                back_context_may_be_all_zero: true,
                forward_context_may_be_all_zero: true,
            } => 6,
        }
    }

    const fn blank(context_may_be_all_zero: bool) -> Self {
        Self::Blank {
            context_may_be_all_zero,
        }
    }

    const fn spinout(
        back_context_may_be_all_zero: bool,
        forward_context_may_be_all_zero: bool,
    ) -> Self {
        Self::Spinout {
            back_context_may_be_all_zero,
            forward_context_may_be_all_zero,
        }
    }
}

#[derive(Clone, Copy)]
struct ReachedParams {
    states: usize,
    colors: usize,
}

#[derive(Clone, Copy)]
struct FarRunParams {
    block_len: usize,
    max_work: u32,
    block_step_limit: u16,
    goal: Goal,
    mirrored: bool,
    defer_blank_targets: bool,
}

#[derive(Clone, Copy)]
struct DirectFarParams {
    goal: Goal,
    direction: u8,
    reached: ReachedParams,
    dfa_states: usize,
    ctrl_states: usize,
    nfa_states: usize,
    any_sink: usize,
    zero_sink: usize,
}

#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
struct StepKey(u64);

impl StepKey {
    #[inline]
    #[expect(clippy::similar_names)]
    fn new(w: WordId, s: State, sgn: i8, ctx: StepContext) -> Self {
        debug_assert!(sgn == 1 || sgn == -1);
        let sign = u64::from(sgn < 0);
        Self(
            u64::from(w.0)
                | (u64::from(s) << 32)
                | (sign << 40)
                | (u64::from(ctx.packed_code()) << 41),
        )
    }
}

/// Exact machine-state membership per DFA state.  Busy Beaver-sized machines
/// use a two-byte inline mask per DFA state; unusual state numbers >=16 use a
/// sparse sorted overflow row without penalizing the common case.
#[derive(Clone, Debug, Default)]
struct DfaStateRows {
    low: Vec<u16>,
    high: Map<DfaId, Vec<State>>,
}

impl DfaStateRows {
    fn ensure(&mut self, r: DfaId) {
        let r = r as usize;
        if self.low.len() <= r {
            self.low.resize(r + 1, 0);
        }
    }

    #[expect(clippy::disallowed_names)]
    fn insert(&mut self, r: DfaId, state: State) -> bool {
        let state_idx = usize::from(state);
        if state_idx < FAR_DENSE_MACHINE_STATES {
            self.ensure(r);
            let bit = 1_u16 << state_idx;
            let row = &mut self.low[r as usize];
            let fresh = *row & bit == 0;
            *row |= bit;
            fresh
        } else {
            let row = self.high.entry(r).or_default();
            match row.binary_search(&state) {
                Ok(_) => false,
                Err(pos) => {
                    row.insert(pos, state);
                    true
                },
            }
        }
    }

    fn low_bits(&self, r: DfaId) -> u16 {
        self.low.get(r as usize).copied().unwrap_or(0)
    }

    #[expect(clippy::map_unwrap_or)]
    fn high_states(&self, r: DfaId) -> &[State] {
        self.high.get(&r).map(Vec::as_slice).unwrap_or(&[])
    }
}

trait DfaStateKey: Copy + Eq + Hash {
    fn state(self) -> State;
    fn dfa(self) -> DfaId;
}

impl DfaStateKey for H2 {
    #[inline]
    fn state(self) -> State {
        self.s()
    }
    #[inline]
    fn dfa(self) -> DfaId {
        self.r()
    }
}
impl DfaStateKey for H2b {
    #[inline]
    fn state(self) -> State {
        self.s()
    }
    #[inline]
    fn dfa(self) -> DfaId {
        self.r()
    }
}

const FAR_DENSE_MACHINE_STATES: usize = 16;

/// Dense membership for ordinary Busy Beaver-sized state sets.  Machines with
/// state numbers >=16 transparently fall back to hashing, preserving the public
/// generality without paying a 256-bit mask for every DFA state.
#[derive(Clone, Debug)]
struct DenseDfaTodoSet<K: DfaStateKey> {
    st: Vec<u16>,
    overflow: Set<K>,
    todo: Vec<K>,
}

impl<K: DfaStateKey> DenseDfaTodoSet<K> {
    fn new() -> Self {
        Self {
            st: Vec::new(),
            overflow: Set::new(),
            todo: Vec::new(),
        }
    }

    fn insert(&mut self, key: K) -> bool {
        let state = usize::from(key.state());
        let fresh = if state < FAR_DENSE_MACHINE_STATES {
            let r = key.dfa() as usize;
            if self.st.len() <= r {
                self.st.resize(r + 1, 0);
            }
            let bit = 1_u16 << state;
            let fresh = self.st[r] & bit == 0;
            self.st[r] |= bit;
            fresh
        } else {
            self.overflow.insert(key)
        };
        if fresh {
            self.todo.push(key);
        }
        fresh
    }

    fn contains(&self, key: &K) -> bool {
        let state = usize::from(key.state());
        if state < FAR_DENSE_MACHINE_STATES {
            self.st
                .get(key.dfa() as usize)
                .is_some_and(|&bits| bits & (1_u16 << state) != 0)
        } else {
            self.overflow.contains(key)
        }
    }

    fn pop_todo(&mut self) -> Option<K> {
        self.todo.pop()
    }
}

#[derive(Clone, Debug, Default)]
struct DenseH2Set {
    st: Vec<u16>,
    overflow: Set<H2>,
}

impl DenseH2Set {
    fn insert(&mut self, key: H2) -> bool {
        let state = usize::from(key.s());
        if state < FAR_DENSE_MACHINE_STATES {
            let r = key.r() as usize;
            if self.st.len() <= r {
                self.st.resize(r + 1, 0);
            }
            let bit = 1_u16 << state;
            let fresh = self.st[r] & bit == 0;
            self.st[r] |= bit;
            fresh
        } else {
            self.overflow.insert(key)
        }
    }

    fn any(&self, mut f: impl FnMut(H2) -> bool) -> bool {
        for (r, &bits) in self.st.iter().enumerate() {
            let mut bits = bits;
            while bits != 0 {
                let state = u8::try_from(bits.trailing_zeros())
                    .expect("dense FAR state bit must fit State");
                let r = u32::try_from(r)
                    .expect("packed DFA id must fit u32");
                if f(H2::new(state, r)) {
                    return true;
                }
                bits &= bits - 1;
            }
        }
        self.overflow.iter().copied().any(f)
    }
}

/// FAR decider with a pluggable DFA history summary.
struct FarDecider<'a, P: GetInstr, S: Summary> {
    prog: &'a P,
    goal: Goal,
    mirrored: bool,
    defer_blank_targets: bool,
    max_work: u32,
    block_step_limit: u16,
    summary_config: S::Config,

    // Interned FAR block words.  Relation keys, DFA edges, push cache keys,
    // and step-cache keys carry compact WordId values instead of cloning Vecs.
    words: WordInterner,
    // Canonical all-zero FAR block, interned once per run.
    blank_word: WordId,

    // work counter
    work: u32,

    // Cache exact block simulations by local FAR context.
    step_cache: Map<StepKey, WordUpdateOutcome>,
    // Separate cache for the conditional Blank pass. These simulations keep
    // running after a local blanking event so the ordinary return closure is
    // still saturated while the blank event is propagated as B2/B3 facts.
    deferred_blank_step_cache: Map<StepKey, WordUpdateOutcome>,

    // DFA.  Blank uses the product of the configured history summary with an
    // exact two-state all-semantically-blank automaton; other goals keep the
    // product bit false.  Full states live only in `idr`: the hash table stores
    // compact fingerprints plus ids, with exact collision checks against `idr`.
    id: Map<u64, DfaId>,
    id_collisions: Map<u64, Vec<DfaId>>,
    idr: Vec<DfaSummaryState<S>>,

    pop: Vec<Vec<DfaEdge>>,
    push: Map<u64, DfaId>,
    new_pops: Vec<(DfaId, DfaEdge)>,

    // relations
    ret2: TodoMap<H2, H2b>,
    ret3: DenseH3TodoMap<H2b>,
    pre23: TodoMap<H2, H3Id>,
    pre32: DenseH3TodoMap<H2>,
    pre33: DenseH3TodoMap<H3Id>,

    pre3l: DenseH3TodoSet,
    retl: DenseDfaTodoSet<H2b>,
    h3s: H3Interner,
    h2s: DenseDfaTodoSet<H2>,

    // Conditional Blank outcomes.
    // B2(a): a computation from H2 `a` can blank provided the whole tape to
    // the left of that boundary is blank.
    // B3(c): a computation from H3 `c` can blank provided the tape strictly
    // left of c.w is blank. pre32/pre33 transport this condition across blocks
    // that the subcomputation may modify or erase.
    blank2: DenseDfaTodoSet<H2>,
    blank3: DenseH3TodoSet,

    // Spinout witnesses found in h2_pop that depend on the tape side
    // forgotten by the H3 -> H2 projection. They are validated only after
    // relation saturation against the H3 generators recorded in `pre23`.
    pending_h2_spinout_targets: DenseH2Set,

    // For each DFA state r, which machine states have H2(s,r).
    r_s: DfaStateRows,

    // Legacy exact existential canonical-zero reachability used by Spinout.
    // Blank does not use this side table: its semantic-zero provenance is part
    // of `DfaSummaryState` itself, so it cannot be recombined across a lossy
    // summary merge.
    zero_context: Vec<bool>,
    zero_push: Vec<Vec<DfaId>>,

    // Reusable buffers for relation propagation. These avoid allocating short
    // temporary Vecs just to break immutable borrows before mutating self.
    scratch_states: Vec<State>,
    scratch_edges: Vec<DfaEdge>,
    scratch_h2: Vec<H2>,
    scratch_h2b: Vec<H2b>,
    scratch_h3: Vec<H3Id>,
}

impl<P: GetInstr, S: Summary> FarDecider<'_, P, S> {
    fn with_init_dfa(
        mut self,
        initial_summary: S,
    ) -> Result<Self, StopReason> {
        self.ensure_dfa_capacity(0);

        let blank_product = self.goal.is_blank();
        let id0 = self.get_id(initial_summary, blank_product);
        debug_assert_eq!(id0, 1);
        if !blank_product {
            self.mark_zero_context(id0);
        }

        let id1 = self.dfa_push_id(self.blank_word, id0)?;
        debug_assert_eq!(id1, id0);

        Ok(self)
    }

    const fn bump(&mut self) -> Result<(), StopReason> {
        if self.work >= self.max_work {
            return Err(StopReason::WorkLimit);
        }
        self.work += 1;
        Ok(())
    }

    fn ensure_dfa_capacity(&mut self, id: DfaId) {
        let idx = id as usize;
        if self.pop.len() <= idx {
            self.pop.resize_with(idx + 1, Vec::new);
        }
        self.r_s.ensure(id);
        if self.zero_context.len() <= idx {
            self.zero_context.resize(idx + 1, false);
        }
        if self.zero_push.len() <= idx {
            self.zero_push.resize_with(idx + 1, Vec::new);
        }
    }

    fn mark_zero_context(&mut self, start: DfaId) {
        let mut todo = vec![start];
        while let Some(r) = todo.pop() {
            self.ensure_dfa_capacity(r);
            if self.zero_context[r as usize] {
                continue;
            }
            self.zero_context[r as usize] = true;
            todo.extend(self.zero_push[r as usize].iter().copied());
        }
    }

    /// For Blank on a tape macro, decorated macro colors may still represent
    /// a semantically blank base-tape cell. Spinout deliberately requires the
    /// canonical zero ray, so only literal color 0 counts there.
    fn word_is_zero_context(&self, wid: WordId) -> bool {
        let word = self.words.get(wid);
        match self.goal {
            Goal::Blank => word
                .cells
                .iter()
                .all(|&color| self.prog.is_blank(color)),
            Goal::Halt | Goal::Spinout => word.is_zero(),
        }
    }

    fn get_id(&mut self, summary: S, blank_all_zero: bool) -> DfaId {
        let st = DfaSummaryState {
            summary,
            blank_all_zero: self.goal.is_blank() && blank_all_zero,
        };
        // Reuse this map's persistent aHash seed when fingerprinting the full
        // state.  The resulting u64 is only an index: exact equality against
        // `idr` below remains authoritative.
        let fingerprint = { self.id.hasher().hash_one(&st) };

        if let Some(&id) = self.id.get(&fingerprint) {
            if self.idr[id as usize] == st {
                return id;
            }
            if let Some(collisions) =
                self.id_collisions.get(&fingerprint)
            {
                for &candidate in collisions {
                    if self.idr[candidate as usize] == st {
                        return candidate;
                    }
                }
            }

            let id = u32::try_from(self.idr.len())
                .expect("FAR DFA exceeded u32::MAX states");
            assert!(
                id <= FAR_PACKED_DFA_ID_MAX,
                "FAR DFA exceeded packed 24-bit relation-key range",
            );
            self.idr.push(st);
            self.id_collisions.entry(fingerprint).or_default().push(id);
            self.ensure_dfa_capacity(id);
            return id;
        }

        let id = u32::try_from(self.idr.len())
            .expect("FAR DFA exceeded u32::MAX states");
        assert!(
            id <= FAR_PACKED_DFA_ID_MAX,
            "FAR DFA exceeded packed 24-bit relation-key range",
        );
        self.idr.push(st);
        self.id.insert(fingerprint, id);
        self.ensure_dfa_capacity(id);
        id
    }

    fn dfa_push_id(
        &mut self,
        wid: WordId,
        ls: DfaId,
    ) -> Result<DfaId, StopReason> {
        let key = dfa_push_key(wid, ls);
        if let Some(&to) = self.push.get(&key) {
            return Ok(to);
        }

        // For Blank this is the exact product transition for the all-blank
        // language.  The predecessor's provenance and the *semantic* blankness
        // of the pushed block travel through the same transition as the history
        // summary, so an LRU merge cannot detach one from the other.
        let blank_all_zero = self.goal.is_blank()
            && self.idr[ls as usize].blank_all_zero
            && self.word_is_zero_context(wid);

        let mut summary = self.idr[ls as usize].summary.clone();
        summary
            .push(wid, &mut self.words, self.summary_config)
            .map_err(|_| StopReason::SummaryOverflow)?;
        let to = self.get_id(summary, blank_all_zero);
        self.push.insert(key, to);
        self.new_pops.push((to, DfaEdge::new(wid, ls)));

        // Spinout still uses canonical-zero existential reachability.  Blank's
        // semantic-zero property is already exact in the product DFA state.
        if !self.goal.is_blank() && self.word_is_zero_context(wid) {
            self.ensure_dfa_capacity(ls.max(to));
            if !self.zero_push[ls as usize].contains(&to) {
                self.zero_push[ls as usize].push(to);
            }
            if self.zero_context[ls as usize] {
                self.mark_zero_context(to);
            }
        }

        Ok(to)
    }

    fn summary_may_be_all_zero_context(&self, r: DfaId) -> bool {
        if self.goal.is_blank() {
            return self
                .idr
                .get(r as usize)
                .is_some_and(|st| st.blank_all_zero);
        }

        let exact =
            self.zero_context.get(r as usize).copied().unwrap_or(false);
        if exact {
            debug_assert!(self.idr.get(r as usize).is_some_and(|st| {
                st.summary.may_be_all_zero_context(&self.words)
            }));
        }
        exact
    }

    fn h2_pop_step_context(&self, r0: DfaId) -> StepContext {
        match self.goal {
            Goal::Halt => StepContext::Normal,
            Goal::Blank => StepContext::blank(
                self.summary_may_be_all_zero_context(r0),
            ),
            Goal::Spinout => StepContext::spinout(
                true,
                self.summary_may_be_all_zero_context(r0),
            ),
        }
    }

    fn h3_back_step_context(&self, c: &H3, b: &H2b) -> StepContext {
        match self.goal {
            Goal::Halt => StepContext::Normal,
            Goal::Blank => {
                let c_context_zero =
                    self.summary_may_be_all_zero_context(c.r());
                let b_context_zero =
                    self.summary_may_be_all_zero_context(b.r());
                StepContext::blank(c_context_zero && b_context_zero)
            },
            Goal::Spinout => {
                let c_context_zero =
                    self.summary_may_be_all_zero_context(c.r());
                let b_context_zero =
                    self.summary_may_be_all_zero_context(b.r());
                StepContext::spinout(b_context_zero, c_context_zero)
            },
        }
    }

    fn retl_step_context(&self, r0: DfaId) -> StepContext {
        match self.goal {
            Goal::Halt => StepContext::Normal,
            Goal::Blank => StepContext::blank(
                self.summary_may_be_all_zero_context(r0),
            ),
            Goal::Spinout => StepContext::spinout(
                self.summary_may_be_all_zero_context(r0),
                true,
            ),
        }
    }

    fn insert_h3(&mut self, c: H3) -> H3Id {
        self.h3s.intern(c)
    }

    fn insert_h2(&mut self, a: H2) -> H2 {
        self.h2s.insert(a);
        a
    }

    fn insert_pre3l(&mut self, c: H3Id) -> H3Id {
        self.pre3l.insert(c);
        c
    }

    fn insert_retl(&mut self, b: H2b) -> H2b {
        self.retl.insert(b);
        b
    }

    fn insert_ret2(&mut self, a: H2, b: H2b) {
        self.ret2.insert(a, b);
    }

    fn insert_ret3(&mut self, a: H3Id, b: H2b) {
        self.ret3.insert(a, b);
    }

    fn insert_blank2(&mut self, a: H2) {
        if self.defer_blank_targets {
            self.blank2.insert(a);
        }
    }

    fn insert_blank3(&mut self, c: H3Id) {
        if self.defer_blank_targets {
            self.blank3.insert(c);
        }
    }

    fn intern_raw_word_update_outcome(
        &mut self,
        outcome: RawWordUpdateOutcome,
    ) -> WordUpdateOutcome {
        match outcome {
            RawWordUpdateOutcome::Exit(raw) => {
                WordUpdateOutcome::Exit(WordUpdateLemma {
                    w1: self.words.intern(raw.w1),
                    s1: raw.s1,
                    is_back: raw.is_back,
                    hit_blank: raw.hit_blank,
                })
            },
            RawWordUpdateOutcome::LocalLoop { hit_blank } => {
                WordUpdateOutcome::LocalLoop { hit_blank }
            },
            RawWordUpdateOutcome::Incomplete => {
                WordUpdateOutcome::Incomplete
            },
        }
    }

    fn tm_step_outcome(
        &mut self,
        w: WordId,
        s: State,
        sgn: i8,
        ctx: StepContext,
        target_mode: BlockTargetMode,
    ) -> Result<WordUpdateOutcome, StopReason> {
        self.bump()?;

        let key = StepKey::new(w, s, sgn, ctx);
        let cached = match target_mode {
            BlockTargetMode::Immediate(_) => {
                self.step_cache.get(&key).copied()
            },
            BlockTargetMode::DeferredBlank => {
                self.deferred_blank_step_cache.get(&key).copied()
            },
        };
        if let Some(outcome) = cached {
            return Ok(outcome);
        }

        let raw = far_raw_word_update_lemma(
            self.prog,
            self.words.clone_word(w),
            s,
            sgn,
            self.block_step_limit,
            target_mode,
            ctx,
            self.mirrored,
        );
        let outcome = self.intern_raw_word_update_outcome(raw);

        match target_mode {
            BlockTargetMode::Immediate(_) => {
                self.step_cache.insert(key, outcome);
            },
            BlockTargetMode::DeferredBlank => {
                self.deferred_blank_step_cache.insert(key, outcome);
            },
        }

        Ok(outcome)
    }

    /// Conditional-Blank local simulation. Unlike `tm_step`, a local blank
    /// event is remembered but does not terminate the block simulation; this is
    /// necessary to keep the return relations closed even when the event's
    /// missing left context has not yet been discharged.
    fn tm_step_deferred_blank(
        &mut self,
        w: WordId,
        s: State,
        sgn: i8,
        ctx: StepContext,
    ) -> Result<(Option<WordUpdateLemma>, bool), StopReason> {
        debug_assert!(self.goal.is_blank());
        debug_assert!(self.defer_blank_targets);

        match self.tm_step_outcome(
            w,
            s,
            sgn,
            ctx,
            BlockTargetMode::DeferredBlank,
        )? {
            WordUpdateOutcome::Exit(res) => {
                let hit_blank = res.hit_blank;
                if res.s1.is_none() {
                    Ok((None, hit_blank))
                } else {
                    Ok((Some(res), hit_blank))
                }
            },
            WordUpdateOutcome::LocalLoop { hit_blank } => {
                Ok((None, hit_blank))
            },
            WordUpdateOutcome::Incomplete => {
                Err(StopReason::BlockTimeout)
            },
        }
    }

    fn tm_step(
        &mut self,
        w: WordId,
        s: State,
        sgn: i8,
        ctx: StepContext,
    ) -> Result<Option<WordUpdateLemma>, StopReason> {
        let outcome = self.tm_step_outcome(
            w,
            s,
            sgn,
            ctx,
            BlockTargetMode::Immediate(self.goal),
        )?;

        let res = match outcome {
            WordUpdateOutcome::Exit(res) => res,
            WordUpdateOutcome::LocalLoop { .. } => return Ok(None),
            WordUpdateOutcome::Incomplete => {
                return Err(StopReason::BlockTimeout);
            },
        };

        if res.hit_blank {
            return Err(StopReason::MayTarget);
        }

        if res.s1.is_none() {
            return match self.goal {
                Goal::Halt => Err(StopReason::MayTarget),
                Goal::Blank | Goal::Spinout => Ok(None),
            };
        }

        Ok(Some(res))
    }

    fn on_h2_pop(
        &mut self,
        a: &H2,
        b: &DfaEdge,
    ) -> Result<(), StopReason> {
        let a = *a;
        let s = a.s();
        let r0 = b.prev();

        let res = if self.defer_blank_targets && self.goal.is_blank() {
            // Inside an H2 subcomputation the right remainder `r0` is explicit,
            // while the whole left side was forgotten by H3 -> H2. Record a
            // local blank as B2(a) when the explicit right side may be zero;
            // the missing left-side condition is propagated later.
            let ctx = StepContext::blank(
                self.summary_may_be_all_zero_context(r0),
            );
            let (step, hit_blank) =
                self.tm_step_deferred_blank(b.w(), s, 1, ctx)?;
            if hit_blank {
                self.insert_blank2(a);
            }
            step
        } else {
            let ctx = self.h2_pop_step_context(r0);
            let first = self.tm_step(b.w(), s, 1, ctx);

            // H2 is the projection of an H3 node and therefore forgets the H3
            // block on one side of the head. For Spinout only, the ordinary H2
            // context historically substitutes `back_zero = true` for that side.
            // If a spinout target disappears when only that assumed-zero side is
            // disabled, defer the witness until the H3 generators of this H2 fact
            // are known. A target supported by the explicit forward/r0 side still
            // fails immediately.
            let step = match first {
                Err(StopReason::MayTarget)
                    if self.goal.is_spinout() =>
                {
                    let forward_zero =
                        self.summary_may_be_all_zero_context(r0);
                    match self.tm_step(
                        b.w(),
                        s,
                        1,
                        StepContext::spinout(false, forward_zero),
                    ) {
                        Err(StopReason::MayTarget) => {
                            return Err(StopReason::MayTarget);
                        },
                        other => {
                            self.pending_h2_spinout_targets.insert(a);
                            other
                        },
                    }
                },
                other => other,
            };
            step?
        };

        let Some(res) = res else {
            return Ok(());
        };
        let s1 = res.s1.unwrap();

        if res.is_back {
            let rr = self.dfa_push_id(res.w1, r0)?;
            self.insert_ret2(a, H2b::new(s1, rr));
        } else {
            let c = self.insert_h3(H3::new(res.w1, s1, r0));
            self.pre32.insert(c, a);
        }

        Ok(())
    }

    fn on_h3_back(
        &mut self,
        c_id: H3Id,
        b: &H2b,
    ) -> Result<(), StopReason> {
        let c = self.h3s.get(c_id);
        let b = *b;
        let s0 = b.s();
        let r0 = b.r();

        let res = if self.defer_blank_targets && self.goal.is_blank() {
            // `b.r` is the current right-hand stack after the H2 return. The
            // context strictly left of c.w is the conditional part of B3(c), so
            // do not require the original c.r to have been zero: the intervening
            // subcomputation may have modified or erased that old context.
            let ctx = StepContext::blank(
                self.summary_may_be_all_zero_context(b.r()),
            );
            let (step, hit_blank) =
                self.tm_step_deferred_blank(c.w(), s0, -1, ctx)?;
            if hit_blank {
                self.insert_blank3(c_id);
            }
            step
        } else {
            self.tm_step(
                c.w(),
                s0,
                -1,
                self.h3_back_step_context(&c, &b),
            )?
        };

        let Some(res) = res else {
            return Ok(());
        };
        let s1 = res.s1.unwrap();

        if res.is_back {
            let c0 = self.insert_h3(H3::new(res.w1, s1, r0));
            let h3s = &self.h3s;
            self.pre33.insert_by(c0, c_id, |a, b| {
                h3s.get(*a).cmp(&h3s.get(*b))
            });
        } else {
            let rr = self.dfa_push_id(res.w1, r0)?;
            self.insert_ret3(c_id, H2b::new(s1, rr));
        }

        Ok(())
    }

    fn on_retl(&mut self, b: &H2b) -> Result<(), StopReason> {
        let b = *b;
        let s0 = b.s();
        let r0 = b.r();

        let blank = self.blank_word;
        let res = if self.defer_blank_targets && self.goal.is_blank() {
            // At the left frontier the missing outer context is the concrete
            // infinite blank ray, so a conditional local event is fully
            // discharged as soon as the current right stack may be all zero.
            let ctx = StepContext::blank(
                self.summary_may_be_all_zero_context(r0),
            );
            let (step, hit_blank) =
                self.tm_step_deferred_blank(blank, s0, -1, ctx)?;
            if hit_blank {
                return Err(StopReason::MayTarget);
            }
            step
        } else {
            self.tm_step(blank, s0, -1, self.retl_step_context(r0))?
        };

        let Some(res) = res else {
            return Ok(());
        };
        let s1 = res.s1.unwrap();

        if res.is_back {
            let c0 = self.insert_h3(H3::new(res.w1, s1, r0));
            self.insert_pre3l(c0);
        } else {
            let rr = self.dfa_push_id(res.w1, r0)?;
            self.insert_retl(H2b::new(s1, rr));
        }

        Ok(())
    }

    fn on_dfa_edge(
        &mut self,
        r: DfaId,
        e: &DfaEdge,
    ) -> Result<(), StopReason> {
        self.bump()?;
        self.ensure_dfa_capacity(r);
        #[expect(clippy::disallowed_names)]
        let row = &mut self.pop[r as usize];
        let pos = row.binary_search(e).unwrap_or_else(|pos| pos);
        row.insert(pos, *e);

        // Preserve descending machine-state order.  High state numbers are rare
        // and copied only on the sparse fallback path.
        self.scratch_states.clear();
        self.scratch_states
            .extend_from_slice(self.r_s.high_states(r));
        while let Some(s) = self.scratch_states.pop() {
            self.on_h2_pop(&H2::new(s, r), e)?;
        }

        let mut states = self.r_s.low_bits(r);
        while states != 0 {
            let bit = u16::BITS - 1 - states.leading_zeros();
            #[expect(clippy::unwrap_in_result)]
            let s = u8::try_from(bit)
                .expect("low FAR state bit must fit State");
            states &= !(1_u16 << bit);
            self.on_h2_pop(&H2::new(s, r), e)?;
        }

        Ok(())
    }

    fn on_h2(&mut self, a: &H2) -> Result<(), StopReason> {
        let a = *a;
        let s = a.s();
        let r = a.r();

        self.bump()?;
        self.ensure_dfa_capacity(r);
        self.r_s.insert(r, s);

        self.scratch_edges.clear();
        self.scratch_edges
            .extend(self.pop[r as usize].iter().copied());
        while let Some(e) = self.scratch_edges.pop() {
            self.on_h2_pop(&a, &e)?;
        }

        Ok(())
    }

    fn on_h3(&mut self, a_id: H3Id) {
        let a = self.h3s.get(a_id);
        let a0 = self.insert_h2(H2::new(a.s(), a.r()));
        let h3s = &self.h3s;
        self.pre23
            .insert_by(a0, a_id, |x, y| h3s.get(*x).cmp(&h3s.get(*y)));
    }

    fn on_ret2(&mut self, a: &H2, b: &H2b) -> Result<(), StopReason> {
        self.scratch_h3.clear();
        self.scratch_h3.extend(self.pre23.values(a).copied());
        while let Some(c_id) = self.scratch_h3.pop() {
            self.on_h3_back(c_id, b)?;
        }
        Ok(())
    }

    fn on_ret3(&mut self, a: H3Id, b: &H2b) {
        self.scratch_h2.clear();
        self.scratch_h2.extend(self.pre32.values(a).iter().copied());
        while let Some(a0) = self.scratch_h2.pop() {
            self.insert_ret2(a0, *b);
        }

        self.scratch_h3.clear();
        self.scratch_h3.extend(self.pre33.values(a).iter().copied());
        while let Some(a1) = self.scratch_h3.pop() {
            self.insert_ret3(a1, *b);
        }

        if self.pre3l.contains(a) {
            self.insert_retl(*b);
        }
    }

    fn on_blank2(&mut self, a: &H2) {
        self.scratch_h3.clear();
        self.scratch_h3.extend(self.pre23.values(a).copied());
        while let Some(c_id) = self.scratch_h3.pop() {
            // B2 requires the whole left side of the H2 boundary to be blank.
            // In H3 that side is c.w plus the still-conditional farther-left
            // context, so an already blank c.w converts B2(a) into B3(c).
            let c = self.h3s.get(c_id);
            if self.word_is_zero_context(c.w()) {
                self.insert_blank3(c_id);
            }
        }
    }

    fn on_blank3(&mut self, c: H3Id) -> Result<(), StopReason> {
        // pre3l means the context strictly left of c.w is the concrete initial
        // blank ray, so B3(c)'s remaining condition is discharged.
        if self.pre3l.contains(c) {
            return Err(StopReason::MayTarget);
        }

        self.scratch_h2.clear();
        self.scratch_h2.extend(self.pre32.values(c).iter().copied());
        while let Some(a0) = self.scratch_h2.pop() {
            // Crossing a block forward turns the old H2 left context into the
            // farther-left context of the resulting H3. The condition is
            // unchanged when transported back through pre32.
            self.insert_blank2(a0);
        }

        self.scratch_h3.clear();
        self.scratch_h3.extend(self.pre33.values(c).iter().copied());
        while let Some(c0) = self.scratch_h3.pop() {
            // pre33 represents an excursion that may modify c.w and return to
            // the same boundary. Only the farther-left context is unchanged,
            // exactly the condition carried by B3.
            self.insert_blank3(c0);
        }

        Ok(())
    }

    fn on_pre23(
        &mut self,
        a: &H2,
        c_id: H3Id,
    ) -> Result<(), StopReason> {
        self.scratch_h2b.clear();
        self.scratch_h2b.extend(self.ret2.values(a).copied());
        while let Some(b) = self.scratch_h2b.pop() {
            self.on_h3_back(c_id, &b)?;
        }

        let c = self.h3s.get(c_id);
        if self.defer_blank_targets
            && self.blank2.contains(a)
            && self.word_is_zero_context(c.w())
        {
            self.insert_blank3(c_id);
        }
        Ok(())
    }

    fn on_pre32(&mut self, a: H3Id, a0: &H2) {
        self.scratch_h2b.clear();
        self.scratch_h2b.extend(self.ret3.values(a).iter().copied());
        while let Some(b) = self.scratch_h2b.pop() {
            self.insert_ret2(*a0, b);
        }

        if self.defer_blank_targets && self.blank3.contains(a) {
            self.insert_blank2(*a0);
        }
    }

    fn on_pre33(&mut self, a: H3Id, a0: H3Id) {
        self.scratch_h2b.clear();
        self.scratch_h2b.extend(self.ret3.values(a).iter().copied());
        while let Some(b) = self.scratch_h2b.pop() {
            self.insert_ret3(a0, b);
        }

        if self.defer_blank_targets && self.blank3.contains(a) {
            self.insert_blank3(a0);
        }
    }

    fn on_pre3l(&mut self, a: H3Id) -> Result<(), StopReason> {
        self.scratch_h2b.clear();
        self.scratch_h2b.extend(self.ret3.values(a).iter().copied());
        while let Some(b) = self.scratch_h2b.pop() {
            self.insert_retl(b);
        }

        if self.defer_blank_targets && self.blank3.contains(a) {
            return Err(StopReason::MayTarget);
        }
        Ok(())
    }

    /// A deferred H2 Spinout witness remains possible iff its H2 node has
    /// some H3 generator whose forgotten side may really be all zero: the
    /// adjacent H3 block is zero and the summary beyond it is reachable from
    /// the initial summary using only zero blocks.
    ///
    /// Every H2 fact is introduced only by `on_h3`, which simultaneously adds
    /// the corresponding `pre23` edge. Therefore `pre23.values(a)` is the full
    /// set of H3 generators for that H2 fact. The check is delayed until the
    /// relation fixed point because more generators may be discovered later.
    fn pending_h2_spinout_target_still_possible(&self) -> bool {
        self.pending_h2_spinout_targets.any(|a| {
            self.pre23.values(&a).any(|&c_id| {
                let c = self.h3s.get(c_id);
                self.words.get(c.w()).is_zero()
                    && self.summary_may_be_all_zero_context(c.r())
            })
        })
    }

    fn run(mut self) -> Result<(), StopReason> {
        let blank = self.blank_word;
        let c0 = H3::new(blank, 0, 1);
        let c0 = self.insert_h3(c0);
        self.insert_pre3l(c0);

        loop {
            if let Some((r, e)) = self.new_pops.pop() {
                self.on_dfa_edge(r, &e)?;
                continue;
            }
            if let Some(a) = self.h3s.pop_todo() {
                self.on_h3(a);
                continue;
            }
            if let Some(a) = self.h2s.pop_todo() {
                self.on_h2(&a)?;
                continue;
            }
            if let Some(b) = self.retl.pop_todo() {
                self.on_retl(&b)?;
                continue;
            }
            if let Some(a) = self.pre3l.pop_todo() {
                self.on_pre3l(a)?;
                continue;
            }
            if let Some(a) = self.blank2.pop_todo() {
                self.on_blank2(&a);
                continue;
            }
            if let Some(c) = self.blank3.pop_todo() {
                self.on_blank3(c)?;
                continue;
            }
            if let Some((a, b)) = self.ret2.pop_todo() {
                self.on_ret2(&a, &b)?;
                continue;
            }
            if let Some((a, b)) = self.ret3.pop_todo() {
                self.on_ret3(a, &b);
                continue;
            }
            if let Some((a, c)) = self.pre23.pop_todo() {
                self.on_pre23(&a, c)?;
                continue;
            }
            if let Some((a, a0)) = self.pre32.pop_todo() {
                self.on_pre32(a, &a0);
                continue;
            }
            if let Some((a, a0)) = self.pre33.pop_todo() {
                self.on_pre33(a, a0);
                continue;
            }
            break;
        }

        if self.pending_h2_spinout_target_still_possible() {
            return Err(StopReason::MayTarget);
        }

        Ok(())
    }
}

/// Exact one-block simulation shared by ordinary FAR and the conditional-Blank
/// closure, over either the raw TM or a finite-history macro through `GetInstr`.
///
/// In `Immediate` mode the first Blank/Spinout target ends the simulation, as
/// ordinary FAR requires.  `DeferredBlank` instead records a blanking event in
/// `hit_blank` and continues to the same exit/local-loop boundary, so the B2/B3
/// conditional closure can saturate its ordinary return relations as well.
///
/// A macro instruction lookup may fail with `Err`; as in standalone CPS this
/// makes the proof attempt inconclusive rather than turning the macro error into
/// a halting transition.
#[expect(clippy::cast_sign_loss)]
fn far_raw_word_update_lemma<P: GetInstr>(
    prog: &P,
    w: Word,
    s: State,
    sgn: i8,
    max_steps: u16,
    target_mode: BlockTargetMode,
    ctx: StepContext,
    mirrored: bool,
) -> RawWordUpdateOutcome {
    debug_assert!(sgn == 1 || sgn == -1);

    let goal = target_mode.goal();
    let defer_blank_targets = target_mode.defers_blank();
    debug_assert!(!defer_blank_targets || goal.is_blank());

    let context_may_be_all_zero = match ctx {
        StepContext::Blank {
            context_may_be_all_zero,
        } => context_may_be_all_zero,
        _ => false,
    };

    let len = i16::try_from(w.len())
        .expect("FAR block length must fit in i16");
    let mut w1 = w;
    let mut nonblank_count = match goal {
        Goal::Blank => w1
            .cells
            .iter()
            .filter(|&&color| !prog.is_blank(color))
            .count(),
        Goal::Halt | Goal::Spinout => {
            w1.cells.iter().filter(|&&color| color != 0).count()
        },
    };
    let mut s1 = s;
    let mut pos: i16 = 0;
    let mut hit_blank = false;

    let mut local_words = Map::new();
    local_words.insert(w1.clone(), 0_u16);
    let mut local_word_id = 0_u16;
    let mut seen = Set::new();
    let mut steps = 0_u16;

    loop {
        // This is the hottest exact-simulation set key.  The local word id is
        // bounded by max_steps <= 51_200, State is u8, and a live in-block
        // position is 0..=255, so the full configuration fits in one u32.
        debug_assert!((0..=i16::from(u8::MAX)).contains(&pos));
        let seen_key = u32::from(local_word_id)
            | (u32::from(s1) << 16)
            | ((pos as u32) << 24);
        if !seen.insert(seen_key) {
            return RawWordUpdateOutcome::LocalLoop { hit_blank };
        }
        if steps == max_steps {
            return RawWordUpdateOutcome::Incomplete;
        }
        steps += 1;

        let input = w1.get(pos as usize);
        let (out_color, shift_right, next_state) =
            match prog.get_instr(&(s1, input)) {
                Err(_) => return RawWordUpdateOutcome::Incomplete,
                Ok(None) => {
                    return RawWordUpdateOutcome::Exit(
                        RawWordUpdateLemma::exit_oriented(
                            w1, None, false, hit_blank,
                        ),
                    );
                },
                Ok(Some(instr)) => instr,
            };

        let dir: i16 = if shift_right { 1 } else { -1 };
        let dir = if mirrored { -dir } else { dir };
        let block_dir = dir * i16::from(sgn);

        // Spinout is specifically a same-state transition while scanning the
        // canonical macro zero, with a canonical-zero ray ahead. Deferred mode
        // is Blank-only, so Spinout targets are always immediate.
        if goal.is_spinout() && input == 0 && next_state == s1 {
            let zero_ray_ahead = match ctx {
                StepContext::Spinout {
                    back_context_may_be_all_zero,
                    forward_context_may_be_all_zero,
                } => {
                    if block_dir > 0 {
                        w1.zero_to_right_of(pos as usize)
                            && forward_context_may_be_all_zero
                    } else {
                        w1.zero_to_left_of(pos as usize)
                            && back_context_may_be_all_zero
                    }
                },
                _ => false,
            };

            if zero_ray_ahead {
                return RawWordUpdateOutcome::Exit(
                    RawWordUpdateLemma::exit_oriented(
                        w1,
                        Some(s1),
                        false,
                        true,
                    ),
                );
            }
        }

        let input_goal_nonblank = match goal {
            Goal::Blank => !prog.is_blank(input),
            Goal::Halt | Goal::Spinout => input != 0,
        };
        let output_goal_nonblank = match goal {
            Goal::Blank => !prog.is_blank(out_color),
            Goal::Halt | Goal::Spinout => out_color != 0,
        };
        let erased_final_nonblank = goal.is_blank()
            && input_goal_nonblank
            && !output_goal_nonblank;

        if input != out_color {
            if input_goal_nonblank {
                nonblank_count -= 1;
            }
            if output_goal_nonblank {
                nonblank_count += 1;
            }
            w1.set(pos as usize, out_color);

            local_word_id = if let Some(&id) = local_words.get(&w1) {
                id
            } else {
                let id = u16::try_from(local_words.len()).expect(
                    "FAR local word table exceeded u16::MAX entries",
                );
                local_words.insert(w1.clone(), id);
                id
            };
        }
        s1 = next_state;

        if erased_final_nonblank
            && nonblank_count == 0
            && context_may_be_all_zero
        {
            if defer_blank_targets {
                hit_blank = true;
            } else {
                return RawWordUpdateOutcome::Exit(
                    RawWordUpdateLemma::exit_oriented(
                        w1,
                        Some(s1),
                        false,
                        true,
                    ),
                );
            }
        }

        pos += block_dir;
        if pos < 0 || pos >= len {
            return RawWordUpdateOutcome::Exit(
                RawWordUpdateLemma::exit_oriented(
                    w1,
                    Some(s1),
                    pos < 0,
                    hit_blank,
                ),
            );
        }
    }
}

/// Build one summary-FAR decider over any raw or macro machine implementing
/// the same instruction interface used by standalone CPS.
fn far_decider_for_with_summary<P: GetInstr, S: Summary>(
    prog: &P,
    params: FarRunParams,
    summary_config: S::Config,
) -> Result<FarDecider<'_, P, S>, StopReason> {
    let initial_summary = S::new(summary_config);
    // Index 0 remains the historical unused sentinel.  It deliberately carries
    // `blank_all_zero = false`; the real initial product state is interned by
    // `with_init_dfa` at index 1.
    let idr = vec![DfaSummaryState {
        summary: initial_summary.clone(),
        blank_all_zero: false,
    }];
    let mut words = WordInterner::new();
    let blank_word = words.intern(Word::zero(params.block_len));
    let decider = FarDecider {
        prog,
        goal: params.goal,
        mirrored: params.mirrored,
        defer_blank_targets: params.defer_blank_targets,
        max_work: params.max_work,
        block_step_limit: params.block_step_limit,
        summary_config,
        words,
        blank_word,
        work: 0,
        step_cache: Map::new(),
        deferred_blank_step_cache: Map::new(),
        id: Map::new(),
        id_collisions: Map::new(),
        idr,
        pop: Vec::new(),
        push: Map::new(),
        new_pops: Vec::new(),
        ret2: TodoMap::new(),
        ret3: DenseH3TodoMap::new(),
        pre23: TodoMap::new(),
        pre32: DenseH3TodoMap::new(),
        pre33: DenseH3TodoMap::new(),
        pre3l: DenseH3TodoSet::new(),
        retl: DenseDfaTodoSet::new(),
        h3s: H3Interner::new(),
        h2s: DenseDfaTodoSet::new(),
        blank2: DenseDfaTodoSet::new(),
        blank3: DenseH3TodoSet::new(),
        pending_h2_spinout_targets: DenseH2Set::default(),
        r_s: DfaStateRows::default(),
        zero_context: Vec::new(),
        zero_push: Vec::new(),
        scratch_states: Vec::new(),
        scratch_edges: Vec::new(),
        scratch_h2: Vec::new(),
        scratch_h2b: Vec::new(),
        scratch_h3: Vec::new(),
    };
    decider.with_init_dfa(initial_summary)
}

fn far_decide_with_config<P: GetInstr, S: Summary>(
    prog: &P,
    params: FarRunParams,
    summary_config: S::Config,
) -> bool {
    let Ok(decider) = far_decider_for_with_summary::<P, S>(
        prog,
        params,
        summary_config,
    ) else {
        return false;
    };
    decider.run().is_ok()
}

fn far_decide_with_for<P: GetInstr, S: Summary<Config = ()>>(
    prog: &P,
    params: FarRunParams,
) -> bool {
    far_decide_with_config::<P, S>(prog, params, ())
}

fn far_decide_rwl_mod_profile<P: GetInstr>(
    prog: &P,
    params: FarRunParams,
    profile: (u8, u8, u8, u8),
) -> bool {
    far_decide_with_config::<P, RwlModSummary>(prog, params, profile)
}

/// Cheap RWL_mod profiles retained in the hot raw-summary portfolio.  These are
/// exactly the old keep-newest-8 behavior, now expressed through BusyCoq's
/// general `(mnc, mod_, len1, len2)` rule.
fn far_decide_rwl_mod_default_portfolio<P: GetInstr>(
    prog: &P,
    params: FarRunParams,
) -> bool {
    FAR_RWL_DEFAULT_MODS.iter().copied().any(|modulus| {
        far_decide_rwl_mod_profile(
            prog,
            params,
            (
                FAR_RWL_DEFAULT_MNC,
                modulus,
                FAR_RWL_DEFAULT_LEN1,
                FAR_RWL_DEFAULT_LEN2,
            ),
        )
    })
}

fn far_decide_rngs_mod_profile<P: GetInstr>(
    prog: &P,
    params: FarRunParams,
    profile: (u8, u8, u8, u8, u8),
) -> bool {
    far_decide_with_config::<P, RngsModSummary>(prog, params, profile)
}

fn far_decide_rngs_mod_default<P: GetInstr>(
    prog: &P,
    params: FarRunParams,
) -> bool {
    far_decide_rngs_mod_profile(
        prog,
        params,
        (
            FAR_RNGS_DEFAULT_MNC,
            FAR_RNGS_DEFAULT_MOD,
            FAR_RNGS_DEFAULT_NG_N,
            FAR_RNGS_DEFAULT_LEN_H,
            FAR_RNGS_DEFAULT_BS_N,
        ),
    )
}

fn far_decide_upstream_cps_lru(
    prog: &impl GetInstr,
    params: FarRunParams,
    profile: (u8, u8, u8),
    lru_n: u8,
) -> bool {
    let (len1, len2, len3) = profile;
    far_decide_with_config::<_, UpstreamCpsLruSummary>(
        prog,
        params,
        (len1, len2, len3, lru_n),
    )
}

/// Full raw summary portfolio. The extra CPS experiments that produced no new
/// holdout proofs were removed; keep only ordinary CPS-LRU, signature CPS-LRU,
/// and the full-color signature variant that did add a Halt proof.
fn far_decide_summary_portfolio<P: GetInstr>(
    prog: &P,
    params: FarRunParams,
) -> bool {
    far_decide_with_for::<_, Ng1Summary>(prog, params)
        || far_decide_with_for::<
            _,
            NgSummary<FAR_NG_TAIL_H_SMALL, FAR_NG_POS_MOD_2>,
        >(prog, params)
        || far_decide_with_for::<_, CpsLruSummary>(prog, params)
        || far_decide_with_for::<_, CpsLruSigSummary>(prog, params)
        || far_decide_with_for::<_, CpsLruSigColorSummary>(prog, params)
        || far_decide_rwl_mod_default_portfolio(prog, params)
        || far_decide_with_for::<
            _,
            NgSummary<FAR_NG_TAIL_H_MED, FAR_NG_POS_MOD_3>,
        >(prog, params)
        || far_decide_with_for::<_, NgSetSummary>(prog, params)
        || far_decide_with_for::<_, LruPairSummary>(prog, params)
        || far_decide_with_for::<_, SetPairSummary>(prog, params)
        || far_decide_rngs_mod_default(prog, params)
        || far_decide_with_for::<_, RsModSummary>(prog, params)
}

#[expect(clippy::multiple_inherent_impl)]
impl<const STATES: usize, const COLORS: usize> Prog<STATES, COLORS> {
    fn far_reached_params(&self) -> ReachedParams {
        let (max_state, max_color) = self.max_reached();
        ReachedParams {
            states: max_state as usize + 1,
            colors: max_color as usize + 1,
        }
    }

    fn far_block_len_cap(&self) -> usize {
        let colors = self.far_reached_params().colors;
        if colors <= 2 {
            FAR_BLOCK_LEN_CAP_COLORS_2
        } else if colors <= 4 {
            FAR_BLOCK_LEN_CAP_COLORS_3_4
        } else {
            FAR_BLOCK_LEN_CAP_COLORS_5_8
        }
    }

    /// Run every summary-based FAR family block-major.  Small block lengths are
    /// exhausted across all summary families before any family is allowed to
    /// spend time on a larger block.  This preserves every distinct proof attempt
    /// while allowing late summaries to prove easy cases before raw FAR has swept
    /// all requested block sizes.
    #[expect(clippy::excessive_nesting)]
    fn far_summary_sweep(
        &self,
        block: usize,
        goal: Goal,
        defer_blank_targets: bool,
    ) -> bool {
        debug_assert!(!defer_blank_targets || goal.is_blank());

        let block = block
            .min(self.far_block_len_cap())
            .min(FAR_BLOCK_LEN_HARD_CAP);
        let history = self.make_lru_macro();

        for block_len in 1..=block {
            let params_base = FarRunParams {
                block_len,
                max_work: u32::try_from(FAR_WORK_PER_LEN * block_len)
                    .expect("FAR work budget must fit u32"),
                block_step_limit: u16::try_from(
                    FAR_STEP_PER_LEN * block_len,
                )
                .expect("FAR block-step budget must fit u16"),
                goal,
                mirrored: false,
                defer_blank_targets,
            };

            // Keep the cheap/hot raw portfolio ahead of every late family in
            // both orientations at this block size.
            for mirrored in [true, false] {
                let params = FarRunParams {
                    mirrored,
                    ..params_base
                };
                if far_decide_summary_portfolio(self, params) {
                    return true;
                }
            }

            // Exact BusyCoq CPS_LRU profiles.  The old experimental
            // `(LRUH,H,tH)` sweep was exactly the LRU_n=0 subset under
            // `(len1,len2,len3)=(H,LRUH,tH)`, so it is intentionally absent.
            if block_len <= FAR_CPS_LRU_EXACT_BLOCK_LEN_CAP {
                // LRU_n=0 first across all profiles: this is exactly the old
                // experimental transform portfolio and was historically useful.
                for (lru_idx, &lru_n) in
                    FAR_CPS_LRU_EXACT_LRU_NS.iter().enumerate()
                {
                    for &profile in FAR_CPS_LRU_EXACT_PROFILES {
                        let (_, len2, _) = profile;
                        // Only `len2` distinct duplicate-removal positions can
                        // affect a suffix of capacity len2.  All later LRU_n
                        // values are equivalent; keep one representative at 0
                        // when len2 itself is zero.
                        let distinct_lru_ns = usize::from(len2)
                            .max(1)
                            .min(FAR_CPS_LRU_EXACT_LRU_NS.len());
                        if lru_idx >= distinct_lru_ns {
                            continue;
                        }

                        for mirrored in [true, false] {
                            let params = FarRunParams {
                                mirrored,
                                ..params_base
                            };
                            if far_decide_upstream_cps_lru(
                                self, params, profile, lru_n,
                            ) {
                                return true;
                            }
                        }
                    }
                }
            }

            if block_len <= FAR_RWL_GENERAL_BLOCK_LEN_CAP {
                for &profile in FAR_RWL_GENERAL_PROFILES {
                    for mirrored in [true, false] {
                        let params = FarRunParams {
                            mirrored,
                            ..params_base
                        };
                        if far_decide_rwl_mod_profile(
                            self, params, profile,
                        ) {
                            return true;
                        }
                    }
                }
            }

            if block_len <= FAR_RNGS_GENERAL_BLOCK_LEN_CAP {
                for &profile in FAR_RNGS_GENERAL_PROFILES {
                    for mirrored in [true, false] {
                        let params = FarRunParams {
                            mirrored,
                            ..params_base
                        };
                        if far_decide_rngs_mod_profile(
                            self, params, profile,
                        ) {
                            return true;
                        }
                    }
                }
            }

            // Full per-cell finite history remains a distinct tape macro, but
            // shares the same summary portfolio and current block size.
            if block_len <= FAR_HISTORY_BLOCK_LEN_CAP {
                for mirrored in [true, false] {
                    let params = FarRunParams {
                        mirrored,
                        ..params_base
                    };
                    if far_decide_summary_portfolio(&history, params) {
                        return true;
                    }
                }
            }
        }

        false
    }

    fn far_cant_target(&self, block: usize, goal: Goal) -> bool {
        self.far_summary_sweep(block, goal, false)
            || self.mitm_cant_target(goal)
            || self.direct_far_cant_target(goal)
            || (goal.is_blank()
                && self.far_summary_sweep(block, Goal::Blank, true))
    }

    fn direct_far_cant_target(&self, goal: Goal) -> bool {
        let reached = self.far_reached_params();
        let ctrl_states = reached.states;

        let max_by_nfa = (DIRECT_FAR_MAX_NFA_STATES
            - DIRECT_FAR_TARGET_STATES)
            / ctrl_states;
        let max_by_entries =
            DIRECT_FAR_MAX_DFA_ENTRIES / reached.colors;
        let max_dfa_states = DIRECT_FAR_MAX_DFA_STATES
            .min(max_by_nfa)
            .min(max_by_entries);

        if max_dfa_states == 0 {
            return false;
        }

        let mut fuel = DIRECT_FAR_MAX_WORK;
        for dfa_states in 1..=max_dfa_states {
            if self.direct_far_decide_exact(
                goal, reached, dfa_states, &mut fuel,
            ) {
                return true;
            }
            if fuel == 0 {
                break;
            }
        }

        false
    }

    fn direct_far_decide_exact(
        &self,
        goal: Goal,
        reached: ReachedParams,
        dfa_states: usize,
        fuel: &mut usize,
    ) -> bool {
        let ctrl_states = reached.states;
        debug_assert!(
            ctrl_states * dfa_states + DIRECT_FAR_TARGET_STATES
                <= DIRECT_FAR_MAX_NFA_STATES
        );

        self.direct_far_decide_direction(
            goal, reached, dfa_states, 0, fuel,
        ) || self.direct_far_decide_direction(
            goal, reached, dfa_states, 1, fuel,
        )
    }

    fn direct_far_decide_direction(
        &self,
        goal: Goal,
        reached: ReachedParams,
        dfa_states: usize,
        direction: u8,
        fuel: &mut usize,
    ) -> bool {
        if *fuel == 0 {
            return false;
        }

        let ctrl_states = reached.states;
        let nfa_states =
            ctrl_states * dfa_states + DIRECT_FAR_TARGET_STATES;
        let any_sink = nfa_states - 2;
        let zero_sink = nfa_states - 1;
        let params = DirectFarParams {
            goal,
            direction,
            reached,
            dfa_states,
            ctrl_states,
            nfa_states,
            any_sink,
            zero_sink,
        };

        let mut r = vec![vec![0_u128; nfa_states]; reached.colors];
        let a = direct_far_bit(any_sink) | direct_far_bit(zero_sink);
        self.direct_far_init_targets(params, &mut r);

        let dfa_entries = reached.colors * dfa_states;
        let mut dfa = vec![0_usize; dfa_entries];
        debug_assert!(dfa_entries < u32::BITS as usize);
        debug_assert!(dfa_states <= 16);
        let deps = DirectFarDeps {
            rows: vec![vec![0; nfa_states]; reached.colors],
            accept: vec![0; nfa_states],
            reject: 0,
        };
        self.direct_far_search(
            params,
            &mut dfa,
            0,
            0,
            &r,
            a,
            &deps,
            0,
            &mut DirectFarRejectCache::default(),
            fuel,
        )
    }

    fn direct_far_init_targets(
        &self,
        params: DirectFarParams,
        r: &mut [Vec<u128>],
    ) {
        let any_bit = direct_far_bit(params.any_sink);
        let zero_bit = direct_far_bit(params.zero_sink);

        // `any_sink` accepts an arbitrary suffix; `zero_sink` accepts only an
        // all-zero suffix.  This lets the same direct-FAR machinery handle
        // local halt targets, global blank targets, and one-sided spinout rays.
        for color_matrix in r.iter_mut() {
            color_matrix[params.any_sink] |= any_bit;
        }
        r[0][params.zero_sink] |= zero_bit;

        match params.goal {
            Goal::Halt => {
                // Missing TM transitions are halting transitions.
                for state in 0..params.reached.states {
                    for read_symbol in 0..params.reached.colors {
                        #[expect(clippy::cast_possible_truncation)]
                        let slot: Slot =
                            (state as State, read_symbol as Color);
                        if self.get(&slot).is_none() {
                            for dfa_state in 0..params.dfa_states {
                                let src = direct_far_idx(
                                    dfa_state,
                                    state,
                                    params.ctrl_states,
                                );
                                r[read_symbol][src] |= any_bit;
                            }
                        }
                    }
                }
            },
            Goal::Blank => {
                // The first blank tape after time 0 is created by a transition
                // that reads a nonzero symbol and writes zero while every other
                // tape cell is already zero.  DFA state 0 denotes the all-zero
                // left context, and `zero_sink` requires an all-zero right ray.
                for state in 0..params.reached.states {
                    for read_symbol in 1..params.reached.colors {
                        #[expect(clippy::cast_possible_truncation)]
                        let slot: Slot =
                            (state as State, read_symbol as Color);
                        let Some(&(write, _, _)) = self.get(&slot)
                        else {
                            continue;
                        };
                        if write != 0 {
                            continue;
                        }

                        let src = direct_far_idx(
                            0,
                            state,
                            params.ctrl_states,
                        );
                        r[read_symbol][src] |= zero_bit;
                    }
                }
            },
            Goal::Spinout => {
                // Match the existing FAR/MITM spinout target: while scanning a
                // zero, a same-state transition with an all-zero ray ahead.
                for state in 0..params.reached.states {
                    #[expect(clippy::cast_possible_truncation)]
                    let slot: Slot = (state as State, 0);
                    let Some(&(_, shift_right, next_state)) =
                        self.get(&slot)
                    else {
                        continue;
                    };
                    if next_state as usize != state {
                        continue;
                    }

                    let move_code = direct_far_move_code(shift_right);
                    if move_code == params.direction {
                        // The forward ray is the NFA/right side in this
                        // orientation, so it must be all zero.
                        for dfa_state in 0..params.dfa_states {
                            let src = direct_far_idx(
                                dfa_state,
                                state,
                                params.ctrl_states,
                            );
                            r[0][src] |= zero_bit;
                        }
                    } else {
                        // The forward ray is the DFA/left side in this
                        // orientation; state 0 denotes the all-zero ray.  The
                        // opposite side is irrelevant.
                        let src = direct_far_idx(
                            0,
                            state,
                            params.ctrl_states,
                        );
                        r[0][src] |= any_bit;
                    }
                }
            },
        }
    }

    /// Exact validator for direct-FAR pruning and final acceptance.  `fixed`
    /// selects arbitrary DFA entries whose current values in `dfa` are available.
    /// It rebuilds the target NFA from scratch and computes the ordinary full
    /// left-rule fixed point, so neither dirty-row propagation nor dependency
    /// bookkeeping is proof-critical.
    fn direct_far_mask_rejected_exact(
        &self,
        params: DirectFarParams,
        dfa: &[usize],
        mut fixed: u32,
    ) -> bool {
        // Entry zero is canonical and fixed to zero in every enumerated DFA.
        // Rejection witnesses omit it so they can recur at later DFS prefixes.
        fixed |= 1;

        let mut r = vec![
            vec![0_u128; params.nfa_states];
            params.reached.colors
        ];
        let mut a = direct_far_bit(params.any_sink)
            | direct_far_bit(params.zero_sink);
        self.direct_far_init_targets(params, &mut r);

        // Right rules for exactly the selected DFA transitions.
        let mut entries = fixed;
        while entries != 0 {
            let entry = entries.trailing_zeros() as usize;
            entries &= entries - 1;
            debug_assert!(entry < dfa.len());

            let dfa_src = entry / params.reached.colors;
            let write_symbol = entry % params.reached.colors;
            let dfa_dst = dfa[entry];

            for ctrl in 0..params.ctrl_states {
                for read_symbol in 0..params.reached.colors {
                    #[expect(clippy::cast_possible_truncation)]
                    let slot: Slot =
                        (ctrl as State, read_symbol as Color);
                    let Some(&(write, shift_right, next_state)) =
                        self.get(&slot)
                    else {
                        continue;
                    };
                    if direct_far_move_code(shift_right)
                        != params.direction
                        || write as usize != write_symbol
                    {
                        continue;
                    }

                    let src = direct_far_idx(
                        dfa_src,
                        ctrl,
                        params.ctrl_states,
                    );
                    let dst = direct_far_idx(
                        dfa_dst,
                        next_state as usize,
                        params.ctrl_states,
                    );
                    r[read_symbol][src] |= direct_far_bit(dst);
                }
            }
        }

        // Full exact left-rule closure over the selected DFA transitions.
        loop {
            let mut changed = false;
            for ctrl in 0..params.ctrl_states {
                for read_symbol in 0..params.reached.colors {
                    #[expect(clippy::cast_possible_truncation)]
                    let slot: Slot =
                        (ctrl as State, read_symbol as Color);
                    let Some(&(write, shift_right, next_state)) =
                        self.get(&slot)
                    else {
                        continue;
                    };
                    if direct_far_move_code(shift_right)
                        == params.direction
                    {
                        continue;
                    }

                    let written = write as usize;
                    let next_ctrl = next_state as usize;
                    let mut entries = fixed;
                    while entries != 0 {
                        let fixed_entry =
                            entries.trailing_zeros() as usize;
                        entries &= entries - 1;

                        let fixed_src =
                            fixed_entry / params.reached.colors;
                        let fixed_symbol =
                            fixed_entry % params.reached.colors;
                        let fixed_dst = dfa[fixed_entry];
                        let middle = direct_far_idx(
                            fixed_src,
                            next_ctrl,
                            params.ctrl_states,
                        );
                        let src = direct_far_idx(
                            fixed_dst,
                            ctrl,
                            params.ctrl_states,
                        );
                        let inferred = direct_far_vec_times_matrix(
                            r[fixed_symbol][middle],
                            &r[written],
                        );
                        let new_bits = inferred & !r[read_symbol][src];
                        if new_bits != 0 {
                            r[read_symbol][src] |= inferred;
                            changed = true;
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }

        loop {
            let next_accept = direct_far_matrix_times_vec(
                &r[0],
                a,
                params.nfa_states,
            );
            if next_accept == a {
                break;
            }
            a = next_accept;
        }

        let start_idx = direct_far_idx(0, 0, params.ctrl_states);
        r[0][start_idx] & a != 0
    }

    #[expect(clippy::too_many_arguments)]
    fn direct_far_search(
        &self,
        params: DirectFarParams,
        dfa: &mut [usize],
        entry: usize,
        max_seen: usize,
        r: &[Vec<u128>],
        a: u128,
        deps: &DirectFarDeps,
        assignments: u128,
        rejects: &mut DirectFarRejectCache,
        fuel: &mut usize,
    ) -> bool {
        if *fuel == 0 {
            return false;
        }

        let dfa_entries = params.reached.colors * params.dfa_states;
        if entry == dfa_entries {
            if max_seen + 1 != params.dfa_states {
                return false;
            }
            let all_fixed = (1_u32 << dfa_entries) - 1;
            // Only the exact full-closure verifier is allowed to certify a DFA.
            return !self.direct_far_mask_rejected_exact(
                params, dfa, all_fixed,
            );
        }

        // Exact-state search: if even introducing one fresh state per remaining
        // transition cannot reach `dfa_states`, this branch cannot be canonical.
        let remaining = dfa_entries - entry;
        if max_seen + 1 + remaining < params.dfa_states {
            return false;
        }

        let max_to_state = if entry == 0 {
            0
        } else {
            (max_seen + 1).min(params.dfa_states - 1)
        };

        for to_state in 0..=max_to_state {
            if *fuel == 0 {
                return false;
            }
            *fuel -= 1;

            dfa[entry] = to_state;
            let next_assignments =
                assignments | ((to_state as u128) << (4 * entry));
            let fixed = (1_u32 << (entry + 1)) - 1;

            // Cached dependency witnesses are rejection-only hints. They can
            // prune this completion subtree but never certify a successful DFA.
            if rejects.rejects(next_assignments, fixed) {
                continue;
            }

            let mut next_r = r.to_vec();
            let mut next_a = a;
            let mut next_deps = deps.clone();
            if !self.direct_far_extend_nfa(
                params,
                dfa,
                &mut next_r,
                &mut next_a,
                entry,
                &mut next_deps,
            ) {
                // Rejection witnesses only prune search. Even if dependency
                // bookkeeping were overly conservative, it cannot certify a
                // non-target result; all successful leaves are exact-verified.
                rejects.remember(
                    next_deps.reject,
                    next_assignments,
                    fixed,
                );
                continue;
            }

            if self.direct_far_search(
                params,
                dfa,
                entry + 1,
                max_seen.max(to_state),
                &next_r,
                next_a,
                &next_deps,
                next_assignments,
                rejects,
                fuel,
            ) {
                return true;
            }
        }

        false
    }

    /// Fast direct-FAR propagation. Dirty-row scheduling and dependency tracking
    /// are search accelerators only: they may prune candidates, but they can never
    /// certify a proof. Any eventual success is rebuilt and checked by the exact
    /// complete-DFA validator in `direct_far_search`.
    #[expect(clippy::excessive_nesting)]
    fn direct_far_extend_nfa(
        &self,
        params: DirectFarParams,
        dfa: &[usize],
        r: &mut [Vec<u128>],
        a: &mut u128,
        entry: usize,
        deps: &mut DirectFarDeps,
    ) -> bool {
        let dfa_src = entry / params.reached.colors;
        let write_symbol = entry % params.reached.colors;
        let dfa_dst = dfa[entry];
        let fixed_entries = entry + 1;

        // Each color has at most DIRECT_FAR_MAX_DFA_ENTRIES NFA-relevant rows
        // under the public caps. The parent branch is already saturated, so only
        // newly changed rows need to seed this propagation round.
        let mut dirty_rows = [0_u128; DIRECT_FAR_MAX_DFA_ENTRIES];
        let mut dirty = &mut dirty_rows[..params.reached.colors];

        // Right-rule for the one newly fixed DFA transition.
        for ctrl in 0..params.ctrl_states {
            for read_symbol in 0..params.reached.colors {
                #[expect(clippy::cast_possible_truncation)]
                let slot: Slot = (ctrl as State, read_symbol as Color);
                let Some(&(write, shift_right, next_state)) =
                    self.get(&slot)
                else {
                    continue;
                };

                let written = write as usize;
                if direct_far_move_code(shift_right) == params.direction
                    && written == write_symbol
                {
                    let src = direct_far_idx(
                        dfa_src,
                        ctrl,
                        params.ctrl_states,
                    );
                    let dst = direct_far_idx(
                        dfa_dst,
                        next_state as usize,
                        params.ctrl_states,
                    );
                    let bit = direct_far_bit(dst);
                    if r[read_symbol][src] & bit == 0 {
                        r[read_symbol][src] |= bit;
                        deps.rows[read_symbol][src] |= 1_u32 << entry;
                        dirty[read_symbol] |= direct_far_bit(src);
                    }
                }
            }
        }

        if !direct_far_extend_accept(&r[0], a, params.nfa_states, deps)
        {
            return false;
        }

        // A left rule reads one row as a set of intermediate states, then unions
        // the corresponding rows of the written-symbol matrix. It can change only
        // if either input row changed since the parent branch was saturated.
        let mut next_dirty_rows = [0_u128; DIRECT_FAR_MAX_DFA_ENTRIES];
        let mut next_dirty =
            &mut next_dirty_rows[..params.reached.colors];
        let mut first_pass = true;
        loop {
            let mut changed = false;
            let mut zero_changed = false;
            next_dirty.fill(0);

            for ctrl in 0..params.ctrl_states {
                for read_symbol in 0..params.reached.colors {
                    #[expect(clippy::cast_possible_truncation)]
                    let slot: Slot =
                        (ctrl as State, read_symbol as Color);
                    let Some(&(write, shift_right, next_state)) =
                        self.get(&slot)
                    else {
                        continue;
                    };

                    let written = write as usize;
                    if direct_far_move_code(shift_right)
                        == params.direction
                    {
                        continue;
                    }

                    let next_ctrl = next_state as usize;
                    for fixed_entry in 0..fixed_entries {
                        let fixed_src =
                            fixed_entry / params.reached.colors;
                        let fixed_symbol =
                            fixed_entry % params.reached.colors;
                        let fixed_dst = dfa[fixed_entry];
                        let middle = direct_far_idx(
                            fixed_src,
                            next_ctrl,
                            params.ctrl_states,
                        );
                        let via = r[fixed_symbol][middle];
                        if !(first_pass && fixed_entry == entry)
                            && dirty[fixed_symbol]
                                & direct_far_bit(middle)
                                == 0
                            && via & dirty[written] == 0
                        {
                            continue;
                        }
                        let src = direct_far_idx(
                            fixed_dst,
                            ctrl,
                            params.ctrl_states,
                        );
                        let inferred = direct_far_vec_times_matrix(
                            via,
                            &r[written],
                        );
                        let new_bits = inferred & !r[read_symbol][src];
                        if new_bits != 0 {
                            let mut support = (1_u32 << fixed_entry)
                                | deps.rows[fixed_symbol][middle];
                            let mut via_support =
                                r[fixed_symbol][middle];
                            let mut uncovered = new_bits;
                            while uncovered != 0 && via_support != 0 {
                                let idx = via_support.trailing_zeros()
                                    as usize;
                                let contributed =
                                    r[written][idx] & uncovered;
                                if contributed != 0 {
                                    support |= deps.rows[written][idx];
                                    uncovered &= !contributed;
                                }
                                via_support &= via_support - 1;
                            }
                            deps.rows[read_symbol][src] |= support;
                            r[read_symbol][src] |= inferred;
                            let row_bit = direct_far_bit(src);
                            dirty[read_symbol] |= row_bit;
                            next_dirty[read_symbol] |= row_bit;
                            changed = true;
                            zero_changed |= read_symbol == 0;
                        }
                    }
                }
            }

            if zero_changed
                && !direct_far_extend_accept(
                    &r[0],
                    a,
                    params.nfa_states,
                    deps,
                )
            {
                return false;
            }

            if !changed {
                return true;
            }
            first_pass = false;
            core::mem::swap(&mut dirty, &mut next_dirty);
        }
    }

    fn mitm_cant_target(&self, goal: Goal) -> bool {
        let colors = self.far_reached_params().colors;

        // Enumerate every closed MITM-DFA skeleton once.  At each closed
        // skeleton, enumerate its weight assignments once and test the full
        // asymmetric finite-memory portfolio against each candidate.
        for dfa_transitions in 2..=MITM_MAX_TRANSITIONS {
            if self.mitm_decide_exact(
                goal,
                dfa_transitions,
                MITM_MAX_WEIGHT_PAIRS,
                colors,
            ) {
                return true;
            }
        }

        false
    }

    fn mitm_decide_exact(
        &self,
        goal: Goal,
        dfa_transitions: usize,
        max_weight_pairs: usize,
        colors: usize,
    ) -> bool {
        let mut left = MitmWfa::new(colors);
        let mut right = MitmWfa::new(colors);
        left.trans[0][0] = (0, 0);
        right.trans[0][0] = (0, 0);

        self.mitm_recurse_dfa(
            goal,
            &mut left,
            &mut right,
            2,
            MitmSearchParams {
                goal_transitions: dfa_transitions,
                max_weight_pairs,
            },
        )
    }

    fn mitm_recurse_dfa(
        &self,
        goal: Goal,
        left: &mut MitmWfa,
        right: &mut MitmWfa,
        current_transitions: usize,
        params: MitmSearchParams,
    ) -> bool {
        match self.mitm_find_closure_break(goal, left, right) {
            None => {
                current_transitions == params.goal_transitions
                    && self.mitm_recurse_weights(
                        goal,
                        left,
                        right,
                        &self.mitm_reachable_weight_slots(left, right),
                        0,
                        params.max_weight_pairs,
                        &mut vec![
                            MitmRejectCache::default();
                            MITM_MEMORY_PROFILES.len()
                        ],
                    )
            },
            Some((MitmSide::Left, state, color)) => {
                if current_transitions >= params.goal_transitions {
                    return false;
                }

                if left.states < params.goal_transitions {
                    let old = left.trans[state][color];
                    let new_state = left.push_dead_state();
                    left.trans[state][color] = (new_state, 0);
                    if self.mitm_recurse_dfa(
                        goal,
                        left,
                        right,
                        current_transitions + 1,
                        params,
                    ) {
                        left.trans[state][color] = old;
                        left.pop_state();
                        return true;
                    }
                    left.trans[state][color] = old;
                    left.pop_state();
                }

                let states = left.states;
                for to_state in 0..states {
                    if to_state == MITM_DEAD {
                        continue;
                    }
                    let old = left.trans[state][color];
                    left.trans[state][color] = (to_state, 0);
                    if self.mitm_recurse_dfa(
                        goal,
                        left,
                        right,
                        current_transitions + 1,
                        params,
                    ) {
                        left.trans[state][color] = old;
                        return true;
                    }
                    left.trans[state][color] = old;
                }

                false
            },
            Some((MitmSide::Right, state, color)) => {
                if current_transitions >= params.goal_transitions {
                    return false;
                }

                if right.states < params.goal_transitions {
                    let old = right.trans[state][color];
                    let new_state = right.push_dead_state();
                    right.trans[state][color] = (new_state, 0);
                    if self.mitm_recurse_dfa(
                        goal,
                        left,
                        right,
                        current_transitions + 1,
                        params,
                    ) {
                        right.trans[state][color] = old;
                        right.pop_state();
                        return true;
                    }
                    right.trans[state][color] = old;
                    right.pop_state();
                }

                let states = right.states;
                for to_state in 0..states {
                    if to_state == MITM_DEAD {
                        continue;
                    }
                    let old = right.trans[state][color];
                    right.trans[state][color] = (to_state, 0);
                    if self.mitm_recurse_dfa(
                        goal,
                        left,
                        right,
                        current_transitions + 1,
                        params,
                    ) {
                        right.trans[state][color] = old;
                        return true;
                    }
                    right.trans[state][color] = old;
                }

                false
            },
        }
    }

    fn mitm_find_closure_break(
        &self,
        goal: Goal,
        left: &MitmWfa,
        right: &MitmWfa,
    ) -> Option<(MitmSide, usize, usize)> {
        let left_rev = left.rev_edges();
        let right_rev = right.rev_edges();
        let start = MitmConfig::start();
        let mut seen = Set::new();
        let mut todo = vec![start];
        let mut nexts = Vec::new();
        seen.insert(start);

        while let Some(cur) = todo.pop() {
            let Some(&(write, ..)) = self.get(&(cur.st, cur.co)) else {
                continue;
            };
            let write = write as usize;

            nexts.clear();
            self.mitm_next_configs_into(
                goal, cur, left, right, &left_rev, &right_rev,
                &mut nexts,
            );
            #[expect(clippy::iter_with_drain)]
            for next in nexts.drain(..) {
                let cfg = next.config;
                if seen.contains(&cfg) {
                    continue;
                }
                if cfg.left == mitm_id(MITM_DEAD) {
                    return Some((
                        MitmSide::Left,
                        mitm_idx(cur.left),
                        write,
                    ));
                }
                if cfg.right == mitm_id(MITM_DEAD) {
                    return Some((
                        MitmSide::Right,
                        mitm_idx(cur.right),
                        write,
                    ));
                }
                seen.insert(cfg);
                todo.push(cfg);
            }
        }

        None
    }

    fn mitm_reachable_weight_slots(
        &self,
        left: &MitmWfa,
        right: &MitmWfa,
    ) -> MitmWeightSlots {
        let left_rev = left.rev_edges();
        let right_rev = right.rev_edges();
        let start = MitmConfig::start();

        let mut seen = Set::new();
        let mut todo = vec![start];
        let mut useful_left =
            vec![vec![false; left.colors]; left.states];
        let mut useful_right =
            vec![vec![false; right.colors]; right.states];
        seen.insert(start);

        while let Some(cur) = todo.pop() {
            let Some(&(write, shift, next_st)) =
                self.get(&(cur.st, cur.co))
            else {
                continue;
            };

            let write = write as usize;
            if shift {
                useful_left[mitm_idx(cur.left)][write] = true;
                let (new_left, _) =
                    left.trans[mitm_idx(cur.left)][write];
                for edge in &right_rev[mitm_idx(cur.right)] {
                    useful_right[mitm_idx(edge.from)]
                        [edge.symbol as usize] = true;
                    let next = MitmConfig {
                        st: next_st,
                        co: edge.symbol,
                        left: mitm_id(new_left),
                        right: edge.from,
                    };
                    if next.left != mitm_id(MITM_DEAD)
                        && next.right != mitm_id(MITM_DEAD)
                        && seen.insert(next)
                    {
                        todo.push(next);
                    }
                }
            } else {
                useful_right[mitm_idx(cur.right)][write] = true;
                let (new_right, _) =
                    right.trans[mitm_idx(cur.right)][write];
                for edge in &left_rev[mitm_idx(cur.left)] {
                    useful_left[mitm_idx(edge.from)]
                        [edge.symbol as usize] = true;
                    let next = MitmConfig {
                        st: next_st,
                        co: edge.symbol,
                        left: edge.from,
                        right: mitm_id(new_right),
                    };
                    if next.left != mitm_id(MITM_DEAD)
                        && next.right != mitm_id(MITM_DEAD)
                        && seen.insert(next)
                    {
                        todo.push(next);
                    }
                }
            }
        }

        let mut slots = MitmWeightSlots::default();
        for (state, colors) in useful_left.into_iter().enumerate() {
            for (color, useful) in colors.into_iter().enumerate() {
                if useful
                    && left.trans[state][color].0 != MITM_DEAD
                    && !(state == 0 && color == 0)
                {
                    slots.left.push((state, color));
                }
            }
        }
        for (state, colors) in useful_right.into_iter().enumerate() {
            for (color, useful) in colors.into_iter().enumerate() {
                if useful
                    && right.trans[state][color].0 != MITM_DEAD
                    && !(state == 0 && color == 0)
                {
                    slots.right.push((state, color));
                }
            }
        }
        slots
    }

    #[expect(clippy::similar_names)]
    fn mitm_recurse_weights(
        &self,
        goal: Goal,
        left: &mut MitmWfa,
        right: &mut MitmWfa,
        weight_slots: &MitmWeightSlots,
        current_weight_pairs: usize,
        max_weight_pairs: usize,
        reject_caches: &mut [MitmRejectCache],
    ) -> bool {
        if self.mitm_check_memory_profiles(
            goal,
            left,
            right,
            reject_caches,
        ) {
            return true;
        }

        if current_weight_pairs >= max_weight_pairs {
            return false;
        }

        let weight_pairs: &[(i32, i32)] = if current_weight_pairs == 0 {
            &[(1, -1)]
        } else {
            &[(1, -1), (-1, 1)]
        };

        for &(lw, rw) in weight_pairs {
            for &(ls, lc) in &weight_slots.left {
                let (lt, old_lw) = left.trans[ls][lc];
                left.trans[ls][lc] = (lt, old_lw + lw);

                for &(rs, rc) in &weight_slots.right {
                    let (rt, old_rw) = right.trans[rs][rc];
                    right.trans[rs][rc] = (rt, old_rw + rw);
                    if self.mitm_recurse_weights(
                        goal,
                        left,
                        right,
                        weight_slots,
                        current_weight_pairs + 1,
                        max_weight_pairs,
                        reject_caches,
                    ) {
                        right.trans[rs][rc] = (rt, old_rw);
                        left.trans[ls][lc] = (lt, old_lw);
                        return true;
                    }
                    right.trans[rs][rc] = (rt, old_rw);
                }

                left.trans[ls][lc] = (lt, old_lw);
            }
        }

        false
    }

    fn mitm_check_memory_profiles(
        &self,
        goal: Goal,
        left: &MitmWfa,
        right: &MitmWfa,
        reject_caches: &mut [MitmRejectCache],
    ) -> bool {
        debug_assert_eq!(
            reject_caches.len(),
            MITM_MEMORY_PROFILES.len()
        );

        // Build each one-sided expansion only once for this weight candidate.
        // Rejection paths persist across weight candidates, separately for each
        // memory profile, and are replayed exactly before full saturation.
        let mut left_memory = Vec::<MitmWfa>::new();
        let mut right_memory = Vec::<MitmWfa>::new();

        for (profile_idx, &memory) in
            MITM_MEMORY_PROFILES.iter().enumerate()
        {
            while left_memory.len() < memory.left {
                let next = left_memory.last().map_or_else(
                    || left.with_memory(),
                    MitmWfa::with_memory,
                );
                left_memory.push(next);
            }
            while right_memory.len() < memory.right {
                let next = right_memory.last().map_or_else(
                    || right.with_memory(),
                    MitmWfa::with_memory,
                );
                right_memory.push(next);
            }

            let try_left = if memory.left == 0 {
                left
            } else {
                &left_memory[memory.left - 1]
            };
            let try_right = if memory.right == 0 {
                right
            } else {
                &right_memory[memory.right - 1]
            };

            if self.mitm_check_weight_candidate_exact(
                goal,
                try_left,
                try_right,
                &mut reject_caches[profile_idx],
            ) {
                return true;
            }
        }

        false
    }

    fn mitm_check_weight_candidate_exact(
        &self,
        goal: Goal,
        left: &MitmWfa,
        right: &MitmWfa,
        reject_cache: &mut MitmRejectCache,
    ) -> bool {
        let left_special = left.derive_special();
        let right_special = right.derive_special();
        if !left.verify_leading_blank()
            || !right.verify_leading_blank()
            || !left.verify_special(&left_special)
            || !right.verify_special(&right_special)
        {
            return false;
        }

        // A cached path can only reject this candidate after exact replay under
        // this candidate's current transition weights and special-state bounds.
        if reject_cache.paths.iter().any(|path| {
            self.mitm_replay_reject_path(
                goal,
                path,
                left,
                right,
                &left_special,
                &right_special,
            )
        }) {
            return false;
        }

        let left_rev = left.rev_edges();
        let right_rev = right.rev_edges();
        self.mitm_build_accept_set_exact(
            goal,
            left,
            right,
            &left_rev,
            &right_rev,
            &left_special,
            &right_special,
            reject_cache,
        )
    }

    #[expect(clippy::too_many_arguments)]
    fn mitm_build_accept_set_exact(
        &self,
        goal: Goal,
        left: &MitmWfa,
        right: &MitmWfa,
        left_rev: &MitmRev,
        right_rev: &MitmRev,
        left_special: &MitmSpecial,
        right_special: &MitmSpecial,
        reject_cache: &mut MitmRejectCache,
    ) -> bool {
        let start = MitmConfig::start();
        let start_bounds = MitmBounds {
            lo: Some(0),
            hi: Some(0),
        };
        if !self.mitm_config_allowed(goal, &start) {
            return false;
        }

        let mut accept = MitmAccept::new();
        // First-discovery predecessors are only candidate rejection witnesses.
        // A path is stored only after exact no-join/no-widen replay validates it.
        let track_paths =
            reject_cache.paths.len() < MITM_MAX_REJECT_PATHS;
        let mut parents = Map::new();
        let mut todo = vec![start];
        let mut nexts = Vec::new();
        accept.insert(start, start_bounds);

        while let Some(cur) = todo.pop() {
            let cur_bounds = accept[&cur];
            nexts.clear();
            self.mitm_next_configs_into(
                goal, cur, left, right, left_rev, right_rev, &mut nexts,
            );
            nexts.sort_by_key(|next| next.config);

            #[expect(clippy::iter_with_drain)]
            for next in nexts.drain(..) {
                let Some(next) = mitm_step_bounds(
                    next,
                    cur_bounds,
                    left_special,
                    right_special,
                ) else {
                    continue;
                };

                if goal.is_blank()
                    && mitm_blank_transition_possible(&next)
                {
                    self.mitm_remember_reject_path(
                        goal,
                        cur,
                        next.cfg,
                        &parents,
                        left,
                        right,
                        left_special,
                        right_special,
                        reject_cache,
                    );
                    return false;
                }

                if track_paths
                    && parents.len() < MITM_MAX_REJECT_PARENTS
                    && !accept.contains_key(&next.cfg)
                    && (cur == start || parents.contains_key(&cur))
                {
                    parents.insert(next.cfg, cur);
                }

                let Some((cfg, _)) =
                    mitm_accept_insert_or_widen(&next, &mut accept)
                else {
                    continue;
                };

                if !self.mitm_config_allowed(goal, &cfg) {
                    self.mitm_remember_reject_path(
                        goal,
                        cur,
                        cfg,
                        &parents,
                        left,
                        right,
                        left_special,
                        right_special,
                        reject_cache,
                    );
                    return false;
                }
                todo.push(cfg);
            }
        }

        true
    }

    /// Replay one abstract path with exact singleton weight bounds. Unlike the
    /// normal accept-set saturation this performs no joins or widening, so it is
    /// safe to use solely as an early rejection test for another weight candidate.
    fn mitm_replay_reject_path(
        &self,
        goal: Goal,
        path: &[MitmConfig],
        left: &MitmWfa,
        right: &MitmWfa,
        left_special: &MitmSpecial,
        right_special: &MitmSpecial,
    ) -> bool {
        let mut cur = MitmConfig::start();
        let mut weight = 0_i32;
        if !self.mitm_config_allowed(goal, &cur) {
            return true;
        }

        for &cfg in path {
            let Some(&(write, shift, next_st)) =
                self.get(&(cur.st, cur.co))
            else {
                return false;
            };
            if cfg.st != next_st {
                return false;
            }

            let written = usize::from(write);
            let scanned = usize::from(cfg.co);
            let delta = if shift {
                let (to, pushed) =
                    left.trans[mitm_idx(cur.left)][written];
                let (back, popped) =
                    right.trans[mitm_idx(cfg.right)][scanned];
                if mitm_id(to) != cfg.left || mitm_id(back) != cur.right
                {
                    return false;
                }
                pushed - popped
            } else {
                let (to, pushed) =
                    right.trans[mitm_idx(cur.right)][written];
                let (back, popped) =
                    left.trans[mitm_idx(cfg.left)][scanned];
                if mitm_id(to) != cfg.right || mitm_id(back) != cur.left
                {
                    return false;
                }
                pushed - popped
            };

            let Some(next_weight) = weight.checked_add(delta) else {
                return false;
            };
            let Some(next) = mitm_step_bounds(
                MitmNext {
                    config: cfg,
                    weight: delta,
                    erased_nonzero: goal.is_blank()
                        && cur.co != 0
                        && write == 0,
                },
                MitmBounds {
                    lo: Some(weight),
                    hi: Some(weight),
                },
                left_special,
                right_special,
            ) else {
                return false;
            };

            if (goal.is_blank()
                && mitm_blank_transition_possible(&next))
                || !self.mitm_config_allowed(goal, &cfg)
            {
                return true;
            }
            weight = next_weight;
            cur = cfg;
        }
        false
    }

    #[expect(clippy::too_many_arguments)]
    fn mitm_remember_reject_path(
        &self,
        goal: Goal,
        mut cur: MitmConfig,
        target: MitmConfig,
        parents: &Map<MitmConfig, MitmConfig>,
        left: &MitmWfa,
        right: &MitmWfa,
        left_special: &MitmSpecial,
        right_special: &MitmSpecial,
        cache: &mut MitmRejectCache,
    ) {
        if cache.paths.len() >= MITM_MAX_REJECT_PATHS {
            return;
        }
        let start = MitmConfig::start();
        let mut path = vec![target];
        while cur != start {
            if path.len() >= MITM_MAX_REJECT_PATH_LEN {
                return;
            }
            path.push(cur);
            let Some(&parent) = parents.get(&cur) else {
                return;
            };
            cur = parent;
        }
        path.reverse();

        // Store only a path that independently reproduces the rejection exactly
        // in the candidate that discovered it.
        if !cache.paths.contains(&path)
            && self.mitm_replay_reject_path(
                goal,
                &path,
                left,
                right,
                left_special,
                right_special,
            )
        {
            cache.paths.push(path);
        }
    }

    fn mitm_config_allowed(
        &self,
        goal: Goal,
        cfg: &MitmConfig,
    ) -> bool {
        match goal {
            Goal::Halt => self.get(&(cfg.st, cfg.co)).is_some(),
            Goal::Blank => true,
            Goal::Spinout => !self.mitm_spinout_config_possible(cfg),
        }
    }

    fn mitm_spinout_config_possible(&self, cfg: &MitmConfig) -> bool {
        if cfg.co != 0 {
            return false;
        }
        let Some(&(_, shift, trans)) = self.get(&(cfg.st, 0)) else {
            return false;
        };

        if trans != cfg.st {
            return false;
        }

        // Spinout is only possible when the ray ahead of the moving head may be
        // all zero.  In the MITM WFA, state 0 is the distinguished all-zero ray:
        // it is the start side state and has the required 0/0 self-loop.
        if shift { cfg.right == 0 } else { cfg.left == 0 }
    }

    fn mitm_next_configs_into(
        &self,
        goal: Goal,
        old: MitmConfig,
        left: &MitmWfa,
        right: &MitmWfa,
        left_rev: &MitmRev,
        right_rev: &MitmRev,
        out: &mut Vec<MitmNext>,
    ) {
        let Some(&(write, shift, next_st)) =
            self.get(&(old.st, old.co))
        else {
            return;
        };

        let write = write as usize;
        if shift {
            // Move right: the written symbol joins the left half; the old right
            // predecessor supplies the next scanned symbol.
            let (new_left, left_weight) =
                left.trans[mitm_idx(old.left)][write];
            for edge in &right_rev[mitm_idx(old.right)] {
                out.push(MitmNext {
                    config: MitmConfig {
                        st: next_st,
                        co: edge.symbol,
                        left: mitm_id(new_left),
                        right: edge.from,
                    },
                    weight: left_weight - edge.weight,
                    erased_nonzero: goal.is_blank()
                        && old.co != 0
                        && write == 0,
                });
            }
        } else {
            // Move left: symmetric case.
            let (new_right, right_weight) =
                right.trans[mitm_idx(old.right)][write];
            for edge in &left_rev[mitm_idx(old.left)] {
                out.push(MitmNext {
                    config: MitmConfig {
                        st: next_st,
                        co: edge.symbol,
                        left: edge.from,
                        right: mitm_id(new_right),
                    },
                    weight: right_weight - edge.weight,
                    erased_nonzero: goal.is_blank()
                        && old.co != 0
                        && write == 0,
                });
            }
        }
    }
}

// -----------------------------------------------------------------------------
// MITMWFAR boolean-only decider
// -----------------------------------------------------------------------------

/**************************************/

#[derive(Clone)]
struct MitmWfa {
    states: usize,
    colors: usize,
    trans: Vec<Vec<(usize, i32)>>,
}

#[derive(Clone, Copy)]
struct MitmRevEdge {
    from: u32,
    symbol: Color,
    weight: i32,
}

type MitmRev = Vec<Vec<MitmRevEdge>>;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct MitmConfig {
    st: State,
    co: Color,
    left: u32,
    right: u32,
}

#[inline]
const fn mitm_idx(state: u32) -> usize {
    state as usize
}

#[inline]
fn mitm_id(state: usize) -> u32 {
    u32::try_from(state).expect("MITM state id exceeded u32::MAX")
}

impl MitmConfig {
    const fn start() -> Self {
        Self {
            st: 0,
            co: 0,
            left: 0,
            right: 0,
        }
    }
}

#[derive(Clone, Copy)]
struct MitmNext {
    config: MitmConfig,
    weight: i32,
    erased_nonzero: bool,
}

#[derive(Clone, Copy)]
struct MitmSearchParams {
    goal_transitions: usize,
    max_weight_pairs: usize,
}

#[derive(Clone, Default)]
struct MitmRejectCache {
    paths: Vec<Vec<MitmConfig>>,
}

#[derive(Clone, Copy)]
struct MitmMemory {
    left: usize,
    right: usize,
}

impl MitmMemory {
    const fn new(left: usize, right: usize) -> Self {
        Self { left, right }
    }
}

#[derive(Clone, Copy, Default)]
struct MitmBounds {
    lo: Option<i32>,
    hi: Option<i32>,
}

impl MitmBounds {
    fn contains_zero(&self) -> bool {
        self.lo.is_none_or(|lo| lo <= 0)
            && self.hi.is_none_or(|hi| hi >= 0)
    }
}

#[derive(Clone)]
struct MitmSpecial {
    nonneg: Vec<bool>,
    nonpos: Vec<bool>,
}

#[derive(Default)]
struct MitmWeightSlots {
    left: Vec<(usize, usize)>,
    right: Vec<(usize, usize)>,
}

type MitmAccept = Map<MitmConfig, MitmBounds>;

#[derive(Clone, Copy)]
enum MitmSide {
    Left,
    Right,
}

struct MitmNextBounds {
    cfg: MitmConfig,
    bounds: MitmBounds,
    hard_lo: bool,
    hard_hi: bool,
    erased_nonzero: bool,
}

fn mitm_step_bounds(
    next: MitmNext,
    bounds: MitmBounds,
    left_special: &MitmSpecial,
    right_special: &MitmSpecial,
) -> Option<MitmNextBounds> {
    let cfg = next.config;
    let mut lo = bounds.lo.map(|x| x + next.weight);
    let mut hi = bounds.hi.map(|x| x + next.weight);

    let hard_lo = left_special.nonneg[mitm_idx(cfg.left)]
        && right_special.nonneg[mitm_idx(cfg.right)];
    if hard_lo && lo.is_none_or(|x| x < 0) {
        lo = Some(0);
    }

    let hard_hi = left_special.nonpos[mitm_idx(cfg.left)]
        && right_special.nonpos[mitm_idx(cfg.right)];
    if hard_hi && hi.is_none_or(|x| x > 0) {
        hi = Some(0);
    }

    if let (Some(lo), Some(hi)) = (lo, hi)
        && hi < lo
    {
        return None;
    }

    Some(MitmNextBounds {
        cfg,
        bounds: MitmBounds { lo, hi },
        hard_lo,
        hard_hi,
        erased_nonzero: next.erased_nonzero,
    })
}

#[expect(clippy::unwrap_in_result)]
fn mitm_accept_insert_or_widen(
    next: &MitmNextBounds,
    accept: &mut MitmAccept,
) -> Option<(MitmConfig, MitmBounds)> {
    let Some(old) = accept.get_mut(&next.cfg) else {
        accept.insert(next.cfg, next.bounds);
        return Some((next.cfg, next.bounds));
    };

    let mut changed = false;

    if let Some(old_lo) = old.lo
        && (next.bounds.lo.is_none() || Some(old_lo) > next.bounds.lo)
    {
        changed = true;
        if old.hi.is_none()
            || next.bounds.lo.is_none()
            || old.hi.unwrap() - next.bounds.lo.unwrap()
                > MITM_MAX_FINITE_INTERVAL
        {
            old.lo = next.hard_lo.then_some(0);
        } else {
            old.lo = next.bounds.lo;
        }
    }

    if let Some(old_hi) = old.hi
        && (next.bounds.hi.is_none() || Some(old_hi) < next.bounds.hi)
    {
        changed = true;
        if old.lo.is_none()
            || next.bounds.hi.is_none()
            || next.bounds.hi.unwrap() - old.lo.unwrap()
                > MITM_MAX_FINITE_INTERVAL
        {
            old.hi = next.hard_hi.then_some(0);
        } else {
            old.hi = next.bounds.hi;
        }
    }

    changed.then_some((next.cfg, *old))
}

fn mitm_blank_transition_possible(next: &MitmNextBounds) -> bool {
    next.erased_nonzero
        && next.cfg.co == 0
        && next.cfg.left == 0
        && next.cfg.right == 0
        && next.bounds.contains_zero()
}

impl MitmWfa {
    fn new(colors: usize) -> Self {
        Self {
            states: 2,
            colors,
            trans: vec![vec![(MITM_DEAD, 0); colors]; 2],
        }
    }

    fn push_dead_state(&mut self) -> usize {
        let new_state = self.states;
        self.states += 1;
        self.trans.push(vec![(MITM_DEAD, 0); self.colors]);
        new_state
    }

    fn pop_state(&mut self) {
        self.states -= 1;
        self.trans.pop();
    }

    fn rev_edges(&self) -> MitmRev {
        let mut rev = vec![Vec::new(); self.states];
        for from in 0..self.states {
            for symbol in 0..self.colors {
                let (to, weight) = self.trans[from][symbol];
                #[expect(clippy::cast_possible_truncation)]
                rev[to].push(MitmRevEdge {
                    from: mitm_id(from),
                    symbol: symbol as Color,
                    weight,
                });
            }
        }
        rev
    }

    fn verify_leading_blank(&self) -> bool {
        self.trans[0][0] == (0, 0)
    }

    fn derive_special(&self) -> MitmSpecial {
        let mut possible_neg = vec![false; self.states];
        let mut possible_pos = vec![false; self.states];

        #[expect(clippy::disallowed_names)]
        for row in &self.trans {
            for &(to, weight) in row {
                if weight < 0 {
                    possible_neg[to] = true;
                }
                if weight > 0 {
                    possible_pos[to] = true;
                }
            }
        }

        self.complete_closure(&mut possible_neg);
        self.complete_closure(&mut possible_pos);

        MitmSpecial {
            nonneg: possible_neg.into_iter().map(|x| !x).collect(),
            nonpos: possible_pos.into_iter().map(|x| !x).collect(),
        }
    }

    fn complete_closure(&self, states: &mut [bool]) {
        let mut todo: Vec<usize> = states
            .iter()
            .enumerate()
            .filter_map(|(state, &yes)| yes.then_some(state))
            .collect();

        while let Some(cur) = todo.pop() {
            for &(next, _) in &self.trans[cur] {
                if !states[next] {
                    states[next] = true;
                    todo.push(next);
                }
            }
        }
    }

    fn verify_special(&self, special: &MitmSpecial) -> bool {
        for from in 0..self.states {
            for &(to, weight) in &self.trans[from] {
                if special.nonpos[to]
                    && (!special.nonpos[from] || weight > 0)
                {
                    return false;
                }
                if special.nonneg[to]
                    && (!special.nonneg[from] || weight < 0)
                {
                    return false;
                }
            }
        }
        true
    }

    fn with_memory(&self) -> Self {
        let mut new_state_numbers =
            vec![vec![usize::MAX; self.colors]; self.states];
        let mut next_state = 0;

        #[expect(clippy::disallowed_names)]
        for (old_state, row) in self.trans.iter().enumerate() {
            for (old_symbol, &(to, _)) in row.iter().enumerate() {
                if to != MITM_DEAD {
                    new_state_numbers[old_state][old_symbol] =
                        next_state;
                    next_state += 1;
                    if old_state == 0 && old_symbol == 0 {
                        // Preserve upstream numbering quirk: state 1 remains dead.
                        next_state += 1;
                    }
                }
            }
        }

        let mut out = Self {
            states: next_state,
            colors: self.colors,
            trans: vec![vec![(MITM_DEAD, 0); self.colors]; next_state],
        };

        for from_old_state in 0..self.states {
            for from_old_symbol in 0..self.colors {
                let from_new =
                    new_state_numbers[from_old_state][from_old_symbol];
                if from_new == usize::MAX {
                    continue;
                }
                let to_old_state =
                    self.trans[from_old_state][from_old_symbol].0;
                for to_old_symbol in 0..self.colors {
                    let (to_old_next, weight) =
                        self.trans[to_old_state][to_old_symbol];
                    if to_old_next != MITM_DEAD {
                        let to_new = new_state_numbers[to_old_state]
                            [to_old_symbol];
                        out.trans[from_new][to_old_symbol] =
                            (to_new, weight);
                    }
                }
            }
        }

        out
    }
}
