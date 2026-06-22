#![expect(clippy::too_many_arguments)]
//! FAR/MITM non-halting prover
//!
//! Self-contained FAR decider with MITMWFAR folded into `far_cant_halt`, implemented as methods on `Prog`.
//!
//! Works for **any number of tape colors** (`COLORS >= 2`) and typical Busy Beaver
//! sizes (e.g. `states <= 8`, `colors <= 8`).
//!
//! # Public API
//! ```ignore
//! // Single effort knob: higher => more analysis / more chances to prove non-halting.
//! // Reasonable values: 2 .. 64/128/256.
//! let proved = prog.far_cant_halt(64);
//! ```
//!
//! Behavior:
//! - Returns `true` only when FAR or MITM/WFAR proves that the machine cannot halt.
//! - Returns `false` otherwise (it may halt, or FAR ran out of budgets).
//!
//! # How the knob works
//! `knob` controls *both*:
//! - the range of block lengths to try (roughly `2..=knob` but with a colors-based cap)
//! - budgets for each FAR run (more work / deeper block simulation)

use core::{cmp::Ordering, hash::Hash};

use ahash::{AHashMap as Map, AHashSet as Set};

use crate::{Color, Goal, Instr, Prog, Slot, State};

// -----------------------------------------------------------------------------
// Top-level tuning constants
// -----------------------------------------------------------------------------

/// Base exploration budget per block length unit.
///
/// With `knob = 16`, `block_len = 16`, effort factor = 1, this gives:
/// `max_work ≈ 12_500 * 16 = 200_000` (matching the previous single-run default).
const FAR_WORK_PER_LEN: usize = 12_500;

/// Base per-block raw-step budget per block length unit.
///
/// With `knob = 16`, `block_len = 16`, effort factor = 1, this gives:
/// `block_step_limit ≈ 200 * 16 = 3_200`.
const FAR_STEP_PER_LEN: usize = 200;

/// Hard cap on block lengths we will try.
/// (Even if `knob` is bigger.)
const FAR_BLOCK_LEN_HARD_CAP: usize = 256;

/// Practical caps by alphabet size (keeps the parameter sweep sane).
///
/// You can loosen these if you want, but for `COLORS > 2` huge block lengths are
/// rarely helpful and tend to explode state-space.
const FAR_BLOCK_LEN_CAP_COLORS_2: usize = 256;
const FAR_BLOCK_LEN_CAP_COLORS_3_4: usize = 128;
const FAR_BLOCK_LEN_CAP_COLORS_5_8: usize = 64;

/// Minimum knob value.
const FAR_KNOB_MIN: usize = 2;

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

/// C++ FAR::RWL_mod defaults.
const FAR_RWL_LEN_H: usize = 8;
const FAR_RWL_LEN_H_TAIL: usize = 0;
const FAR_RWL_MNC: usize = 2;
const FAR_RWL_MOD: usize = 1;

/// C++ FAR::CPS_LRU defaults.
const FAR_CPS_LRU_LEN_H: usize = 8;
const FAR_CPS_LRU_LEN_H_NO_LRU: usize = 2;

/// C++ FAR::RNGS_mod defaults.
const FAR_RNGS_NG_N: usize = 4;
const FAR_RNGS_LEN_H: usize = 8;
const FAR_RNGS_MNC: usize = 2;
const FAR_RNGS_MOD: usize = 1;
const FAR_RNGS_BS_N: usize = 0;

/// C++ FAR::RS_mod defaults.
const FAR_RS_NG_N: usize = 4;
const FAR_RS_LEN_H: usize = 8;
const FAR_RS_MNC: usize = 2;
const FAR_RS_MOD: usize = 1;
const FAR_RS_STRICT: bool = true;

// MITM parameters -------------------------------------------------------------

const MITM_DEAD: usize = 1;
const MITM_MAX_FINITE_INTERVAL: i32 = 100;
const MITM_MAX_TRANSITIONS: usize = 10;
const MITM_MAX_WEIGHT_PAIRS: usize = 1;

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

/// Compute an "effort factor" from the knob.
///
/// This increases budgets uniformly as knob grows:
/// - knob < 16 => 1
/// - 16..31    => 1
/// - 32..63    => 2
/// - 64..127   => 4
/// - 128..255  => 8
/// - 256..     => 16
fn effort_factor(knob: usize) -> usize {
    (knob / 16).max(1)
}

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

// -----------------------------------------------------------------------------
// Public method on Prog
// -----------------------------------------------------------------------------

impl<const STATES: usize, const COLORS: usize> Prog<STATES, COLORS> {
    /// FAR non-halting prover (Finite Automaton Reduction).
    ///
    /// `knob` is the only user-facing parameter.
    /// Higher means more analysis and more chances to prove non-halting.
    ///
    /// Returns:
    /// - `true` iff FAR or MITM/WFAR proved the machine cannot halt.
    /// - `false` otherwise.
    pub fn far_cant_halt(&self, knob: usize) -> bool {
        self.far_sweep_fast(knob, Goal::Halt)
            || self.mitm_cant_halt()
            || self.far_sweep_slow(knob, Goal::Halt)
            || self.direct_far_cant_target(Goal::Halt)
    }

    /// FAR blank-tape prover.
    ///
    /// Returns `true` iff FAR proved that the machine can never blank the tape
    /// after time 0.  The initial all-zero tape is ignored.  This runs the same
    /// FAR fixed-point computation as `far_cant_halt`, but tracks whether any
    /// non-zero block has ever been seen and treats a derived all-zero
    /// left-context / active-block configuration as the goal.  If the FAR
    /// over-approximation reaches a fixed point without deriving that goal,
    /// the tape cannot ever become globally blank.  Because FAR summaries remember
    /// history rather than exact present contents, a zero context is recognized by
    /// asking whether the summary is compatible with an all-zero block stack, not
    /// by requiring the initial DFA state.
    pub fn far_cant_blank(&self, knob: usize) -> bool {
        self.far_sweep_fast(knob, Goal::Blank)
            || self.mitm_cant_blank()
            || self.far_sweep_slow(knob, Goal::Blank)
            || self.direct_far_cant_target(Goal::Blank)
    }

    /// FAR spinout prover.
    ///
    /// Returns `true` iff FAR proves that the machine can never enter a
    /// one-sided all-zero same-state drift.
    pub fn far_cant_spinout(&self, knob: usize) -> bool {
        self.far_sweep_fast(knob, Goal::Spinout)
            || self.mitm_cant_spinout()
            || self.far_sweep_slow(knob, Goal::Spinout)
            || self.direct_far_cant_target(Goal::Spinout)
    }
}

// -----------------------------------------------------------------------------
// Internal implementation
// -----------------------------------------------------------------------------

/// A block-word: a length-`len` vector of tape symbols.
///
/// `Word` uses a `Vec<Color>` so it supports any number of colors.
#[derive(Clone, Eq, PartialEq, Debug, Hash)]
struct Word {
    cells: Vec<Color>,
}

