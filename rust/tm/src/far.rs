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

/// NG summary window: how many most-recent pushed blocks to remember.
const FAR_NG_WINDOW: usize = 3;

/// NG summary tail: how many earliest pushed blocks to remember (usually 0).
const FAR_NG_TAIL: usize = 0;

/// NG summary modulus (usually 1).
const FAR_NG_POS_MOD: usize = 1;

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
        far_sweep_fast::<STATES, COLORS>(self, knob, Goal::Halt)
            || mitm_cant_halt::<STATES, COLORS>(self)
            || far_sweep_slow::<STATES, COLORS>(self, knob, Goal::Halt)
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
        far_sweep_fast::<STATES, COLORS>(self, knob, Goal::Blank)
            || mitm_cant_blank::<STATES, COLORS>(self)
            || far_sweep_slow::<STATES, COLORS>(self, knob, Goal::Blank)
    }

    pub fn far_cant_spinout(&self, _knob: usize) -> bool {
        unimplemented!()
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
    s1: i16,
    is_back: bool,
    saw_nonzero: bool,
    hit_blank: bool,
    #[expect(dead_code)]
    n_step: usize,
}

/// Non-interned result used only while computing a cache miss.
#[derive(Clone, Debug)]
struct RawWordUpdateLemma {
    w1: Word,
    s1: i16,
    is_back: bool,
    saw_nonzero: bool,
    hit_blank: bool,
    n_step: usize,
}

