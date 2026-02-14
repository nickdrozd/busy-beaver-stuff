//! FAR non-halting prover (Finite Automaton Reduction)
//!
//! Self-contained FAR decider, implemented as a method on `Prog`.
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
//! - Returns `true` only when FAR reaches a fixed point (a sound proof that the
//!   machine cannot halt).
//! - Returns `false` otherwise (it may halt, or FAR ran out of budgets).
//!
//! # How the knob works
//! `knob` controls *both*:
//! - the range of block lengths to try (roughly `2..=knob` but with a colors-based cap)
//! - budgets for each FAR run (more work / deeper block simulation)

use core::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use crate::{Color, Instr, Prog, Slot, State};

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
#[inline]
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
    /// - `true` iff FAR *proved* the machine cannot halt.
    /// - `false` otherwise.
    pub fn far_cant_halt(&self, knob: usize) -> bool {
        if COLORS < 2 || STATES == 0 {
            return false;
        }

        let knob = knob.max(FAR_KNOB_MIN);
        let eff = effort_factor(knob);

        // Choose a practical maximum block length based on alphabet size.
        let cap_by_colors = if COLORS <= 2 {
            FAR_BLOCK_LEN_CAP_COLORS_2
        } else if COLORS <= 4 {
            FAR_BLOCK_LEN_CAP_COLORS_3_4
        } else {
            FAR_BLOCK_LEN_CAP_COLORS_5_8
        };

        let max_block_len =
            knob.min(cap_by_colors).min(FAR_BLOCK_LEN_HARD_CAP);

        // Sweep block lengths like CPS-style "check 2..=X".
        // Stop early if we find a proof.
        for block_len in 2..=max_block_len {
            // Scale budgets with both block_len and knob (via eff).
            let max_work = FAR_WORK_PER_LEN
                .saturating_mul(block_len)
                .saturating_mul(eff);

            let block_step_limit = FAR_STEP_PER_LEN
                .saturating_mul(block_len)
                .saturating_mul(eff);

            if far_decide_ng::<STATES, COLORS>(
                self,
                block_len,
                max_work,
                block_step_limit,
            ) {
                return true;
            }
        }

        false
    }

    pub fn far_cant_blank(&self, _knob: usize) -> bool {
        unimplemented!()
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
#[derive(Clone, Eq, PartialEq, Debug)]
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

    #[inline]
    fn get(&self, idx: usize) -> Color {
        self.cells[idx]
    }

    #[inline]
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

/// Result of simulating the TM within one block until it exits or halts.
#[derive(Clone, Debug)]
struct WordUpdateLemma {
    w1: Word,
    s1: i16,
    is_back: bool,
    #[expect(dead_code)]
    n_step: usize,
}

impl WordUpdateLemma {
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
                });
            };

            if out_color as usize >= COLORS {
                return None;
            }
            if next_state as usize >= STATES {
                return None;
            }

            w1.set(pos as usize, out_color);
            s1 = i16::from(next_state);

            let dir: i32 = if shift_right { 1 } else { -1 };
            pos += dir * i32::from(sgn);

            if pos < 0 || pos >= len {
                return Some(Self {
                    w1,
                    s1,
                    n_step: t + 1,
                    is_back: pos < 0,
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
    ) -> Option<Self> {
        let mut res = Self::from_prog(prog, w, s, sgn, max_steps)?;
        if res.s1 >= 0 && !res.is_back {
            res.w1 = res.w1.reverse();
        }
        Some(res)
    }
}

/// NG stack summary: keep bounded history (window + tail + mod counter).
#[derive(Clone, Debug)]
struct NgSummary {
    q: Vec<Word>,
    q0: Vec<Word>,
    modu: usize,

    window: usize,
    tail: usize,
    pos_mod: usize,
}

impl NgSummary {
    fn new(window: usize, tail: usize, pos_mod: usize) -> Self {
        Self {
            q: Vec::new(),
            q0: Vec::new(),
            modu: 0,
            window,
            tail,
            pos_mod: pos_mod.max(1),
        }
    }

    fn push(&mut self, w: Word) {
        // Infinite blanks compression at the "front".
        if self.q.is_empty() && w.is_zero() {
            return;
        }

        if self.window > 0 && self.q.len() == self.window {
            self.q.remove(0);
        }
        self.q.push(w.clone());

        if self.q0.len() < self.tail {
            self.q0.push(w);
        }

        self.modu = (self.modu + 1) % self.pos_mod;
    }
}

impl PartialEq for NgSummary {
    fn eq(&self, other: &Self) -> bool {
        self.modu == other.modu
            && self.q == other.q
            && self.q0 == other.q0
    }
}

impl Eq for NgSummary {}

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

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
struct H2 {
    s: i16,
    r: usize,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
struct H2b {
    s: i16,
    r: usize,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
struct H3 {
    w: Word,
    s: i16,
    r: usize,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
struct DfaEdge {
    w: Word,
    prev: usize,
}

/// A set with a LIFO todo stack.
#[derive(Clone, Debug)]
struct TodoSet<K: Ord + Clone> {
    st: BTreeSet<K>,
    todo: Vec<K>,
}

impl<K: Ord + Clone> TodoSet<K> {
    const fn new() -> Self {
        Self {
            st: BTreeSet::new(),
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
struct TodoMap<K: Ord + Clone, V: Ord + Clone> {
    mp: BTreeMap<K, BTreeSet<V>>,
    todo: Vec<(K, V)>,
}

impl<K: Ord + Clone, V: Ord + Clone> TodoMap<K, V> {
    const fn new() -> Self {
        Self {
            mp: BTreeMap::new(),
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
    MayHalt,
}

/// FAR decider (NG summary specialization).
struct FarDecider<'a, const STATES: usize, const COLORS: usize> {
    prog: &'a Prog<STATES, COLORS>,

    block_len: usize,
    max_work: usize,
    block_step_limit: usize,

    // work counter
    work: usize,

    // DFA
    id: BTreeMap<NgSummary, usize>,
    idr: Vec<NgSummary>,

    pop: Vec<Vec<DfaEdge>>,
    push: BTreeMap<(Word, usize), usize>,
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

    // For each DFA state r, which machine states s have H2(s,r)
    r_s: Vec<BTreeSet<i16>>,
}

impl<'a, const STATES: usize, const COLORS: usize>
    FarDecider<'a, STATES, COLORS>
{
    fn new(
        prog: &'a Prog<STATES, COLORS>,
        block_len: usize,
        max_work: usize,
        block_step_limit: usize,
        ng_window: usize,
        ng_tail: usize,
        ng_pos_mod: usize,
    ) -> Self {
        let idr = vec![NgSummary::new(0, 0, 1)];

        Self {
            prog,
            block_len,
            max_work: max_work.max(1),
            block_step_limit: block_step_limit.max(1),

            work: 0,

            id: BTreeMap::new(),
            idr,

            pop: Vec::new(),
            push: BTreeMap::new(),
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
        }
        .with_init_dfa(ng_window, ng_tail, ng_pos_mod)
    }

    fn with_init_dfa(
        mut self,
        ng_window: usize,
        ng_tail: usize,
        ng_pos_mod: usize,
    ) -> Self {
        // ensure vector sizes are at least 1 (dummy)
        self.ensure_dfa_capacity(0);

        let init = NgSummary::new(ng_window, ng_tail, ng_pos_mod);
        let id0 = self.get_id(init);
        debug_assert_eq!(id0, 1);

        let blank = Word::zero(self.block_len);
        let id1 = self.dfa_push(blank, id0);
        debug_assert_eq!(id1, id0);

        self
    }

    #[inline]
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
            self.r_s.resize_with(id + 1, BTreeSet::new);
        }
    }

    fn get_id(&mut self, st: NgSummary) -> usize {
        if let Some(&id) = self.id.get(&st) {
            return id;
        }
        let id = self.idr.len();
        self.id.insert(st.clone(), id);
        self.idr.push(st);
        self.ensure_dfa_capacity(id);
        id
    }

    fn dfa_push(&mut self, w: Word, ls: usize) -> usize {
        let key = (w.clone(), ls);
        if let Some(&to) = self.push.get(&key) {
            return to;
        }

        let mut st = self.idr[ls].clone();
        st.push(w.clone());
        let to = self.get_id(st);
        self.push.insert(key, to);

        // delay adding pop-edge until processed
        self.new_pops.push((to, DfaEdge { w, prev: ls }));
        to
    }

    fn tm_step(
        &mut self,
        w: Word,
        s: i16,
        sgn: i8,
    ) -> Result<WordUpdateLemma, StopReason> {
        self.bump()?;

        let Some(res) = WordUpdateLemma::from_prog_v2::<STATES, COLORS>(
            self.prog,
            w,
            s,
            sgn,
            self.block_step_limit,
        ) else {
            return Err(StopReason::BlockTimeout);
        };

        if res.s1 == -1 {
            return Err(StopReason::MayHalt);
        }

        Ok(res)
    }

    // -------------------------
    // Event handlers (port of far.cpp)
    // -------------------------

    fn on_h2_pop(
        &mut self,
        a: H2,
        b: DfaEdge,
    ) -> Result<(), StopReason> {
        let H2 { s, .. } = a;
        let DfaEdge { w, prev: r0 } = b;

        let res = self.tm_step(w, s, 1)?;

        if res.is_back {
            // <s1 w1 [r0]
            let rr = self.dfa_push(res.w1, r0);
            self.ret2.insert(a, H2b { s: res.s1, r: rr });
        } else {
            // w1 s1> [r0]
            let c = H3 {
                w: res.w1,
                s: res.s1,
                r: r0,
            };
            self.h3s.insert(c.clone());
            self.pre32.insert(c, a);
        }

        Ok(())
    }

    fn on_h3_back(&mut self, c: H3, b: &H2b) -> Result<(), StopReason> {
        let H3 { w, .. } = c.clone();
        let H2b { s: s0, r: r0 } = b;

        let res = self.tm_step(w, *s0, -1)?;

        if res.is_back {
            // w1 s1> [r0]
            let c0 = H3 {
                w: res.w1,
                s: res.s1,
                r: *r0,
            };
            self.h3s.insert(c0.clone());
            self.pre33.insert(c0, c);
        } else {
            // <s1 w1 [r0]
            let rr = self.dfa_push(res.w1, *r0);
            self.ret3.insert(c, H2b { s: res.s1, r: rr });
        }

        Ok(())
    }

    fn on_retl(&mut self, b: &H2b) -> Result<(), StopReason> {
        let H2b { s: s0, r: r0 } = b;

        let blank = Word::zero(self.block_len);
        let res = self.tm_step(blank, *s0, -1)?;

        if res.is_back {
            // w1 s1> [r0]
            let c0 = H3 {
                w: res.w1,
                s: res.s1,
                r: *r0,
            };
            self.h3s.insert(c0.clone());
            self.pre3l.insert(c0);
        } else {
            // <s1 w1 [r0]
            let rr = self.dfa_push(res.w1, *r0);
            self.retl.insert(H2b { s: res.s1, r: rr });
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
        self.pop[r].push(e.clone());

        // For all known H2(s,r), process this new pop-edge.
        let states: Vec<i16> = self.r_s[r].iter().copied().collect();
        for s in states {
            self.on_h2_pop(H2 { s, r }, e.clone())?;
        }

        Ok(())
    }

    fn on_h2(&mut self, a: &H2) -> Result<(), StopReason> {
        let H2 { s, r } = a;

        self.bump()?;
        self.ensure_dfa_capacity(*r);
        self.r_s[*r].insert(*s);

        // apply all known pop edges out of r
        let edges = self.pop[*r].clone();
        for e in edges {
            self.on_h2_pop(a.clone(), e)?;
        }

        Ok(())
    }

    fn on_h3(&mut self, a: H3) {
        let H3 { s, r, .. } = a;
        let a0 = H2 { s, r };
        self.h2s.insert(a0.clone());
        self.pre23.insert(a0, a);
    }

    fn on_ret2(&mut self, a: &H2, b: &H2b) -> Result<(), StopReason> {
        let cs: Vec<H3> = self.pre23.values(a).cloned().collect();
        for c in cs {
            self.on_h3_back(c, b)?;
        }
        Ok(())
    }

    fn on_ret3(&mut self, a: &H3, b: &H2b) {
        let a0s: Vec<H2> = self.pre32.values(a).cloned().collect();
        for a0 in a0s {
            self.ret2.insert(a0, b.clone());
        }

        let a1s: Vec<H3> = self.pre33.values(a).cloned().collect();
        for a1 in a1s {
            self.ret3.insert(a1, b.clone());
        }

        if self.pre3l.contains(a) {
            self.retl.insert(b.clone());
        }
    }

    fn on_pre23(&mut self, a: &H2, c: &H3) -> Result<(), StopReason> {
        let bs: Vec<H2b> = self.ret2.values(a).cloned().collect();
        for b in bs {
            self.on_h3_back(c.clone(), &b)?;
        }
        Ok(())
    }

    fn on_pre32(&mut self, a: &H3, a0: &H2) {
        let bs: Vec<H2b> = self.ret3.values(a).cloned().collect();
        for b in bs {
            self.ret2.insert(a0.clone(), b);
        }
    }

    fn on_pre33(&mut self, a: &H3, a0: &H3) {
        let bs: Vec<H2b> = self.ret3.values(a).cloned().collect();
        for b in bs {
            self.ret3.insert(a0.clone(), b);
        }
    }

    fn on_pre3l(&mut self, a: &H3) {
        let bs: Vec<H2b> = self.ret3.values(a).cloned().collect();
        for b in bs {
            self.retl.insert(b);
        }
    }

    fn run(mut self) -> Result<(), StopReason> {
        // initial condition: H3(blank, A, r=1) and pre3L
        let blank = Word::zero(self.block_len);
        let c0 = H3 {
            w: blank,
            s: 0,
            r: 1,
        };
        self.h3s.insert(c0.clone());
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
                self.on_pre3l(&a);
                continue;
            }
            if let Some((a, b)) = self.ret2.pop_todo() {
                self.on_ret2(&a, &b)?;
                continue;
            }
            if let Some((a, b)) = self.ret3.pop_todo() {
                self.on_ret3(&a, &b);
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

            // Fixed point reached => proved non-halting.
            break;
        }

        Ok(())
    }
}

/// Run FAR(decider) with NG summary for a single `(block_len, budgets)`.
///
/// Returns `true` iff it proves non-halting.
fn far_decide_ng<const STATES: usize, const COLORS: usize>(
    prog: &Prog<STATES, COLORS>,
    block_len: usize,
    max_work: usize,
    block_step_limit: usize,
) -> bool {
    if block_len == 0 {
        return false;
    }

    let decider = FarDecider::new(
        prog,
        block_len,
        max_work,
        block_step_limit,
        FAR_NG_WINDOW,
        FAR_NG_TAIL,
        FAR_NG_POS_MOD,
    );

    decider.run().is_ok()
}