impl Word {
    fn zero(len: usize) -> Self {
        Self {
            cells: vec![0; len],
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

    fn set(&mut self, idx: usize, v: Color) {
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
        self
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

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct WordId(usize);

#[derive(Clone, Debug)]
struct WordInterner {
    ids: Map<Word, WordId>,
    words: Vec<Word>,
}

impl WordInterner {
    fn new() -> Self {
        Self {
            ids: Map::new(),
            words: Vec::new(),
        }
    }

    fn intern(&mut self, w: Word) -> WordId {
        if let Some(&id) = self.ids.get(&w) {
            return id;
        }
        let id = WordId(self.words.len());
        self.words.push(w.clone());
        self.ids.insert(w, id);
        id
    }

    fn get(&self, id: WordId) -> &Word {
        &self.words[id.0]
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
    saw_nonzero: bool,
    hit_blank: bool,
}

/// Non-interned result used only while computing a cache miss.
#[derive(Clone, Debug)]
struct RawWordUpdateLemma {
    w1: Word,
    s1: Option<State>,
    is_back: bool,
    saw_nonzero: bool,
    hit_blank: bool,
}

impl RawWordUpdateLemma {
    fn exit_oriented(
        w1: Word,
        s1: Option<State>,
        is_back: bool,
        saw_nonzero: bool,
        hit_blank: bool,
    ) -> Self {
        let mut res = Self {
            w1,
            s1,
            is_back,
            saw_nonzero,
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
    fn new() -> Self;
    fn push(
        &mut self,
        w: WordId,
        words: &mut WordInterner,
    ) -> Result<(), SummaryOverflow>;

    /// True iff this finite summary is compatible with a concrete stack whose
    /// stored blocks are all zero.  FAR summaries are lossy history summaries:
    /// after a non-zero block is erased, the summary may still be non-initial
    /// while the represented concrete stack is all-zero.  Blank-tape goal
    /// detection must therefore use this predicate instead of `id == initial`.
    fn may_be_all_zero_context(&self, words: &WordInterner) -> bool;
}

#[derive(Clone, Copy, Debug)]
struct SummaryOverflow;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct RepeatWord {
    w: WordId,
    n: usize,
    m: usize,
}

impl RepeatWord {
    const fn new(w: WordId, n: usize, m: usize) -> Self {
        Self { w, n, m }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct Ng1Summary {
    q: Vec<WordId>,
}

impl Summary for Ng1Summary {
    fn new() -> Self {
        Self { q: Vec::new() }
    }

    fn push(
        &mut self,
        w: WordId,
        words: &mut WordInterner,
    ) -> Result<(), SummaryOverflow> {
        if self.q.is_empty() && words.get(w).is_zero() {
            return Ok(());
        }
        if FAR_NG1_N > 0 && self.q.len() == FAR_NG1_N {
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
    q: Vec<WordId>,
    q0: Vec<WordId>,
    mod_pos: usize,
}

impl<const TAIL_H: usize, const POS_MOD: usize> Summary
    for NgSummary<TAIL_H, POS_MOD>
{
    fn new() -> Self {
        Self {
            q: Vec::new(),
            q0: Vec::new(),
            mod_pos: 0,
        }
    }

    fn push(
        &mut self,
        w: WordId,
        words: &mut WordInterner,
    ) -> Result<(), SummaryOverflow> {
        if self.q.is_empty() && words.get(w).is_zero() {
            return Ok(());
        }

        if FAR_NG_N > 0 && self.q.len() == FAR_NG_N {
            self.q.remove(0);
        }
        self.q.push(w);

        if self.q0.len() < TAIL_H {
            self.q0.push(w);
        }

        self.mod_pos = (self.mod_pos + 1) % POS_MOD.max(1);
        Ok(())
    }

    fn may_be_all_zero_context(&self, words: &WordInterner) -> bool {
        self.q.iter().all(|&w| words.get(w).is_zero())
            && self.q0.iter().all(|&w| words.get(w).is_zero())
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct NgSetSummary {
    q: Vec<WordId>,
    lru: Vec<Vec<WordId>>,
}

impl Summary for NgSetSummary {
    fn new() -> Self {
        Self {
            q: Vec::new(),
            lru: Vec::new(),
        }
    }

    fn push(
        &mut self,
        w: WordId,
        words: &mut WordInterner,
    ) -> Result<(), SummaryOverflow> {
        if self.q.is_empty() && words.get(w).is_zero() {
            return Ok(());
        }

        let old_ngram = self.q.clone();
        self.q.insert(0, w);

        if self.q.len() > FAR_NGSET_NG_N {
            self.q.truncate(FAR_NGSET_NG_N);

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
    q: Vec<WordId>,
    lru: Vec<(WordId, WordId)>,
}

impl Summary for LruPairSummary {
    fn new() -> Self {
        Self {
            q: Vec::new(),
            lru: Vec::new(),
        }
    }

    fn push(
        &mut self,
        w: WordId,
        words: &mut WordInterner,
    ) -> Result<(), SummaryOverflow> {
        if self.q.is_empty() && words.get(w).is_zero() {
            return Ok(());
        }

        self.q.push(w);
        if self.q.len()
            <= FAR_LRU_PAIR_LEN_H_NO_LRU + FAR_LRU_PAIR_LEN_H_TAIL
        {
            return Ok(());
        }

        if FAR_LRU_PAIR_LEN_H_TAIL > 0
            && FAR_LRU_PAIR_LEN_H
                > FAR_LRU_PAIR_LEN_H_NO_LRU + FAR_LRU_PAIR_LEN_H_TAIL
        {
            let i = FAR_LRU_PAIR_LEN_H_TAIL;
            let pair = (self.q[i - 1], self.q[i]);
            if let Some(pos) = self.lru.iter().position(|&p| p == pair)
            {
                self.lru.remove(pos);
            } else {
                let max_lru = FAR_LRU_PAIR_LEN_H
                    - FAR_LRU_PAIR_LEN_H_NO_LRU
                    - FAR_LRU_PAIR_LEN_H_TAIL;
                if self.lru.len() >= max_lru && max_lru > 0 {
                    self.lru.pop();
                }
            }
            self.lru.insert(0, pair);
        }

        self.q.remove(FAR_LRU_PAIR_LEN_H_TAIL);
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
    q: Vec<WordId>,
    lru: Vec<(WordId, WordId)>,
}

impl Summary for SetPairSummary {
    fn new() -> Self {
        Self {
            q: Vec::new(),
            lru: Vec::new(),
        }
    }

    fn push(
        &mut self,
        w: WordId,
        words: &mut WordInterner,
    ) -> Result<(), SummaryOverflow> {
        if self.q.is_empty() && words.get(w).is_zero() {
            return Ok(());
        }

        self.q.push(w);
        if self.q.len()
            <= FAR_SET_PAIR_LEN_H_NO_LRU + FAR_SET_PAIR_LEN_H_TAIL
        {
            return Ok(());
        }

        if FAR_SET_PAIR_LEN_H_TAIL > 0
            && FAR_SET_PAIR_LEN_H
                > FAR_SET_PAIR_LEN_H_NO_LRU + FAR_SET_PAIR_LEN_H_TAIL
        {
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
        }

        self.q.remove(FAR_SET_PAIR_LEN_H_TAIL);
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
struct RwlModSummary {
    q: Vec<RepeatWord>,
}

impl Summary for RwlModSummary {
    fn new() -> Self {
        Self { q: Vec::new() }
    }

    fn push(
        &mut self,
        w: WordId,
        words: &mut WordInterner,
    ) -> Result<(), SummaryOverflow> {
        if self.q.is_empty() {
            if !words.get(w).is_zero() {
                self.q.push(RepeatWord::new(
                    w,
                    1,
                    1 % FAR_RWL_MOD.max(1),
                ));
            }
            return Ok(());
        }

        if let Some(last) = self.q.last_mut()
            && last.w == w
        {
            last.n = last.n.saturating_add(1).min(FAR_RWL_MNC);
            last.m = (last.m + 1) % FAR_RWL_MOD.max(1);
            return Ok(());
        }

        self.q.push(RepeatWord::new(w, 1, 1 % FAR_RWL_MOD.max(1)));
        if self.q.len() > FAR_RWL_LEN_H {
            #[expect(clippy::unnecessary_min_or_max)]
            let idx = FAR_RWL_LEN_H_TAIL.min(self.q.len() - 1);
            self.q.remove(idx);
        }
        Ok(())
    }

    fn may_be_all_zero_context(&self, words: &WordInterner) -> bool {
        self.q.iter().all(|rw| words.get(rw.w).is_zero())
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct CpsLruSummary {
    ls: Vec<WordId>,
}

impl Summary for CpsLruSummary {
    fn new() -> Self {
        Self { ls: Vec::new() }
    }

    fn push(
        &mut self,
        w: WordId,
        words: &mut WordInterner,
    ) -> Result<(), SummaryOverflow> {
        if self.ls.is_empty() && words.get(w).is_zero() {
            return Ok(());
        }
        self.ls.insert(0, w);
        if self.ls.len() <= FAR_CPS_LRU_LEN_H_NO_LRU {
            return Ok(());
        }
        if FAR_CPS_LRU_LEN_H_NO_LRU + 1 > self.ls.len() {
            return Ok(());
        }
        let key = self.ls[FAR_CPS_LRU_LEN_H_NO_LRU];
        let start = FAR_CPS_LRU_LEN_H_NO_LRU + 1;
        let mut remove_idx = None;
        for i in start..self.ls.len() {
            if self.ls[i] == key {
                remove_idx = Some(i);
                break;
            }
        }
        if remove_idx.is_none() && self.ls.len() > FAR_CPS_LRU_LEN_H {
            remove_idx = Some(self.ls.len() - 1);
        }
        if let Some(i) = remove_idx {
            self.ls.remove(i);
        }
        Ok(())
    }

    fn may_be_all_zero_context(&self, words: &WordInterner) -> bool {
        self.ls.iter().all(|&w| words.get(w).is_zero())
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct RngsModSummary {
    q: Vec<RepeatWord>,
    q0: Vec<WordId>,
    w1: Option<WordId>,
}

impl Summary for RngsModSummary {
    fn new() -> Self {
        Self {
            q: Vec::new(),
            q0: Vec::new(),
            w1: None,
        }
    }

    fn push(
        &mut self,
        w: WordId,
        words: &mut WordInterner,
    ) -> Result<(), SummaryOverflow> {
        if self.w1.is_none() && words.get(w).is_zero() {
            return Ok(());
        }

        let w1 = ngram_word_id(w, self.w1, FAR_RNGS_NG_N, words);
        self.w1 = Some(w1);
        let mut key = w1;

        self.q0.push(key);
        if self.q0.len() > FAR_RNGS_BS_N {
            key = self.q0.remove(0);
        } else {
            return Ok(());
        }

        promote_repeat_word(
            &mut self.q,
            key,
            FAR_RNGS_LEN_H,
            FAR_RNGS_MNC,
            FAR_RNGS_MOD,
            false,
        )
    }

    fn may_be_all_zero_context(&self, words: &WordInterner) -> bool {
        self.w1.is_none_or(|w| words.get(w).is_zero())
            && self.q0.iter().all(|&w| words.get(w).is_zero())
            && self.q.iter().all(|rw| words.get(rw.w).is_zero())
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct RsModSummary {
    q: Vec<RepeatWord>,
    q0: Vec<WordId>,
}

impl Summary for RsModSummary {
    fn new() -> Self {
        Self {
            q: Vec::new(),
            q0: Vec::new(),
        }
    }

    fn push(
        &mut self,
        w: WordId,
        words: &mut WordInterner,
    ) -> Result<(), SummaryOverflow> {
        if self.q0.is_empty() && words.get(w).is_zero() {
            return Ok(());
        }

        self.q0.push(w);
        let key = if self.q0.len() > FAR_RS_NG_N {
            self.q0.remove(0)
        } else {
            return Ok(());
        };

        promote_repeat_word(
            &mut self.q,
            key,
            FAR_RS_LEN_H,
            FAR_RS_MNC,
            FAR_RS_MOD,
            FAR_RS_STRICT,
        )
    }

    fn may_be_all_zero_context(&self, words: &WordInterner) -> bool {
        self.q0.iter().all(|&w| words.get(w).is_zero())
            && self.q.iter().all(|rw| words.get(rw.w).is_zero())
    }
}

fn ngram_word_id(
    head: WordId,
    tail: Option<WordId>,
    limit: usize,
    words: &mut WordInterner,
) -> WordId {
    if limit == 0 {
        return words.intern(Word { cells: Vec::new() });
    }

    let mut cells = Vec::with_capacity(limit);
    cells.extend(words.get(head).cells.iter().copied());
    if cells.len() < limit
        && let Some(tail) = tail
    {
        cells.extend(words.get(tail).cells.iter().copied());
    }
    cells.truncate(limit);
    words.intern(Word { cells })
}

fn promote_repeat_word(
    q: &mut Vec<RepeatWord>,
    key: WordId,
    len_h: usize,
    mnc: usize,
    modu: usize,
    strict: bool,
) -> Result<(), SummaryOverflow> {
    let modulo = modu.max(1);
    for i in 0..q.len() {
        if q[i].w == key {
            let mut rep = q.remove(i);
            rep.n = rep.n.saturating_add(1).min(mnc);
            rep.m = (rep.m + 1) % modulo;
            q.push(rep);
            return Ok(());
        }
    }

    q.push(RepeatWord::new(key, 1, 1 % modulo));
    if q.len() > len_h {
        if strict {
            return Err(SummaryOverflow);
        }
        q.remove(0);
    }
    Ok(())
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct H2 {
    s: State,
    r: usize,
    dirty: bool,
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct H2b {
    s: State,
    r: usize,
    dirty: bool,
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct H3 {
    w: WordId,
    s: State,
    r: usize,
    dirty: bool,
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct DfaEdge {
    w: WordId,
    prev: usize,
}

/// A set with a LIFO todo stack.
#[derive(Clone, Debug)]
struct TodoSet<K: Eq + Hash + Clone> {
    st: Set<K>,
    todo: Vec<K>,
}

impl<K: Eq + Hash + Clone> TodoSet<K> {
    fn new() -> Self {
        Self {
            st: Set::new(),
            todo: Vec::new(),
        }
    }

    fn insert(&mut self, k: K) -> bool {
        if self.st.insert(k.clone()) {
            self.todo.push(k);
            true
        } else {
            false
        }
    }

    fn contains(&self, k: &K) -> bool {
        self.st.contains(k)
    }

    fn pop_todo(&mut self) -> Option<K> {
        self.todo.pop()
    }
}

/// A map K -> set(V) with a todo stack of inserted pairs.
#[derive(Clone, Debug)]
struct TodoMap<K: Eq + Hash + Clone, V: Eq + Hash + Clone> {
    mp: Map<K, Set<V>>,
    todo: Vec<(K, V)>,
}

impl<K: Eq + Hash + Clone, V: Eq + Hash + Clone> TodoMap<K, V> {
    fn new() -> Self {
        Self {
            mp: Map::new(),
            todo: Vec::new(),
        }
    }

    fn insert(&mut self, k: K, v: V) -> bool {
        let entry = self.mp.entry(k.clone()).or_default();
        if entry.insert(v.clone()) {
            self.todo.push((k, v));
            true
        } else {
            false
        }
    }

    fn values<'a>(&'a self, k: &K) -> impl Iterator<Item = &'a V> {
        self.mp.get(k).into_iter().flat_map(|s| s.iter())
    }

    fn pop_todo(&mut self) -> Option<(K, V)> {
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
        dirty_before: bool,
        context_may_be_all_zero: bool,
    },
    Spinout {
        back_context_may_be_all_zero: bool,
        forward_context_may_be_all_zero: bool,
    },
}

impl StepContext {
    const fn blank(
        dirty_before: bool,
        context_may_be_all_zero: bool,
    ) -> Self {
        Self::Blank {
            dirty_before,
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
    max_work: usize,
    block_step_limit: usize,
    goal: Goal,
    mirrored: bool,
    reached: ReachedParams,
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

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
struct StepKey {
    w: WordId,
    s: State,
    sgn: i8,
    ctx: StepContext,
}

/// FAR decider with a pluggable DFA history summary.
struct FarDecider<
    'a,
    const STATES: usize,
    const COLORS: usize,
    S: Summary,
> {
    prog: &'a Prog<STATES, COLORS>,
    goal: Goal,
    mirrored: bool,
    reached: ReachedParams,

    block_len: usize,
    max_work: usize,
    block_step_limit: usize,

    // Interned FAR block words.  Relation keys, DFA edges, push cache keys,
    // and step-cache keys carry compact WordId values instead of cloning Vecs.
    words: WordInterner,

    // work counter
    work: usize,

    // Cache exact block simulations by local FAR context.
    step_cache: Map<StepKey, Option<WordUpdateLemma>>,

    // DFA
    id: Map<S, usize>,
    idr: Vec<S>,

    pop: Vec<Vec<DfaEdge>>,
    push: Map<(WordId, usize), usize>,
    new_pops: Vec<(usize, DfaEdge)>,

    // relations
    ret2: TodoMap<H2, H2b>,
    ret3: TodoMap<H3, H2b>,
    pre23: TodoMap<H2, H3>,
    pre32: TodoMap<H3, H2>,
    pre33: TodoMap<H3, H3>,

    pre3l: TodoSet<H3>,
    retl: TodoSet<H2b>,
    h3s: TodoSet<H3>,
    h2s: TodoSet<H2>,

    // For each DFA state r, which (machine state, dirty) pairs have H2(s,r,dirty)
    r_s: Vec<Set<(State, bool)>>,

    // Exact existential zero-context information for the lossy summary DFA.
    // `zero_context[r]` means that state `r` is reachable from the initial
    // summary using only all-zero blocks.  Looking only at the contents of a
    // lossy summary is too permissive: a forgotten non-zero block can leave a
    // summary that happens to contain only zero-looking data even though no
    // all-zero stack reaches that state.
    zero_context: Vec<bool>,
    zero_push: Vec<Vec<usize>>,

    // Reusable buffers for relation propagation. These avoid allocating short
    // temporary Vecs just to break immutable borrows before mutating self.
    scratch_states: Vec<(State, bool)>,
    scratch_edges: Vec<DfaEdge>,
    scratch_h2: Vec<H2>,
    scratch_h2b: Vec<H2b>,
    scratch_h3: Vec<H3>,
}

impl<const STATES: usize, const COLORS: usize, S: Summary>
    FarDecider<'_, STATES, COLORS, S>
{
    fn with_init_dfa(mut self) -> Result<Self, StopReason> {
        self.ensure_dfa_capacity(0);

        let id0 = self.get_id(S::new());
        debug_assert_eq!(id0, 1);
        self.mark_zero_context(id0);

        let blank = Word::zero(self.block_len);
        let id1 = self.dfa_push(blank, id0)?;
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

    fn ensure_dfa_capacity(&mut self, id: usize) {
        if self.pop.len() <= id {
            self.pop.resize_with(id + 1, Vec::new);
        }
        if self.r_s.len() <= id {
            self.r_s.resize_with(id + 1, Set::new);
        }
        if self.zero_context.len() <= id {
            self.zero_context.resize(id + 1, false);
        }
        if self.zero_push.len() <= id {
            self.zero_push.resize_with(id + 1, Vec::new);
        }
    }

    fn mark_zero_context(&mut self, start: usize) {
        let mut todo = vec![start];
        while let Some(r) = todo.pop() {
            self.ensure_dfa_capacity(r);
            if self.zero_context[r] {
                continue;
            }
            self.zero_context[r] = true;
            todo.extend(self.zero_push[r].iter().copied());
        }
    }

    fn get_id(&mut self, st: S) -> usize {
        if let Some(&id) = self.id.get(&st) {
            return id;
        }
        let id = self.idr.len();
        self.id.insert(st.clone(), id);
        self.idr.push(st);
        self.ensure_dfa_capacity(id);
        id
    }

    fn dfa_push(
        &mut self,
        w: Word,
        ls: usize,
    ) -> Result<usize, StopReason> {
        let wid = self.words.intern(w);
        self.dfa_push_id(wid, ls)
    }

    fn dfa_push_id(
        &mut self,
        wid: WordId,
        ls: usize,
    ) -> Result<usize, StopReason> {
        let key = (wid, ls);
        if let Some(&to) = self.push.get(&key) {
            return Ok(to);
        }

        let mut st = self.idr[ls].clone();
        st.push(wid, &mut self.words)
            .map_err(|_| StopReason::SummaryOverflow)?;
        let to = self.get_id(st);
        self.push.insert(key, to);
        self.new_pops.push((to, DfaEdge { w: wid, prev: ls }));

        if self.words.get(wid).is_zero() {
            self.ensure_dfa_capacity(ls.max(to));
            if !self.zero_push[ls].contains(&to) {
                self.zero_push[ls].push(to);
            }
            if self.zero_context[ls] {
                self.mark_zero_context(to);
            }
        }

        Ok(to)
    }

    fn summary_may_be_all_zero_context(&self, r: usize) -> bool {
        let exact = self.zero_context.get(r).copied().unwrap_or(false);
        debug_assert!(
            !exact
                || self.idr.get(r).is_some_and(|st| {
                    st.may_be_all_zero_context(&self.words)
                })
        );
        exact
    }

    const fn canonical_dirty(&self, dirty: bool) -> bool {
        self.goal.is_blank() && dirty
    }

    const fn canonical_h2(&self, mut a: H2) -> H2 {
        a.dirty = self.canonical_dirty(a.dirty);
        a
    }

    const fn canonical_h2b(&self, mut b: H2b) -> H2b {
        b.dirty = self.canonical_dirty(b.dirty);
        b
    }

    const fn canonical_h3(&self, mut c: H3) -> H3 {
        c.dirty = self.canonical_dirty(c.dirty);
        c
    }

    fn h2_pop_step_context(
        &self,
        dirty: bool,
        r0: usize,
    ) -> StepContext {
        match self.goal {
            Goal::Halt => StepContext::Normal,
            Goal::Blank => StepContext::blank(
                dirty,
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
                    self.summary_may_be_all_zero_context(c.r);
                let b_context_zero =
                    self.summary_may_be_all_zero_context(b.r);
                StepContext::blank(
                    c.dirty || b.dirty,
                    c_context_zero && b_context_zero,
                )
            },
            Goal::Spinout => {
                let c_context_zero =
                    self.summary_may_be_all_zero_context(c.r);
                let b_context_zero =
                    self.summary_may_be_all_zero_context(b.r);
                StepContext::spinout(b_context_zero, c_context_zero)
            },
        }
    }

    fn retl_step_context(&self, dirty: bool, r0: usize) -> StepContext {
        match self.goal {
            Goal::Halt => StepContext::Normal,
            Goal::Blank => StepContext::blank(
                dirty,
                self.summary_may_be_all_zero_context(r0),
            ),
            Goal::Spinout => StepContext::spinout(
                self.summary_may_be_all_zero_context(r0),
                true,
            ),
        }
    }

    fn blank_h3_goal(&self, c: &H3) -> bool {
        self.goal.is_blank()
            && c.dirty
            && self.words.get(c.w).is_zero()
            && self.summary_may_be_all_zero_context(c.r)
            && self.h3s.contains(c)
            && self.pre3l.contains(c)
    }

    fn blank_retl_goal(&self, b: &H2b) -> bool {
        self.goal.is_blank()
            && b.dirty
            && self.summary_may_be_all_zero_context(b.r)
    }

    fn check_blank_h3(&self, c: &H3) -> Result<(), StopReason> {
        if self.blank_h3_goal(c) {
            return Err(StopReason::MayTarget);
        }
        Ok(())
    }

    fn check_blank_retl(&self, b: &H2b) -> Result<(), StopReason> {
        if self.blank_retl_goal(b) {
            return Err(StopReason::MayTarget);
        }
        Ok(())
    }

    fn insert_h3(&mut self, c: H3) -> Result<H3, StopReason> {
        let c = self.canonical_h3(c);
        self.h3s.insert(c);
        self.check_blank_h3(&c)?;
        Ok(c)
    }

    fn insert_h2(&mut self, a: H2) -> H2 {
        let a = self.canonical_h2(a);
        self.h2s.insert(a);
        a
    }

    fn insert_pre3l(&mut self, c: H3) -> Result<H3, StopReason> {
        let c = self.canonical_h3(c);
        self.pre3l.insert(c);
        self.check_blank_h3(&c)?;
        Ok(c)
    }

    fn insert_retl(&mut self, b: H2b) -> Result<H2b, StopReason> {
        let b = self.canonical_h2b(b);
        self.retl.insert(b);
        self.check_blank_retl(&b)?;
        Ok(b)
    }

    fn insert_ret2(&mut self, a: H2, b: H2b) {
        self.ret2
            .insert(self.canonical_h2(a), self.canonical_h2b(b));
    }

    fn insert_ret3(&mut self, a: H3, b: H2b) {
        self.ret3
            .insert(self.canonical_h3(a), self.canonical_h2b(b));
    }

    fn tm_step(
        &mut self,
        w: WordId,
        s: State,
        sgn: i8,
        ctx: StepContext,
    ) -> Result<Option<WordUpdateLemma>, StopReason> {
        self.bump()?;

        let key = StepKey { w, s, sgn, ctx };

        let res = (if let Some(&cached) = self.step_cache.get(&key) {
            cached
        } else {
            let computed = self
                .prog
                .raw_word_update_lemma(
                    self.words.clone_word(key.w),
                    key.s,
                    key.sgn,
                    self.block_step_limit,
                    self.goal,
                    key.ctx,
                    self.mirrored,
                    self.reached,
                )
                .map(|raw| WordUpdateLemma {
                    w1: self.words.intern(raw.w1),
                    s1: raw.s1,
                    is_back: raw.is_back,
                    saw_nonzero: raw.saw_nonzero,
                    hit_blank: raw.hit_blank,
                });
            self.step_cache.insert(key, computed);
            computed
        })
        .ok_or(StopReason::BlockTimeout)?;

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
        let a = self.canonical_h2(*a);
        let H2 { s, dirty, .. } = a;
        let r0 = b.prev;

        let Some(res) = self.tm_step(
            b.w,
            s,
            1,
            self.h2_pop_step_context(dirty, r0),
        )?
        else {
            return Ok(());
        };
        let s1 = res.s1.unwrap();

        let dirty1 = self.canonical_dirty(dirty || res.saw_nonzero);
        if res.is_back {
            let rr = self.dfa_push_id(res.w1, r0)?;
            self.insert_ret2(
                a,
                H2b {
                    s: s1,
                    r: rr,
                    dirty: dirty1,
                },
            );
        } else {
            let c = self.insert_h3(H3 {
                w: res.w1,
                s: s1,
                r: r0,
                dirty: dirty1,
            })?;
            self.pre32.insert(c, a);
        }

        Ok(())
    }

    fn on_h3_back(
        &mut self,
        c: &H3,
        b: &H2b,
    ) -> Result<(), StopReason> {
        let c = self.canonical_h3(*c);
        let b = self.canonical_h2b(*b);
        let H2b {
            s: s0,
            r: r0,
            dirty: dirty_b,
        } = b;

        let Some(res) = self.tm_step(
            c.w,
            s0,
            -1,
            self.h3_back_step_context(&c, &b),
        )?
        else {
            return Ok(());
        };
        let s1 = res.s1.unwrap();

        let dirty1 =
            self.canonical_dirty(c.dirty || dirty_b || res.saw_nonzero);
        if res.is_back {
            let c0 = self.insert_h3(H3 {
                w: res.w1,
                s: s1,
                r: r0,
                dirty: dirty1,
            })?;
            self.pre33.insert(c0, c);
        } else {
            let rr = self.dfa_push_id(res.w1, r0)?;
            self.insert_ret3(
                c,
                H2b {
                    s: s1,
                    r: rr,
                    dirty: dirty1,
                },
            );
        }

        Ok(())
    }

    fn on_retl(&mut self, b: &H2b) -> Result<(), StopReason> {
        let b = self.canonical_h2b(*b);
        let H2b {
            s: s0,
            r: r0,
            dirty,
        } = b;

        let blank = self.words.intern(Word::zero(self.block_len));
        let Some(res) = self.tm_step(
            blank,
            s0,
            -1,
            self.retl_step_context(dirty, r0),
        )?
        else {
            return Ok(());
        };
        let s1 = res.s1.unwrap();

        let dirty1 = self.canonical_dirty(dirty || res.saw_nonzero);
        if res.is_back {
            let c0 = self.insert_h3(H3 {
                w: res.w1,
                s: s1,
                r: r0,
                dirty: dirty1,
            })?;
            self.insert_pre3l(c0)?;
        } else {
            let rr = self.dfa_push_id(res.w1, r0)?;
            self.insert_retl(H2b {
                s: s1,
                r: rr,
                dirty: dirty1,
            })?;
        }

        Ok(())
    }

    fn on_dfa_edge(
        &mut self,
        r: usize,
        e: &DfaEdge,
    ) -> Result<(), StopReason> {
        self.bump()?;
        self.ensure_dfa_capacity(r);
        self.pop[r].push(*e);

        self.scratch_states.clear();
        self.scratch_states.extend(self.r_s[r].iter().copied());
        self.scratch_states.sort_unstable();
        while let Some((s, dirty)) = self.scratch_states.pop() {
            self.on_h2_pop(&H2 { s, r, dirty }, e)?;
        }

        Ok(())
    }

    fn on_h2(&mut self, a: &H2) -> Result<(), StopReason> {
        let a = self.canonical_h2(*a);
        let H2 { s, r, .. } = a;

        self.bump()?;
        self.ensure_dfa_capacity(r);
        self.r_s[r].insert((s, a.dirty));

        self.scratch_edges.clear();
        self.scratch_edges.extend(self.pop[r].iter().copied());
        self.scratch_edges.sort_unstable();
        while let Some(e) = self.scratch_edges.pop() {
            self.on_h2_pop(&a, &e)?;
        }

        Ok(())
    }

    fn on_h3(&mut self, a: H3) {
        let a = self.canonical_h3(a);
        let a0 = self.insert_h2(H2 {
            s: a.s,
            r: a.r,
            dirty: a.dirty,
        });
        self.pre23.insert(a0, a);
    }

    fn on_ret2(&mut self, a: &H2, b: &H2b) -> Result<(), StopReason> {
        self.scratch_h3.clear();
        self.scratch_h3.extend(self.pre23.values(a).copied());
        self.scratch_h3.sort_unstable();
        while let Some(c) = self.scratch_h3.pop() {
            self.on_h3_back(&c, b)?;
        }
        Ok(())
    }

    fn on_ret3(&mut self, a: &H3, b: &H2b) -> Result<(), StopReason> {
        self.scratch_h2.clear();
        self.scratch_h2.extend(self.pre32.values(a).copied());
        self.scratch_h2.sort_unstable();
        while let Some(a0) = self.scratch_h2.pop() {
            self.insert_ret2(a0, *b);
        }

        self.scratch_h3.clear();
        self.scratch_h3.extend(self.pre33.values(a).copied());
        self.scratch_h3.sort_unstable();
        while let Some(a1) = self.scratch_h3.pop() {
            self.insert_ret3(a1, *b);
        }

        if self.pre3l.contains(a) {
            self.insert_retl(*b)?;
        }
        Ok(())
    }

    fn on_pre23(&mut self, a: &H2, c: &H3) -> Result<(), StopReason> {
        self.scratch_h2b.clear();
        self.scratch_h2b.extend(self.ret2.values(a).copied());
        self.scratch_h2b.sort_unstable();
        while let Some(b) = self.scratch_h2b.pop() {
            self.on_h3_back(c, &b)?;
        }
        Ok(())
    }

    fn on_pre32(&mut self, a: &H3, a0: &H2) {
        self.scratch_h2b.clear();
        self.scratch_h2b.extend(self.ret3.values(a).copied());
        self.scratch_h2b.sort_unstable();
        while let Some(b) = self.scratch_h2b.pop() {
            self.insert_ret2(*a0, b);
        }
    }

    fn on_pre33(&mut self, a: &H3, a0: &H3) {
        self.scratch_h2b.clear();
        self.scratch_h2b.extend(self.ret3.values(a).copied());
        self.scratch_h2b.sort_unstable();
        while let Some(b) = self.scratch_h2b.pop() {
            self.insert_ret3(*a0, b);
        }
    }

    fn on_pre3l(&mut self, a: &H3) -> Result<(), StopReason> {
        self.scratch_h2b.clear();
        self.scratch_h2b.extend(self.ret3.values(a).copied());
        self.scratch_h2b.sort_unstable();
        while let Some(b) = self.scratch_h2b.pop() {
            self.insert_retl(b)?;
        }
        Ok(())
    }

    fn run(mut self) -> Result<(), StopReason> {
        let blank = self.words.intern(Word::zero(self.block_len));
        let c0 = H3 {
            w: blank,
            s: 0,
            r: 1,
            dirty: false,
        };
        let c0 = self.insert_h3(c0)?;
        self.insert_pre3l(c0)?;

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
                self.on_pre3l(&a)?;
                continue;
            }
            if let Some((a, b)) = self.ret2.pop_todo() {
                self.on_ret2(&a, &b)?;
                continue;
            }
            if let Some((a, b)) = self.ret3.pop_todo() {
                self.on_ret3(&a, &b)?;
                continue;
            }
            if let Some((a, c)) = self.pre23.pop_todo() {
                self.on_pre23(&a, &c)?;
                continue;
            }
            if let Some((a, a0)) = self.pre32.pop_todo() {
                self.on_pre32(&a, &a0);
                continue;
            }
            if let Some((a, a0)) = self.pre33.pop_todo() {
                self.on_pre33(&a, &a0);
                continue;
            }
            break;
        }

        Ok(())
    }
}

#[expect(clippy::multiple_inherent_impl)]
impl<const STATES: usize, const COLORS: usize> Prog<STATES, COLORS> {
    fn far_reached_params(&self) -> ReachedParams {
        let (max_state, max_color) = self.max_reached();
        ReachedParams {
            states: (max_state as usize + 1).min(STATES),
            colors: (max_color as usize + 1).min(COLORS),
        }
    }

    /// Exact block simulation (matches C++ `WordUpdateLemma::from`).
    ///
    /// - `sgn = +1`: block is oriented in the natural direction
    /// - `sgn = -1`: motion is flipped in the block
    ///
    /// Returns `None` if we exceed `max_steps` or encounter an invalid transition.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss
    )]
    fn raw_word_update_lemma(
        &self,
        w: Word,
        s: State,
        sgn: i8,
        max_steps: usize,
        goal: Goal,
        ctx: StepContext,
        mirrored: bool,
        reached: ReachedParams,
    ) -> Option<RawWordUpdateLemma> {
        debug_assert!(sgn == 1 || sgn == -1);
        let len = w.len() as i32;
        let mut w1 = w;
        let mut nonzero_count =
            w1.cells.iter().filter(|&&x| x != 0).count();
        let mut saw_nonzero = nonzero_count != 0;
        let mut s1 = s;
        let mut pos: i32 = 0;

        for _ in 0..max_steps {
            let input = w1.get(pos as usize);
            if input as usize >= reached.colors {
                return None;
            }
            if s1 as usize >= reached.states {
                return None;
            }

            // Lookup transition; missing transition means HALT.
            let slot: Slot = (s1, input);
            let instr: Option<&Instr> = self.get(&slot);

            let Some(&(out_color, shift_right, next_state)) = instr
            else {
                return Some(RawWordUpdateLemma::exit_oriented(
                    w1,
                    None,
                    false,
                    saw_nonzero,
                    false,
                ));
            };

            if out_color as usize >= reached.colors {
                return None;
            }
            if next_state as usize >= reached.states {
                return None;
            }

            let dir: i32 = if shift_right { 1 } else { -1 };
            let dir = if mirrored { -dir } else { dir };
            let block_dir = dir * i32::from(sgn);
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
                    return Some(RawWordUpdateLemma::exit_oriented(
                        w1,
                        Some(s1),
                        false,
                        saw_nonzero,
                        true,
                    ));
                }
            }

            if input != out_color {
                if input != 0 {
                    nonzero_count -= 1;
                }
                if out_color != 0 {
                    nonzero_count += 1;
                }
                w1.set(pos as usize, out_color);
            }
            s1 = next_state;

            saw_nonzero |= nonzero_count != 0;
            if goal.is_blank() && nonzero_count == 0 {
                let blank_hit = match ctx {
                    StepContext::Blank {
                        dirty_before,
                        context_may_be_all_zero,
                    } => {
                        context_may_be_all_zero
                            && (dirty_before || saw_nonzero)
                    },
                    _ => false,
                };

                if blank_hit {
                    return Some(RawWordUpdateLemma::exit_oriented(
                        w1,
                        Some(s1),
                        false,
                        saw_nonzero,
                        true,
                    ));
                }
            }

            pos += block_dir;

            if pos < 0 || pos >= len {
                return Some(RawWordUpdateLemma::exit_oriented(
                    w1,
                    Some(s1),
                    pos < 0,
                    saw_nonzero,
                    false,
                ));
            }
        }

        None
    }

    fn far_decider<S: Summary>(
        &self,
        params: FarRunParams,
    ) -> Result<FarDecider<'_, STATES, COLORS, S>, StopReason> {
        let idr = vec![S::new()];

        let decider = FarDecider {
            prog: self,
            goal: params.goal,
            mirrored: params.mirrored,
            reached: params.reached,
            block_len: params.block_len,
            max_work: params.max_work.max(1),
            block_step_limit: params.block_step_limit.max(1),

            words: WordInterner::new(),

            work: 0,

            step_cache: Map::new(),

            id: Map::new(),
            idr,

            pop: Vec::new(),
            push: Map::new(),
            new_pops: Vec::new(),

            ret2: TodoMap::new(),
            ret3: TodoMap::new(),
            pre23: TodoMap::new(),
            pre32: TodoMap::new(),
            pre33: TodoMap::new(),

            pre3l: TodoSet::new(),
            retl: TodoSet::new(),
            h3s: TodoSet::new(),
            h2s: TodoSet::new(),

            r_s: Vec::new(),
            zero_context: Vec::new(),
            zero_push: Vec::new(),

            scratch_states: Vec::new(),
            scratch_edges: Vec::new(),
            scratch_h2: Vec::new(),
            scratch_h2b: Vec::new(),
            scratch_h3: Vec::new(),
        };

        decider.with_init_dfa()
    }

    fn far_decide_with<S: Summary>(
        &self,
        params: FarRunParams,
    ) -> bool {
        if params.block_len == 0 {
            return false;
        }

        let Ok(decider) = self.far_decider::<S>(params) else {
            return false;
        };

        decider.run().is_ok()
    }

    fn far_sweep_fast(&self, knob: usize, goal: Goal) -> bool {
        self.far_sweep_with(goal, knob, Self::far_decide_fast)
    }

    fn far_sweep_slow(&self, knob: usize, goal: Goal) -> bool {
        self.far_sweep_with(goal, knob, Self::far_decide_slow)
    }

    fn far_sweep_with(
        &self,
        goal: Goal,
        knob: usize,
        decide: fn(&Self, FarRunParams) -> bool,
    ) -> bool {
        let knob = knob.max(FAR_KNOB_MIN);
        let eff = effort_factor(knob);
        let reached = self.far_reached_params();

        let cap_by_colors = if reached.colors <= 2 {
            FAR_BLOCK_LEN_CAP_COLORS_2
        } else if reached.colors <= 4 {
            FAR_BLOCK_LEN_CAP_COLORS_3_4
        } else {
            FAR_BLOCK_LEN_CAP_COLORS_5_8
        };

        let max_block_len =
            knob.min(cap_by_colors).min(FAR_BLOCK_LEN_HARD_CAP);

        for block_len in 1..=max_block_len {
            let max_work = FAR_WORK_PER_LEN
                .saturating_mul(block_len)
                .saturating_mul(eff);

            let block_step_limit = FAR_STEP_PER_LEN
                .saturating_mul(block_len)
                .saturating_mul(eff);

            for mirrored in [false, true] {
                if decide(
                    self,
                    FarRunParams {
                        block_len,
                        max_work,
                        block_step_limit,
                        goal,
                        mirrored,
                        reached,
                    },
                ) {
                    return true;
                }
            }
        }

        false
    }

    fn far_decide_fast(&self, params: FarRunParams) -> bool {
        self.far_decide_with::<Ng1Summary>(params)
            || self.far_decide_with::<NgSummary<
                FAR_NG_TAIL_H_SMALL,
                FAR_NG_POS_MOD_2,
            >>(params)
            || self.far_decide_with::<CpsLruSummary>(params)
            || self.far_decide_with::<RwlModSummary>(params)
    }

    fn far_decide_slow(&self, params: FarRunParams) -> bool {
        self.far_decide_with::<NgSummary<
            FAR_NG_TAIL_H_MED,
            FAR_NG_POS_MOD_3,
        >>(params)
            || self.far_decide_with::<NgSetSummary>(params)
            || self.far_decide_with::<LruPairSummary>(params)
            || self.far_decide_with::<SetPairSummary>(params)
            || self.far_decide_with::<RngsModSummary>(params)
            || self.far_decide_with::<RsModSummary>(params)
    }

    fn direct_far_cant_target(&self, goal: Goal) -> bool {
        let reached = self.far_reached_params();
        let ctrl_states = Self::direct_far_ctrl_states(goal, reached);
        if ctrl_states == 0 || reached.colors == 0 {
            return false;
        }

        let max_by_nfa = DIRECT_FAR_MAX_NFA_STATES
            .saturating_sub(DIRECT_FAR_TARGET_STATES)
            / ctrl_states;
        let max_by_entries =
            DIRECT_FAR_MAX_DFA_ENTRIES / reached.colors;
        let max_dfa_states = DIRECT_FAR_MAX_DFA_STATES
            .min(max_by_nfa)
            .min(max_by_entries);

        if max_dfa_states == 0
            || !self.direct_far_valid_program(reached)
        {
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

    fn direct_far_valid_program(&self, reached: ReachedParams) -> bool {
        for state in 0..reached.states {
            for color in 0..reached.colors {
                #[expect(clippy::cast_possible_truncation)]
                let slot: Slot = (state as State, color as Color);
                #[expect(clippy::collapsible_if)]
                if let Some(&(write, _, next_state)) = self.get(&slot) {
                    if write as usize >= reached.colors
                        || next_state as usize >= reached.states
                    {
                        return false;
                    }
                }
            }
        }
        true
    }

    const fn direct_far_ctrl_states(
        goal: Goal,
        reached: ReachedParams,
    ) -> usize {
        match goal {
            Goal::Blank => reached.states * 2,
            Goal::Halt | Goal::Spinout => reached.states,
        }
    }

    const fn direct_far_start_ctrl(_goal: Goal) -> usize {
        0
    }

    const fn direct_far_ctrl_tm_state(
        goal: Goal,
        ctrl: usize,
        reached: ReachedParams,
    ) -> usize {
        match goal {
            Goal::Blank => ctrl % reached.states,
            Goal::Halt | Goal::Spinout => ctrl,
        }
    }

    const fn direct_far_ctrl_dirty(
        goal: Goal,
        ctrl: usize,
        reached: ReachedParams,
    ) -> bool {
        match goal {
            Goal::Blank => ctrl >= reached.states,
            Goal::Halt | Goal::Spinout => false,
        }
    }

    fn direct_far_next_ctrl(
        goal: Goal,
        ctrl: usize,
        read_symbol: usize,
        write_symbol: usize,
        next_state: usize,
        reached: ReachedParams,
    ) -> usize {
        match goal {
            Goal::Blank => {
                let dirty =
                    Self::direct_far_ctrl_dirty(goal, ctrl, reached)
                        || read_symbol != 0
                        || write_symbol != 0;
                next_state + usize::from(dirty) * reached.states
            },
            Goal::Halt | Goal::Spinout => next_state,
        }
    }

    fn direct_far_decide_exact(
        &self,
        goal: Goal,
        reached: ReachedParams,
        dfa_states: usize,
        fuel: &mut usize,
    ) -> bool {
        let ctrl_states = Self::direct_far_ctrl_states(goal, reached);
        let nfa_states = ctrl_states
            .saturating_mul(dfa_states)
            .saturating_add(DIRECT_FAR_TARGET_STATES);
        if dfa_states == 0
            || nfa_states == 0
            || nfa_states > DIRECT_FAR_MAX_NFA_STATES
        {
            return false;
        }

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

        let ctrl_states = Self::direct_far_ctrl_states(goal, reached);
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
        self.direct_far_search(params, &mut dfa, 0, 0, &r, a, fuel)
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
                // A dirty all-zero global tape is the blank target.  DFA state
                // 0 is the all-zero left context because transition (0, 0) is
                // fixed to 0 by the canonical direct-DFA search.
                for state in 0..params.reached.states {
                    let dirty_ctrl = state + params.reached.states;
                    let src = direct_far_idx(
                        0,
                        dirty_ctrl,
                        params.ctrl_states,
                    );
                    r[0][src] |= zero_bit;
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

    fn direct_far_search(
        &self,
        params: DirectFarParams,
        dfa: &mut [usize],
        entry: usize,
        max_seen: usize,
        r: &[Vec<u128>],
        a: u128,
        fuel: &mut usize,
    ) -> bool {
        if *fuel == 0 {
            return false;
        }

        let dfa_entries = params.reached.colors * params.dfa_states;
        if entry == dfa_entries {
            return max_seen + 1 == params.dfa_states;
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
            let mut next_r = r.to_vec();
            let mut next_a = a;
            if !self.direct_far_extend_nfa(
                params,
                dfa,
                &mut next_r,
                &mut next_a,
                entry,
            ) {
                continue;
            }

            if self.direct_far_search(
                params,
                dfa,
                entry + 1,
                max_seen.max(to_state),
                &next_r,
                next_a,
                fuel,
            ) {
                return true;
            }
        }

        false
    }

    fn direct_far_extend_nfa(
        &self,
        params: DirectFarParams,
        dfa: &[usize],
        r: &mut [Vec<u128>],
        a: &mut u128,
        entry: usize,
    ) -> bool {
        let dfa_src = entry / params.reached.colors;
        let write_symbol = entry % params.reached.colors;
        let dfa_dst = dfa[entry];
        let fixed_entries = entry + 1;

        // Right-rule for the one newly fixed DFA transition.
        for ctrl in 0..params.ctrl_states {
            let state = Self::direct_far_ctrl_tm_state(
                params.goal,
                ctrl,
                params.reached,
            );
            for read_symbol in 0..params.reached.colors {
                #[expect(clippy::cast_possible_truncation)]
                let slot: Slot = (state as State, read_symbol as Color);
                let Some(&(write, shift_right, next_state)) =
                    self.get(&slot)
                else {
                    continue;
                };

                let written = write as usize;
                if direct_far_move_code(shift_right) == params.direction
                    && written == write_symbol
                {
                    let next_ctrl = Self::direct_far_next_ctrl(
                        params.goal,
                        ctrl,
                        read_symbol,
                        written,
                        next_state as usize,
                        params.reached,
                    );
                    let src = direct_far_idx(
                        dfa_src,
                        ctrl,
                        params.ctrl_states,
                    );
                    let dst = direct_far_idx(
                        dfa_dst,
                        next_ctrl,
                        params.ctrl_states,
                    );
                    r[read_symbol][src] |= direct_far_bit(dst);
                }
            }
        }

        // Left-rule closure over all currently fixed DFA transitions.
        loop {
            let mut changed = false;

            for ctrl in 0..params.ctrl_states {
                let state = Self::direct_far_ctrl_tm_state(
                    params.goal,
                    ctrl,
                    params.reached,
                );
                for read_symbol in 0..params.reached.colors {
                    #[expect(clippy::cast_possible_truncation)]
                    let slot: Slot =
                        (state as State, read_symbol as Color);
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

                    let next_ctrl = Self::direct_far_next_ctrl(
                        params.goal,
                        ctrl,
                        read_symbol,
                        written,
                        next_state as usize,
                        params.reached,
                    );
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

        let mut prev_accept = *a;
        loop {
            let next_accept = direct_far_matrix_times_vec(
                &r[0],
                prev_accept,
                params.nfa_states,
            );
            *a = next_accept;
            if next_accept == prev_accept {
                break;
            }
            prev_accept = next_accept;
        }

        // If the initial all-zero configuration is already accepted by this
        // partial DFA/NFA lower bound, no completion can prove the target absent.
        let start_idx = direct_far_idx(
            0,
            Self::direct_far_start_ctrl(params.goal),
            params.ctrl_states,
        );
        (r[0][start_idx] & *a) == 0
    }

    fn mitm_cant_halt(&self) -> bool {
        self.mitm_cant_target(Goal::Halt)
    }

    fn mitm_cant_blank(&self) -> bool {
        self.mitm_cant_target(Goal::Blank)
    }

    fn mitm_cant_spinout(&self) -> bool {
        self.mitm_cant_target(Goal::Spinout)
    }

    fn mitm_cant_target(&self, goal: Goal) -> bool {
        let reached = self.far_reached_params();

        // Fast, boolean-only port of Iijil1/MITMWFAR's search path:
        // enumerate closed MITM-DFA skeletons, then try one (+1,-1) WFA
        // weight pair, deriving and verifying the accept set in memory.
        // Cheap passes first.  These preserve the same eventual prover power
        // as the final pass, but avoid paying for memory expansion on easy TMs.
        for added_memory in [0_usize, 1] {
            for dfa_transitions in 2..=MITM_MAX_TRANSITIONS {
                if self.mitm_decide_exact(
                    goal,
                    dfa_transitions,
                    MITM_MAX_WEIGHT_PAIRS,
                    added_memory,
                    reached,
                ) {
                    return true;
                }
            }
        }

        false
    }

    fn mitm_decide_exact(
        &self,
        goal: Goal,
        dfa_transitions: usize,
        max_weight_pairs: usize,
        added_memory: usize,
        reached: ReachedParams,
    ) -> bool {
        let mut left = MitmWfa::new(reached.colors);
        let mut right = MitmWfa::new(reached.colors);
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
                added_memory,
                reached,
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
        match self.mitm_find_closure_break(
            goal,
            left,
            right,
            params.reached,
        ) {
            None => {
                current_transitions == params.goal_transitions
                    && self.mitm_recurse_weights(
                        goal,
                        left,
                        right,
                        &self.mitm_reachable_weight_slots(
                            goal,
                            left,
                            right,
                            params.reached,
                        ),
                        0,
                        params.max_weight_pairs,
                        params.added_memory,
                        params.reached,
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
        reached: ReachedParams,
    ) -> Option<(MitmSide, usize, usize)> {
        let left_rev = left.rev_edges();
        let right_rev = right.rev_edges();
        let start = MitmConfig::start();
        let mut seen = Set::new();
        let mut todo = vec![start];
        let mut nexts = Vec::new();
        seen.insert(start);

        while let Some(cur) = todo.pop() {
            if cur.st >= reached.states || cur.co >= reached.colors {
                continue;
            }
            #[expect(clippy::cast_possible_truncation)]
            let Some(&(write, _shift, _)) =
                self.get(&(cur.st as State, cur.co as Color))
            else {
                continue;
            };
            let write = write as usize;

            nexts.clear();
            self.mitm_next_configs_into(
                goal, cur, left, right, &left_rev, &right_rev,
                &mut nexts, reached,
            );
            #[expect(clippy::iter_with_drain)]
            for next in nexts.drain(..) {
                let cfg = next.config;
                if seen.contains(&cfg) {
                    continue;
                }
                if cfg.left == MITM_DEAD {
                    return Some((MitmSide::Left, cur.left, write));
                }
                if cfg.right == MITM_DEAD {
                    return Some((MitmSide::Right, cur.right, write));
                }
                seen.insert(cfg);
                todo.push(cfg);
            }
        }

        None
    }

    fn mitm_reachable_weight_slots(
        &self,
        goal: Goal,
        left: &MitmWfa,
        right: &MitmWfa,
        reached: ReachedParams,
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
            if cur.st >= reached.states
                || cur.co >= reached.colors
                || cur.left >= left.states
                || cur.right >= right.states
            {
                continue;
            }
            #[expect(clippy::cast_possible_truncation)]
            let Some(&(write, shift, next_st)) =
                self.get(&(cur.st as State, cur.co as Color))
            else {
                continue;
            };

            let write = write as usize;
            let next_st = next_st as usize;
            let next_dirty = goal.is_blank()
                && (cur.dirty || cur.co != 0 || write != 0);

            if shift {
                useful_left[cur.left][write] = true;
                let (new_left, _) = left.trans[cur.left][write];
                for edge in &right_rev[cur.right] {
                    useful_right[edge.from][edge.symbol] = true;
                    let next = MitmConfig {
                        st: next_st,
                        co: edge.symbol,
                        left: new_left,
                        right: edge.from,
                        dirty: next_dirty,
                    };
                    if next.left != MITM_DEAD
                        && next.right != MITM_DEAD
                        && seen.insert(next)
                    {
                        todo.push(next);
                    }
                }
            } else {
                useful_right[cur.right][write] = true;
                let (new_right, _) = right.trans[cur.right][write];
                for edge in &left_rev[cur.left] {
                    useful_left[edge.from][edge.symbol] = true;
                    let next = MitmConfig {
                        st: next_st,
                        co: edge.symbol,
                        left: edge.from,
                        right: new_right,
                        dirty: next_dirty,
                    };
                    if next.left != MITM_DEAD
                        && next.right != MITM_DEAD
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
        added_memory: usize,
        reached: ReachedParams,
    ) -> bool {
        if self.mitm_check_weight_candidate(
            goal,
            left,
            right,
            added_memory,
            reached,
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
                        added_memory,
                        reached,
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

    fn mitm_check_weight_candidate(
        &self,
        goal: Goal,
        left: &MitmWfa,
        right: &MitmWfa,
        added_memory: usize,
        reached: ReachedParams,
    ) -> bool {
        if added_memory == 0 {
            return self.mitm_check_weight_candidate_exact(
                goal, left, right, reached,
            );
        }

        let mut try_left = left.clone();
        let mut try_right = right.clone();
        for _ in 0..added_memory {
            try_left = try_left.with_memory();
            try_right = try_right.with_memory();
        }
        self.mitm_check_weight_candidate_exact(
            goal, &try_left, &try_right, reached,
        )
    }

    fn mitm_check_weight_candidate_exact(
        &self,
        goal: Goal,
        left: &MitmWfa,
        right: &MitmWfa,
        reached: ReachedParams,
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
            reached,
        )
    }

    fn mitm_build_accept_set_exact(
        &self,
        goal: Goal,
        left: &MitmWfa,
        right: &MitmWfa,
        left_rev: &MitmRev,
        right_rev: &MitmRev,
        left_special: &MitmSpecial,
        right_special: &MitmSpecial,
        reached: ReachedParams,
    ) -> bool {
        let start = MitmConfig::start();
        let start_bounds = MitmBounds {
            lo: Some(0),
            hi: Some(0),
        };
        if !self.mitm_config_allowed(
            goal,
            &start,
            &start_bounds,
            reached,
        ) {
            return false;
        }

        let mut accept = MitmAccept::new();
        let mut todo = vec![start];
        let mut nexts = Vec::new();
        accept.insert(start, start_bounds);

        while let Some(cur) = todo.pop() {
            let cur_bounds = accept[&cur];
            nexts.clear();
            self.mitm_next_configs_into(
                goal, cur, left, right, left_rev, right_rev,
                &mut nexts, reached,
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

                let Some((cfg, bounds)) =
                    mitm_accept_insert_or_widen(&next, &mut accept)
                else {
                    continue;
                };

                if !self
                    .mitm_config_allowed(goal, &cfg, &bounds, reached)
                {
                    return false;
                }
                todo.push(cfg);
            }
        }

        true
    }

    #[expect(clippy::cast_possible_truncation)]
    fn mitm_config_allowed(
        &self,
        goal: Goal,
        cfg: &MitmConfig,
        bounds: &MitmBounds,
        reached: ReachedParams,
    ) -> bool {
        match goal {
            Goal::Halt => {
                cfg.st < reached.states
                    && cfg.co < reached.colors
                    && self
                        .get(&(cfg.st as State, cfg.co as Color))
                        .is_some()
            },
            Goal::Blank => {
                cfg.st < reached.states
                    && cfg.co < reached.colors
                    && !mitm_blank_config_possible(cfg, bounds)
            },
            Goal::Spinout => {
                !self.mitm_spinout_config_possible(cfg, reached)
            },
        }
    }

    fn mitm_spinout_config_possible(
        &self,
        cfg: &MitmConfig,
        reached: ReachedParams,
    ) -> bool {
        if cfg.st >= reached.states || cfg.co != 0 {
            return false;
        }
        #[expect(clippy::cast_possible_truncation)]
        let Some(&(_, shift, trans)) = self.get(&(cfg.st as State, 0))
        else {
            return false;
        };

        if trans as usize != cfg.st {
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
        reached: ReachedParams,
    ) {
        if old.st >= reached.states || old.co >= reached.colors {
            return;
        }
        #[expect(clippy::cast_possible_truncation)]
        let Some(&(write, shift, next_st)) =
            self.get(&(old.st as State, old.co as Color))
        else {
            return;
        };

        let write = write as usize;
        let next_st = next_st as usize;
        let next_dirty =
            goal.is_blank() && (old.dirty || old.co != 0 || write != 0);

        if shift {
            // Move right: the written symbol joins the left half; the old right
            // predecessor supplies the next scanned symbol.
            let (new_left, left_weight) = left.trans[old.left][write];
            for edge in &right_rev[old.right] {
                out.push(MitmNext {
                    config: MitmConfig {
                        st: next_st,
                        co: edge.symbol,
                        left: new_left,
                        right: edge.from,
                        dirty: next_dirty,
                    },
                    weight: left_weight - edge.weight,
                });
            }
        } else {
            // Move left: symmetric case.
            let (new_right, right_weight) =
                right.trans[old.right][write];
            for edge in &left_rev[old.left] {
                out.push(MitmNext {
                    config: MitmConfig {
                        st: next_st,
                        co: edge.symbol,
                        left: edge.from,
                        right: new_right,
                        dirty: next_dirty,
                    },
                    weight: right_weight - edge.weight,
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
    from: usize,
    symbol: usize,
    weight: i32,
}

type MitmRev = Vec<Vec<MitmRevEdge>>;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct MitmConfig {
    st: usize,
    co: usize,
    left: usize,
    right: usize,
    dirty: bool,
}

impl MitmConfig {
    const fn start() -> Self {
        Self {
            st: 0,
            co: 0,
            left: 0,
            right: 0,
            dirty: false,
        }
    }
}

#[derive(Clone, Copy)]
struct MitmNext {
    config: MitmConfig,
    weight: i32,
}

#[derive(Clone, Copy)]
struct MitmSearchParams {
    goal_transitions: usize,
    max_weight_pairs: usize,
    added_memory: usize,
    reached: ReachedParams,
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

    let hard_lo = left_special.nonneg[cfg.left]
        && right_special.nonneg[cfg.right];
    if hard_lo && lo.is_none_or(|x| x < 0) {
        lo = Some(0);
    }

    let hard_hi = left_special.nonpos[cfg.left]
        && right_special.nonpos[cfg.right];
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

fn mitm_blank_config_possible(
    cfg: &MitmConfig,
    bounds: &MitmBounds,
) -> bool {
    cfg.dirty
        && cfg.co == 0
        && cfg.left == 0
        && cfg.right == 0
        && bounds.contains_zero()
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
                rev[to].push(MitmRevEdge {
                    from,
                    symbol,
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