impl RawWordUpdateLemma {
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
    fn from_prog<const STATES: usize, const COLORS: usize>(
        prog: &Prog<STATES, COLORS>,
        w: Word,
        s: i16,
        sgn: i8,
        max_steps: usize,
        goal: Goal,
        dirty_before: bool,
        context_may_be_all_zero: bool,
    ) -> Option<Self> {
        debug_assert!(sgn == 1 || sgn == -1);
        if s < 0 {
            return None;
        }
        if COLORS < 2 {
            return None;
        }

        let len = w.len() as i32;
        let mut w1 = w;
        let mut nonzero_count =
            w1.cells.iter().filter(|&&x| x != 0).count();
        let mut saw_nonzero = nonzero_count != 0;
        let mut s1 = s;
        let mut pos: i32 = 0;

        for t in 0..max_steps {
            let input = w1.get(pos as usize);
            if input as usize >= COLORS {
                return None;
            }
            let s_usize = s1 as usize;
            if s_usize >= STATES {
                return None;
            }

            // Lookup transition; missing transition means HALT.
            let slot: Slot = (s1 as State, input);
            let instr: Option<&Instr> = prog.get(&slot);

            let Some(&(out_color, shift_right, next_state)) = instr
            else {
                return Some(Self {
                    w1,
                    s1: -1,
                    n_step: t + 1,
                    is_back: false,
                    saw_nonzero,
                    hit_blank: false,
                });
            };

            if out_color as usize >= COLORS {
                return None;
            }
            if next_state as usize >= STATES {
                return None;
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
            s1 = i16::from(next_state);

            saw_nonzero |= nonzero_count != 0;
            if goal.is_blank()
                && context_may_be_all_zero
                && (dirty_before || saw_nonzero)
                && nonzero_count == 0
            {
                return Some(Self {
                    w1,
                    s1,
                    n_step: t + 1,
                    is_back: false,
                    saw_nonzero,
                    hit_blank: true,
                });
            }

            let dir: i32 = if shift_right { 1 } else { -1 };
            pos += dir * i32::from(sgn);

            if pos < 0 || pos >= len {
                return Some(Self {
                    w1,
                    s1,
                    n_step: t + 1,
                    is_back: pos < 0,
                    saw_nonzero,
                    hit_blank: false,
                });
            }
        }

        None
    }

    /// C++ `WordUpdateLemma::from_v2`:
    /// if we exit forward (not back) and did not halt, reverse the block.
    fn from_prog_v2<const STATES: usize, const COLORS: usize>(
        prog: &Prog<STATES, COLORS>,
        w: Word,
        s: i16,
        sgn: i8,
        max_steps: usize,
        goal: Goal,
        dirty_before: bool,
        context_may_be_all_zero: bool,
    ) -> Option<Self> {
        let mut res = Self::from_prog(
            prog,
            w,
            s,
            sgn,
            max_steps,
            goal,
            dirty_before,
            context_may_be_all_zero,
        )?;
        if res.s1 >= 0 && !res.is_back {
            res.w1 = res.w1.reverse();
        }
        Some(res)
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

/// NG stack summary: keep bounded history (window + tail + mod counter).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct NgSummary {
    q: Vec<WordId>,
    q0: Vec<WordId>,
    modu: usize,
}

impl Summary for NgSummary {
    fn new() -> Self {
        Self {
            q: Vec::new(),
            q0: Vec::new(),
            modu: 0,
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

        if FAR_NG_WINDOW > 0 && self.q.len() == FAR_NG_WINDOW {
            self.q.remove(0);
        }
        self.q.push(w);

        #[expect(clippy::absurd_extreme_comparisons)]
        if self.q0.len() < FAR_NG_TAIL {
            self.q0.push(w);
        }

        self.modu = (self.modu + 1) % FAR_NG_POS_MOD.max(1);
        Ok(())
    }

    fn may_be_all_zero_context(&self, words: &WordInterner) -> bool {
        self.q.iter().all(|&w| words.get(w).is_zero())
            && self.q0.iter().all(|&w| words.get(w).is_zero())
    }
}

impl PartialOrd for NgSummary {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NgSummary {
    fn cmp(&self, other: &Self) -> Ordering {
        self.modu
            .cmp(&other.modu)
            .then_with(|| self.q.cmp(&other.q))
            .then_with(|| self.q0.cmp(&other.q0))
    }
}

/// C++ FAR::NG1 default parameters.
const FAR_NG1_N: usize = 3;

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

/// C++ FAR::RWL_mod defaults.
const FAR_RWL_LEN_H: usize = 8;
const FAR_RWL_LEN_H_TAIL: usize = 0;
const FAR_RWL_MNC: usize = 2;
const FAR_RWL_MOD: usize = 1;

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

/// C++ FAR::CPS_LRU defaults.
const FAR_CPS_LRU_LEN_H: usize = 8;
const FAR_CPS_LRU_LEN_H_NO_LRU: usize = 2;

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

/// C++ FAR::RNGS_mod defaults.
const FAR_RNGS_NG_N: usize = 4;
const FAR_RNGS_LEN_H: usize = 8;
const FAR_RNGS_MNC: usize = 2;
const FAR_RNGS_MOD: usize = 1;
const FAR_RNGS_BS_N: usize = 0;

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

/// C++ FAR::RS_mod defaults.
const FAR_RS_NG_N: usize = 4;
const FAR_RS_LEN_H: usize = 8;
const FAR_RS_MNC: usize = 2;
const FAR_RS_MOD: usize = 1;
const FAR_RS_STRICT: bool = true;

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
    s: i16,
    r: usize,
    dirty: bool,
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct H2b {
    s: i16,
    r: usize,
    dirty: bool,
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
struct H3 {
    w: WordId,
    s: i16,
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

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
struct StepKey {
    w: WordId,
    s: i16,
    sgn: i8,
    dirty_before: bool,
    context_may_be_all_zero: bool,
}

/// FAR decider with a pluggable DFA history summary.
struct FarDecider<
    'a,
    S: Summary,
    const STATES: usize,
    const COLORS: usize,
> {
    prog: &'a Prog<STATES, COLORS>,
    goal: Goal,

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
    r_s: Vec<Set<(i16, bool)>>,

    // Reusable buffers for relation propagation. These avoid allocating short
    // temporary Vecs just to break immutable borrows before mutating self.
    scratch_states: Vec<(i16, bool)>,
    scratch_edges: Vec<DfaEdge>,
    scratch_h2: Vec<H2>,
    scratch_h2b: Vec<H2b>,
    scratch_h3: Vec<H3>,
}

impl<'a, S: Summary, const STATES: usize, const COLORS: usize>
    FarDecider<'a, S, STATES, COLORS>
{
    fn new(
        prog: &'a Prog<STATES, COLORS>,
        block_len: usize,
        max_work: usize,
        block_step_limit: usize,
        goal: Goal,
    ) -> Result<Self, StopReason> {
        let idr = vec![S::new()];

        let this = Self {
            prog,
            goal,
            block_len,
            max_work: max_work.max(1),
            block_step_limit: block_step_limit.max(1),

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

            scratch_states: Vec::new(),
            scratch_edges: Vec::new(),
            scratch_h2: Vec::new(),
            scratch_h2b: Vec::new(),
            scratch_h3: Vec::new(),
        };

        this.with_init_dfa()
    }

    fn with_init_dfa(mut self) -> Result<Self, StopReason> {
        self.ensure_dfa_capacity(0);

        let id0 = self.get_id(S::new());
        debug_assert_eq!(id0, 1);

        let blank = Word::zero(self.block_len);
        let id1 = self.dfa_push(blank, id0)?;
        debug_assert_eq!(id1, id0);

        Ok(self)
    }

    const fn bump(&mut self) -> Result<(), StopReason> {
        self.work = self.work.saturating_add(1);
        if self.work >= self.max_work {
            return Err(StopReason::WorkLimit);
        }
        Ok(())
    }

    fn ensure_dfa_capacity(&mut self, id: usize) {
        if self.pop.len() <= id {
            self.pop.resize_with(id + 1, Vec::new);
        }
        if self.r_s.len() <= id {
            self.r_s.resize_with(id + 1, Set::new);
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
        Ok(to)
    }

    fn summary_may_be_all_zero_context(&self, r: usize) -> bool {
        self.idr
            .get(r)
            .is_some_and(|st| st.may_be_all_zero_context(&self.words))
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

    fn insert_h3(&mut self, c: &H3) -> Result<(), StopReason> {
        self.h3s.insert(*c);
        self.check_blank_h3(c)
    }

    fn insert_h2(&mut self, a: H2) {
        self.h2s.insert(a);
    }

    fn insert_pre3l(&mut self, c: &H3) -> Result<(), StopReason> {
        self.pre3l.insert(*c);
        self.check_blank_h3(c)
    }

    fn insert_retl(&mut self, b: &H2b) -> Result<(), StopReason> {
        self.retl.insert(*b);
        self.check_blank_retl(b)
    }

    fn insert_ret2(&mut self, a: H2, b: H2b) {
        self.ret2.insert(a, b);
    }

    fn insert_ret3(&mut self, a: H3, b: H2b) {
        self.ret3.insert(a, b);
    }

    fn tm_step(
        &mut self,
        w: WordId,
        s: i16,
        sgn: i8,
        dirty_before: bool,
        context_may_be_all_zero: bool,
    ) -> Result<Option<WordUpdateLemma>, StopReason> {
        self.bump()?;

        let key = StepKey {
            w,
            s,
            sgn,
            dirty_before,
            context_may_be_all_zero,
        };

        let res = (if let Some(&cached) = self.step_cache.get(&key) {
            cached
        } else {
            let computed =
                RawWordUpdateLemma::from_prog_v2::<STATES, COLORS>(
                    self.prog,
                    self.words.clone_word(key.w),
                    key.s,
                    key.sgn,
                    self.block_step_limit,
                    self.goal,
                    key.dirty_before,
                    key.context_may_be_all_zero,
                )
                .map(|raw| WordUpdateLemma {
                    w1: self.words.intern(raw.w1),
                    s1: raw.s1,
                    is_back: raw.is_back,
                    saw_nonzero: raw.saw_nonzero,
                    hit_blank: raw.hit_blank,
                    n_step: raw.n_step,
                });
            self.step_cache.insert(key, computed);
            computed
        })
        .ok_or(StopReason::BlockTimeout)?;

        if res.hit_blank {
            return Err(StopReason::MayTarget);
        }

        if res.s1 == -1 {
            if self.goal.is_halt() {
                return Err(StopReason::MayTarget);
            }
            return Ok(None);
        }

        Ok(Some(res))
    }

    fn on_h2_pop(
        &mut self,
        a: &H2,
        b: &DfaEdge,
    ) -> Result<(), StopReason> {
        let H2 { s, dirty, .. } = *a;
        let r0 = b.prev;

        let context_zero = self.summary_may_be_all_zero_context(r0);
        let Some(res) = self.tm_step(b.w, s, 1, dirty, context_zero)?
        else {
            return Ok(());
        };

        let dirty1 = dirty || res.saw_nonzero;
        if res.is_back {
            let rr = self.dfa_push_id(res.w1, r0)?;
            self.insert_ret2(
                *a,
                H2b {
                    s: res.s1,
                    r: rr,
                    dirty: dirty1,
                },
            );
        } else {
            let c = H3 {
                w: res.w1,
                s: res.s1,
                r: r0,
                dirty: dirty1,
            };
            self.insert_h3(&c)?;
            self.pre32.insert(c, *a);
        }

        Ok(())
    }

    fn on_h3_back(
        &mut self,
        c: &H3,
        b: &H2b,
    ) -> Result<(), StopReason> {
        let H2b {
            s: s0,
            r: r0,
            dirty: dirty_b,
        } = *b;

        let context_zero = self.summary_may_be_all_zero_context(c.r)
            && self.summary_may_be_all_zero_context(r0);
        let Some(res) = self.tm_step(
            c.w,
            s0,
            -1,
            c.dirty || dirty_b,
            context_zero,
        )?
        else {
            return Ok(());
        };

        let dirty1 = c.dirty || dirty_b || res.saw_nonzero;
        if res.is_back {
            let c0 = H3 {
                w: res.w1,
                s: res.s1,
                r: r0,
                dirty: dirty1,
            };
            self.insert_h3(&c0)?;
            self.pre33.insert(c0, *c);
        } else {
            let rr = self.dfa_push_id(res.w1, r0)?;
            self.insert_ret3(
                *c,
                H2b {
                    s: res.s1,
                    r: rr,
                    dirty: dirty1,
                },
            );
        }

        Ok(())
    }

    fn on_retl(&mut self, b: &H2b) -> Result<(), StopReason> {
        let H2b {
            s: s0,
            r: r0,
            dirty,
        } = *b;

        let blank = self.words.intern(Word::zero(self.block_len));
        let context_zero = self.summary_may_be_all_zero_context(r0);
        let Some(res) =
            self.tm_step(blank, s0, -1, dirty, context_zero)?
        else {
            return Ok(());
        };

        let dirty1 = dirty || res.saw_nonzero;
        if res.is_back {
            let c0 = H3 {
                w: res.w1,
                s: res.s1,
                r: r0,
                dirty: dirty1,
            };
            self.insert_h3(&c0)?;
            self.insert_pre3l(&c0)?;
        } else {
            let rr = self.dfa_push_id(res.w1, r0)?;
            let b0 = H2b {
                s: res.s1,
                r: rr,
                dirty: dirty1,
            };
            self.insert_retl(&b0)?;
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
        while let Some((s, dirty)) = self.scratch_states.pop() {
            self.on_h2_pop(&H2 { s, r, dirty }, e)?;
        }

        Ok(())
    }

    fn on_h2(&mut self, a: &H2) -> Result<(), StopReason> {
        let H2 { s, r, .. } = a;

        self.bump()?;
        self.ensure_dfa_capacity(*r);
        self.r_s[*r].insert((*s, a.dirty));

        self.scratch_edges.clear();
        self.scratch_edges.extend(self.pop[*r].iter().copied());
        while let Some(e) = self.scratch_edges.pop() {
            self.on_h2_pop(a, &e)?;
        }

        Ok(())
    }

    fn on_h3(&mut self, a: H3) {
        let a0 = H2 {
            s: a.s,
            r: a.r,
            dirty: a.dirty,
        };
        self.insert_h2(a0);
        self.pre23.insert(a0, a);
    }

    fn on_ret2(&mut self, a: &H2, b: &H2b) -> Result<(), StopReason> {
        self.scratch_h3.clear();
        self.scratch_h3.extend(self.pre23.values(a).copied());
        while let Some(c) = self.scratch_h3.pop() {
            self.on_h3_back(&c, b)?;
        }
        Ok(())
    }

    fn on_ret3(&mut self, a: &H3, b: &H2b) -> Result<(), StopReason> {
        self.scratch_h2.clear();
        self.scratch_h2.extend(self.pre32.values(a).copied());
        while let Some(a0) = self.scratch_h2.pop() {
            self.insert_ret2(a0, *b);
        }

        self.scratch_h3.clear();
        self.scratch_h3.extend(self.pre33.values(a).copied());
        while let Some(a1) = self.scratch_h3.pop() {
            self.insert_ret3(a1, *b);
        }

        if self.pre3l.contains(a) {
            self.insert_retl(b)?;
        }
        Ok(())
    }

    fn on_pre23(&mut self, a: &H2, c: &H3) -> Result<(), StopReason> {
        self.scratch_h2b.clear();
        self.scratch_h2b.extend(self.ret2.values(a).copied());
        while let Some(b) = self.scratch_h2b.pop() {
            self.on_h3_back(c, &b)?;
        }
        Ok(())
    }

    fn on_pre32(&mut self, a: &H3, a0: &H2) {
        self.scratch_h2b.clear();
        self.scratch_h2b.extend(self.ret3.values(a).copied());
        while let Some(b) = self.scratch_h2b.pop() {
            self.insert_ret2(*a0, b);
        }
    }

    fn on_pre33(&mut self, a: &H3, a0: &H3) {
        self.scratch_h2b.clear();
        self.scratch_h2b.extend(self.ret3.values(a).copied());
        while let Some(b) = self.scratch_h2b.pop() {
            self.insert_ret3(*a0, b);
        }
    }

    fn on_pre3l(&mut self, a: &H3) -> Result<(), StopReason> {
        self.scratch_h2b.clear();
        self.scratch_h2b.extend(self.ret3.values(a).copied());
        while let Some(b) = self.scratch_h2b.pop() {
            self.insert_retl(&b)?;
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
        self.h3s.insert(c0);
        self.pre3l.insert(c0);

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

fn far_decide_with<
    S: Summary,
    const STATES: usize,
    const COLORS: usize,
>(
    prog: &Prog<STATES, COLORS>,
    block_len: usize,
    max_work: usize,
    block_step_limit: usize,
    goal: Goal,
) -> bool {
    if block_len == 0 {
        return false;
    }

    let Ok(decider) = FarDecider::<S, STATES, COLORS>::new(
        prog,
        block_len,
        max_work,
        block_step_limit,
        goal,
    ) else {
        return false;
    };

    decider.run().is_ok()
}

fn far_sweep_fast<const STATES: usize, const COLORS: usize>(
    prog: &Prog<STATES, COLORS>,
    knob: usize,
    goal: Goal,
) -> bool {
    far_sweep_with::<STATES, COLORS>(
        prog,
        knob,
        goal,
        far_decide_fast::<STATES, COLORS>,
    )
}

fn far_sweep_slow<const STATES: usize, const COLORS: usize>(
    prog: &Prog<STATES, COLORS>,
    knob: usize,
    goal: Goal,
) -> bool {
    far_sweep_with::<STATES, COLORS>(
        prog,
        knob,
        goal,
        far_decide_slow::<STATES, COLORS>,
    )
}

fn far_sweep_with<const STATES: usize, const COLORS: usize>(
    prog: &Prog<STATES, COLORS>,
    knob: usize,
    goal: Goal,
    decide: fn(
        &Prog<STATES, COLORS>,
        usize,
        usize,
        usize,
        Goal,
    ) -> bool,
) -> bool {
    if COLORS < 2 || STATES == 0 {
        return false;
    }

    let knob = knob.max(FAR_KNOB_MIN);
    let eff = effort_factor(knob);

    let cap_by_colors = if COLORS <= 2 {
        FAR_BLOCK_LEN_CAP_COLORS_2
    } else if COLORS <= 4 {
        FAR_BLOCK_LEN_CAP_COLORS_3_4
    } else {
        FAR_BLOCK_LEN_CAP_COLORS_5_8
    };

    let max_block_len =
        knob.min(cap_by_colors).min(FAR_BLOCK_LEN_HARD_CAP);

    for block_len in 2..=max_block_len {
        let max_work = FAR_WORK_PER_LEN
            .saturating_mul(block_len)
            .saturating_mul(eff);

        let block_step_limit = FAR_STEP_PER_LEN
            .saturating_mul(block_len)
            .saturating_mul(eff);

        if decide(prog, block_len, max_work, block_step_limit, goal) {
            return true;
        }
    }

    false
}

fn far_decide_fast<const STATES: usize, const COLORS: usize>(
    prog: &Prog<STATES, COLORS>,
    block_len: usize,
    max_work: usize,
    block_step_limit: usize,
    goal: Goal,
) -> bool {
    far_decide_with::<Ng1Summary, STATES, COLORS>(
        prog,
        block_len,
        max_work,
        block_step_limit,
        goal,
    ) || far_decide_with::<CpsLruSummary, STATES, COLORS>(
        prog,
        block_len,
        max_work,
        block_step_limit,
        goal,
    ) || far_decide_with::<RwlModSummary, STATES, COLORS>(
        prog,
        block_len,
        max_work,
        block_step_limit,
        goal,
    )
}

fn far_decide_slow<const STATES: usize, const COLORS: usize>(
    prog: &Prog<STATES, COLORS>,
    block_len: usize,
    max_work: usize,
    block_step_limit: usize,
    goal: Goal,
) -> bool {
    far_decide_with::<RngsModSummary, STATES, COLORS>(
        prog,
        block_len,
        max_work,
        block_step_limit,
        goal,
    ) || far_decide_with::<RsModSummary, STATES, COLORS>(
        prog,
        block_len,
        max_work,
        block_step_limit,
        goal,
    ) || far_decide_with::<NgSummary, STATES, COLORS>(
        prog,
        block_len,
        max_work,
        block_step_limit,
        goal,
    )
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

#[derive(Clone, Copy)]
struct MitmNext {
    config: MitmConfig,
    weight: i32,
}

#[derive(Clone, Copy, Default)]
struct MitmBounds {
    lo: Option<i32>,
    hi: Option<i32>,
}

#[derive(Clone)]
struct MitmSpecial {
    nonneg: Vec<bool>,
    nonpos: Vec<bool>,
}

type MitmAccept = Map<MitmConfig, MitmBounds>;

#[derive(Clone, Copy)]
enum MitmSide {
    Left,
    Right,
}

const MITM_DEAD: usize = 1;
const MITM_MAX_FINITE_INTERVAL: i32 = 100;

fn mitm_cant_halt<const STATES: usize, const COLORS: usize>(
    prog: &Prog<STATES, COLORS>,
) -> bool {
    mitm_cant_target::<STATES, COLORS>(prog, Goal::Halt)
}

fn mitm_cant_blank<const STATES: usize, const COLORS: usize>(
    prog: &Prog<STATES, COLORS>,
) -> bool {
    mitm_cant_target::<STATES, COLORS>(prog, Goal::Blank)
}

fn mitm_cant_target<const STATES: usize, const COLORS: usize>(
    prog: &Prog<STATES, COLORS>,
    goal: Goal,
) -> bool {
    // Fast, boolean-only port of Iijil1/MITMWFAR's search path:
    // enumerate closed MITM-DFA skeletons, then try one (+1,-1) WFA
    // weight pair, deriving and verifying the accept set in memory.
    const MAX_TRANSITIONS: usize = 9;
    const MAX_WEIGHT_PAIRS: usize = 1;

    if COLORS < 2 || STATES == 0 || prog.get(&(0, 0)).is_none() {
        return false;
    }

    // Cheap passes first.  These preserve the same eventual prover power
    // as the final pass, but avoid paying for memory expansion on easy TMs.
    for added_memory in [0_usize, 1] {
        for dfa_transitions in 2..=MAX_TRANSITIONS {
            if mitm_decide_exact(
                prog,
                goal,
                dfa_transitions,
                dfa_transitions,
                dfa_transitions,
                MAX_WEIGHT_PAIRS,
                added_memory,
            ) {
                return true;
            }
        }
    }

    false
}

fn mitm_decide_exact<const STATES: usize, const COLORS: usize>(
    prog: &Prog<STATES, COLORS>,
    goal: Goal,
    goal_transitions: usize,
    max_left_states: usize,
    max_right_states: usize,
    max_weight_pairs: usize,
    added_memory: usize,
) -> bool {
    let mut left = MitmWfa::new(COLORS);
    let mut right = MitmWfa::new(COLORS);
    left.trans[0][0] = (0, 0);
    right.trans[0][0] = (0, 0);

    mitm_recurse_dfa(
        prog,
        goal,
        &left,
        &right,
        2,
        goal_transitions,
        max_left_states,
        max_right_states,
        max_weight_pairs,
        added_memory,
    )
}

#[expect(clippy::too_many_arguments, clippy::needless_borrow)]
fn mitm_recurse_dfa<const STATES: usize, const COLORS: usize>(
    prog: &Prog<STATES, COLORS>,
    goal: Goal,
    left: &MitmWfa,
    right: &MitmWfa,
    current_transitions: usize,
    goal_transitions: usize,
    max_left_states: usize,
    max_right_states: usize,
    max_weight_pairs: usize,
    added_memory: usize,
) -> bool {
    match mitm_find_closure_break(prog, goal, left, right) {
        None => {
            current_transitions == goal_transitions
                && mitm_recurse_weights(
                    prog,
                    goal,
                    left,
                    right,
                    0,
                    max_weight_pairs,
                    added_memory,
                )
        },
        Some((MitmSide::Left, state, color)) => {
            if current_transitions >= goal_transitions {
                return false;
            }

            if left.states < max_left_states {
                let mut next = left.clone();
                let new_state = next.push_dead_state();
                next.trans[state][color] = (new_state, 0);
                if mitm_recurse_dfa(
                    prog,
                    goal,
                    &next,
                    &right,
                    current_transitions + 1,
                    goal_transitions,
                    max_left_states,
                    max_right_states,
                    max_weight_pairs,
                    added_memory,
                ) {
                    return true;
                }
            }

            for to_state in 0..left.states {
                if to_state == MITM_DEAD {
                    continue;
                }
                let mut next = left.clone();
                next.trans[state][color] = (to_state, 0);
                if mitm_recurse_dfa(
                    prog,
                    goal,
                    &next,
                    &right,
                    current_transitions + 1,
                    goal_transitions,
                    max_left_states,
                    max_right_states,
                    max_weight_pairs,
                    added_memory,
                ) {
                    return true;
                }
            }

            false
        },
        Some((MitmSide::Right, state, color)) => {
            if current_transitions >= goal_transitions {
                return false;
            }

            if right.states < max_right_states {
                let mut next = right.clone();
                let new_state = next.push_dead_state();
                next.trans[state][color] = (new_state, 0);
                if mitm_recurse_dfa(
                    prog,
                    goal,
                    &left,
                    &next,
                    current_transitions + 1,
                    goal_transitions,
                    max_left_states,
                    max_right_states,
                    max_weight_pairs,
                    added_memory,
                ) {
                    return true;
                }
            }

            for to_state in 0..right.states {
                if to_state == MITM_DEAD {
                    continue;
                }
                let mut next = right.clone();
                next.trans[state][color] = (to_state, 0);
                if mitm_recurse_dfa(
                    prog,
                    goal,
                    &left,
                    &next,
                    current_transitions + 1,
                    goal_transitions,
                    max_left_states,
                    max_right_states,
                    max_weight_pairs,
                    added_memory,
                ) {
                    return true;
                }
            }

            false
        },
    }
}

fn mitm_find_closure_break<const STATES: usize, const COLORS: usize>(
    prog: &Prog<STATES, COLORS>,
    goal: Goal,
    left: &MitmWfa,
    right: &MitmWfa,
) -> Option<(MitmSide, usize, usize)> {
    let left_rev = left.rev_edges();
    let right_rev = right.rev_edges();
    let start = MitmConfig {
        st: 0,
        co: 0,
        left: 0,
        right: 0,
        dirty: false,
    };
    let mut seen = Set::new();
    let mut todo = vec![start];
    seen.insert(start);

    while let Some(cur) = todo.pop() {
        if cur.st >= STATES || cur.co >= COLORS {
            continue;
        }
        #[expect(clippy::cast_possible_truncation)]
        let Some(&(write, shift, _)) =
            prog.get(&(cur.st as State, cur.co as Color))
        else {
            continue;
        };
        let write = write as usize;

        for next in mitm_next_configs(
            prog, goal, cur, left, right, &left_rev, &right_rev,
        ) {
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

        let _ = shift;
    }

    None
}

#[expect(clippy::similar_names)]
fn mitm_recurse_weights<const STATES: usize, const COLORS: usize>(
    prog: &Prog<STATES, COLORS>,
    goal: Goal,
    left: &MitmWfa,
    right: &MitmWfa,
    current_weight_pairs: usize,
    max_weight_pairs: usize,
    added_memory: usize,
) -> bool {
    let mut try_left = left.clone();
    let mut try_right = right.clone();
    for _ in 0..added_memory {
        try_left = try_left.with_memory();
        try_right = try_right.with_memory();
    }

    let left_special = try_left.derive_special();
    let right_special = try_right.derive_special();
    let left_rev = try_left.rev_edges();
    let right_rev = try_right.rev_edges();
    if let Some(accept) = mitm_find_accept_set(
        prog,
        goal,
        &try_left,
        &try_right,
        &left_rev,
        &right_rev,
        &left_special,
        &right_special,
    ) && mitm_verify(
        prog,
        goal,
        &try_left,
        &try_right,
        &left_rev,
        &right_rev,
        &left_special,
        &right_special,
        &accept,
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
        for ls in 0..left.states {
            for lc in 0..left.colors {
                let (lt, old_lw) = left.trans[ls][lc];
                if lt == MITM_DEAD || (ls == 0 && lc == 0) {
                    continue;
                }
                let mut new_left = left.clone();
                new_left.trans[ls][lc] = (lt, old_lw + lw);

                for rs in 0..right.states {
                    for rc in 0..right.colors {
                        let (rt, old_rw) = right.trans[rs][rc];
                        if rt == MITM_DEAD || (rs == 0 && rc == 0) {
                            continue;
                        }
                        let mut new_right = right.clone();
                        new_right.trans[rs][rc] = (rt, old_rw + rw);
                        if mitm_recurse_weights(
                            prog,
                            goal,
                            &new_left,
                            &new_right,
                            current_weight_pairs + 1,
                            max_weight_pairs,
                            added_memory,
                        ) {
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
}

fn mitm_find_accept_set<const STATES: usize, const COLORS: usize>(
    prog: &Prog<STATES, COLORS>,
    goal: Goal,
    left: &MitmWfa,
    right: &MitmWfa,
    left_rev: &MitmRev,
    right_rev: &MitmRev,
    left_special: &MitmSpecial,
    right_special: &MitmSpecial,
) -> Option<MitmAccept> {
    let start = MitmConfig {
        st: 0,
        co: 0,
        left: 0,
        right: 0,
        dirty: false,
    };
    let mut accept = MitmAccept::new();
    let mut todo = vec![start];
    accept.insert(
        start,
        MitmBounds {
            lo: Some(0),
            hi: Some(0),
        },
    );

    while let Some(cur) = todo.pop() {
        let cur_bounds = accept[&cur];
        let mut nexts = mitm_next_configs(
            prog, goal, cur, left, right, left_rev, right_rev,
        );
        if nexts.is_empty() {
            if goal.is_halt() {
                return None;
            }
            continue;
        }
        nexts.sort_by_key(|next| next.config);

        for next in nexts {
            if mitm_accept_contain_next(
                next,
                cur_bounds,
                left_special,
                right_special,
                &mut accept,
            ) {
                todo.push(next.config);
            }
        }
    }

    Some(accept)
}

fn mitm_accept_contain_next(
    next: MitmNext,
    bounds: MitmBounds,
    left_special: &MitmSpecial,
    right_special: &MitmSpecial,
    accept: &mut MitmAccept,
) -> bool {
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
        return false;
    }

    let next_bounds = MitmBounds { lo, hi };
    let Some(old) = accept.get_mut(&cfg) else {
        accept.insert(cfg, next_bounds);
        return true;
    };

    let mut changed = false;

    if let Some(old_lo) = old.lo
        && (next_bounds.lo.is_none() || Some(old_lo) > next_bounds.lo)
    {
        changed = true;
        if old.hi.is_none()
            || next_bounds.lo.is_none()
            || old.hi.unwrap() - next_bounds.lo.unwrap()
                > MITM_MAX_FINITE_INTERVAL
        {
            old.lo = hard_lo.then_some(0);
        } else {
            old.lo = next_bounds.lo;
        }
    }

    if let Some(old_hi) = old.hi
        && (next_bounds.hi.is_none() || Some(old_hi) < next_bounds.hi)
    {
        changed = true;
        if old.lo.is_none()
            || next_bounds.hi.is_none()
            || next_bounds.hi.unwrap() - old.lo.unwrap()
                > MITM_MAX_FINITE_INTERVAL
        {
            old.hi = hard_hi.then_some(0);
        } else {
            old.hi = next_bounds.hi;
        }
    }

    changed
}

fn mitm_verify<const STATES: usize, const COLORS: usize>(
    prog: &Prog<STATES, COLORS>,
    goal: Goal,
    left: &MitmWfa,
    right: &MitmWfa,
    left_rev: &MitmRev,
    right_rev: &MitmRev,
    left_special: &MitmSpecial,
    right_special: &MitmSpecial,
    accept: &MitmAccept,
) -> bool {
    left.verify_leading_blank()
        && right.verify_leading_blank()
        && left.verify_special(left_special)
        && right.verify_special(right_special)
        && mitm_start_accepted(accept)
        && mitm_target_not_accepted(prog, goal, accept)
        && mitm_forward_closed(
            prog,
            goal,
            left,
            right,
            left_rev,
            right_rev,
            left_special,
            right_special,
            accept,
        )
}

fn mitm_start_accepted(accept: &MitmAccept) -> bool {
    let start = MitmConfig {
        st: 0,
        co: 0,
        left: 0,
        right: 0,
        dirty: false,
    };
    let Some(bounds) = accept.get(&start) else {
        return false;
    };
    bounds.lo.is_none_or(|lo| lo <= 0)
        && bounds.hi.is_none_or(|hi| hi >= 0)
}

#[expect(clippy::cast_possible_truncation)]
fn mitm_target_not_accepted<
    const STATES: usize,
    const COLORS: usize,
>(
    prog: &Prog<STATES, COLORS>,
    goal: Goal,
    accept: &MitmAccept,
) -> bool {
    match goal {
        Goal::Halt => accept.keys().all(|cfg| {
            cfg.st < STATES
                && cfg.co < COLORS
                && prog
                    .get(&(cfg.st as State, cfg.co as Color))
                    .is_some()
        }),
        Goal::Blank => accept.iter().all(|(cfg, bounds)| {
            cfg.st < STATES
                && cfg.co < COLORS
                && !mitm_blank_config_possible(cfg, bounds)
        }),
        Goal::Spinout => unreachable!(),
    }
}

fn mitm_blank_config_possible(
    cfg: &MitmConfig,
    bounds: &MitmBounds,
) -> bool {
    cfg.dirty
        && cfg.co == 0
        && cfg.left == 0
        && cfg.right == 0
        && bounds.lo.is_none_or(|lo| lo <= 0)
        && bounds.hi.is_none_or(|hi| hi >= 0)
}

fn mitm_forward_closed<const STATES: usize, const COLORS: usize>(
    prog: &Prog<STATES, COLORS>,
    goal: Goal,
    left: &MitmWfa,
    right: &MitmWfa,
    left_rev: &MitmRev,
    right_rev: &MitmRev,
    left_special: &MitmSpecial,
    right_special: &MitmSpecial,
    accept: &MitmAccept,
) -> bool {
    for (&cfg, &bounds) in accept {
        for next in mitm_next_configs(
            prog, goal, cfg, left, right, left_rev, right_rev,
        ) {
            if !mitm_next_accepted(
                next,
                bounds,
                left_special,
                right_special,
                accept,
            ) {
                return false;
            }
        }
    }
    true
}

fn mitm_next_accepted(
    next: MitmNext,
    bounds: MitmBounds,
    left_special: &MitmSpecial,
    right_special: &MitmSpecial,
    accept: &MitmAccept,
) -> bool {
    let cfg = next.config;
    let mut lo = bounds.lo.map(|x| x + next.weight);
    let mut hi = bounds.hi.map(|x| x + next.weight);

    if left_special.nonneg[cfg.left]
        && right_special.nonneg[cfg.right]
        && lo.is_none_or(|x| x < 0)
    {
        lo = Some(0);
    }
    if left_special.nonpos[cfg.left]
        && right_special.nonpos[cfg.right]
        && hi.is_none_or(|x| x > 0)
    {
        hi = Some(0);
    }
    if let (Some(lo), Some(hi)) = (lo, hi)
        && hi < lo
    {
        return true;
    }

    let Some(accepted) = accept.get(&cfg) else {
        return false;
    };

    if let Some(acc_lo) = accepted.lo
        && lo.is_none_or(|next_lo| acc_lo > next_lo)
    {
        return false;
    }
    if let Some(acc_hi) = accepted.hi
        && hi.is_none_or(|next_hi| acc_hi < next_hi)
    {
        return false;
    }

    true
}

fn mitm_next_configs<const STATES: usize, const COLORS: usize>(
    prog: &Prog<STATES, COLORS>,
    goal: Goal,
    old: MitmConfig,
    left: &MitmWfa,
    right: &MitmWfa,
    left_rev: &MitmRev,
    right_rev: &MitmRev,
) -> Vec<MitmNext> {
    if old.st >= STATES || old.co >= COLORS {
        return vec![];
    }
    #[expect(clippy::cast_possible_truncation)]
    let Some(&(write, shift, next_st)) =
        prog.get(&(old.st as State, old.co as Color))
    else {
        return vec![];
    };

    let write = write as usize;
    let next_st = next_st as usize;
    let next_dirty =
        goal.is_blank() && (old.dirty || old.co != 0 || write != 0);
    let mut out = vec![];

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
        let (new_right, right_weight) = right.trans[old.right][write];
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

    out
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
