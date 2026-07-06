use core::fmt;
use std::collections::VecDeque;

use ahash::{AHashMap as Dict, AHashSet as Set};

use crate::{Color, Goal, Prog, Shift, config, macros::GetInstr};

use Goal::*;

pub type Radius = usize;

const MAX_DEPTH: usize = 10_000;

/**************************************/

impl<const s: usize, const c: usize> Prog<s, c> {
    pub fn cps_cant_halt(&self, rad: Radius) -> bool {
        self.cps_run_macros(rad, Halt)
    }

    pub fn cps_cant_blank(&self, rad: Radius) -> bool {
        self.cps_run_macros(rad, Blank)
    }

    pub fn cps_cant_spinout(&self, rad: Radius) -> bool {
        self.cps_run_macros(rad, Spinout)
    }

    /// Sound-but-incomplete CPS-based certifier for:
    /// "cannot quasihalt" under your definition:
    ///   no reachable state is visited only finitely often.
    ///
    /// Strategy:
    /// 1) Run CPS closure for non-halting (Goal::Halt) while
    ///    recording the CPS transition graph.
    /// 2) On the resulting finite CPS over-approx graph, certify
    ///    *non-quasihalt* by a sound sufficient condition: - for
    ///    every control-state q, the induced subgraph on reachable
    ///    nodes with state != q is acyclic (has no directed cycle).
    ///    If a directed cycle exists avoiding q, then there exists an
    ///    infinite path that can avoid q after some finite prefix, so
    ///    we cannot certify.
    pub fn cps_cant_quasihalt(&self, rad: Radius) -> bool {
        assert!(rad > 1);

        (2..rad).any(|seg| {
            cps_cant_quasihalt(&self.make_transcript_macro(4), seg)
                || cps_cant_quasihalt(&self.make_lru_macro(), seg)
                || cps_cant_quasihalt(self, seg)
        })
    }

    fn cps_run_macros(&self, rad: Radius, goal: Goal) -> bool {
        assert!(rad > 1);

        (2..rad).any(|seg| {
            cps_cant_reach(&self.make_transcript_macro(16), seg, goal)
                || cps_cant_reach(
                    &self.make_transcript_macro(4),
                    seg,
                    goal,
                )
                || cps_cant_reach(&self.make_lru_macro(), seg, goal)
                || cps_cant_reach(self, seg, goal)
        })
    }
}

fn cps_cant_reach(
    prog: &impl GetInstr,
    rad: Radius,
    goal: Goal,
) -> bool {
    cps_cant_reach_obs(prog, rad, goal, &mut NoObs)
}

fn cps_cant_quasihalt(prog: &impl GetInstr, rad: Radius) -> bool {
    let mut g = GraphObs::default();

    if !cps_cant_reach_obs(prog, rad, Halt, &mut g) {
        return false;
    }

    g.dedup_edges();

    if let Some(ok) = g.cant_quasihalt_functional_fastpath() {
        return ok;
    }

    g.cant_quasihalt_no_avoidable_state_cycle()
}

/**************************************/

const NZ_CAP: u8 = 2;

fn cps_cant_reach_obs(
    prog: &impl GetInstr,
    rad: Radius,
    goal: Goal,
    obs: &mut impl CpsObs,
) -> bool {
    let mut configs = Configs::init(rad);

    while let Some(config @ Config { state, mut tape }) =
        configs.todo.pop_front()
    {
        obs.see(&config);

        let init_scan = tape.scan;

        let (print, shift, next_state) =
            match prog.get_instr(&(state, init_scan)) {
                Err(_) => return false,
                Ok(None) => match goal {
                    Halt => return false,
                    _ => continue,
                },
                Ok(Some(instr)) => instr,
            };

        // The count on the pushed side describes the hidden tail beyond
        // push.last.  Record it together with that continuation color so a
        // later pull cannot combine the color from one occurrence with the
        // tail evidence from another.
        let push_tail_nz =
            if shift { tape.left_nz } else { tape.right_nz };

        let (pull, push): (&mut Span, &mut Span) = if shift {
            (&mut tape.rspan, &mut tape.lspan)
        } else {
            (&mut tape.lspan, &mut tape.rspan)
        };

        configs.add_span(shift, push, push_tail_nz);

        let dropped = push.push(print, &mut configs.span_pool);
        tape.scan = pull.pull(&mut configs.span_pool);

        match goal {
            Halt => {},
            Blank | Spinout => {
                let dropped_nz = match goal {
                    // For blank, nz means nonblank in the base alphabet.
                    Blank => !prog.is_blank(dropped),
                    // Spinout distinguishes canonical macro-zero from
                    // nonzero.  In particular, history-decorated blank
                    // colors are not interchangeable with a canonical
                    // blank ray.
                    Spinout => dropped != 0,
                    Halt => unreachable!(),
                };

                // The pushed-away cell becomes part of the hidden ray.
                // Keep a small saturated lower bound on how many known
                // nonzero cells are hidden on each side.
                if shift {
                    tape.left_nz = tape
                        .left_nz
                        .saturating_add(u8::from(dropped_nz))
                        .min(NZ_CAP);
                } else {
                    tape.right_nz = tape
                        .right_nz
                        .saturating_add(u8::from(dropped_nz))
                        .min(NZ_CAP);
                }
            },
        }

        let continuations = if shift {
            configs.rspans.get_continuations(pull)
        } else {
            configs.lspans.get_continuations(pull)
        }
        .clone();

        for Continuation { color, tail_nz } in continuations {
            let (left_nz, right_nz) = match goal {
                // Halt and quasihalt do not use hidden nonzero evidence.
                // Keeping both counters at zero avoids splitting otherwise
                // identical CPS configurations.
                Halt => (0, 0),
                Blank | Spinout => {
                    let entered_nz = match goal {
                        Blank => !prog.is_blank(color),
                        Spinout => color != 0,
                        Halt => unreachable!(),
                    };

                    // The current count constrains the whole hidden ray:
                    // after its first color enters the represented window,
                    // at least current_nz - entered_nz remain.  The learned
                    // continuation independently supplies a lower bound for
                    // that exact tail.  Both facts hold, so retain their max.
                    if shift {
                        (
                            tape.left_nz,
                            tail_nz.max(
                                tape.right_nz.saturating_sub(u8::from(
                                    entered_nz,
                                )),
                            ),
                        )
                    } else {
                        (
                            tail_nz.max(
                                tape.left_nz.saturating_sub(u8::from(
                                    entered_nz,
                                )),
                            ),
                            tape.right_nz,
                        )
                    }
                },
            };

            let reached_goal = match goal {
                Blank => {
                    prog.is_blank(color)
                        && prog.is_blank(tape.scan)
                        && pull
                            .base_blank_span(prog, &configs.span_pool)
                        && left_nz == 0
                        && right_nz == 0
                        && push.base_all_blank(prog, &configs.span_pool)
                },
                Spinout => {
                    // The transition must itself be the state's blank
                    // transition.  Merely entering a blank ray from a
                    // nonblank cell does not imply that the next step
                    // repeats the same instruction.  Any retained nonzero
                    // evidence ahead also rules out a canonical blank ray.
                    let ahead_nz =
                        if shift { right_nz } else { left_nz };

                    init_scan == 0
                        && color == 0
                        && tape.scan == 0
                        && pull.blank_span(&configs.span_pool)
                        && ahead_nz == 0
                        && state == next_state
                },
                Halt => false,
            };

            if reached_goal {
                return false;
            }

            let mut pull_clone = *pull;
            pull_clone.last = color;

            let next_tape = Tape::from_spans(
                tape.scan, *push, pull_clone, shift, left_nz, right_nz,
            );

            let next_config = Config {
                state: next_state,
                tape: next_tape,
            };

            obs.edge(&config, &next_config);

            if configs.intern_config(&next_config) {
                configs.todo.push_back(next_config);
            }
        }

        let pull_key = pull.span;

        if shift {
            configs.r_watch.entry(pull_key).or_default().push(config);
        } else {
            configs.l_watch.entry(pull_key).or_default().push(config);
        }

        if configs.at_capacity() {
            return false;
        }
    }

    true
}

/**************************************/

trait CpsObs {
    fn see(&mut self, _c: &Config) {}
    fn edge(&mut self, _from: &Config, _to: &Config) {}
}

struct NoObs;
impl CpsObs for NoObs {}

#[derive(Default)]
struct GraphObs {
    id: Dict<Config, usize>,
    nodes: Vec<Config>,
    succ: Vec<Vec<usize>>,
    states: Set<u8>,
}

impl CpsObs for GraphObs {
    fn see(&mut self, c: &Config) {
        self.intern(c);
    }

    fn edge(&mut self, from: &Config, to: &Config) {
        let u = self.intern(from);
        let v = self.intern(to);
        self.succ[u].push(v);
    }
}

impl GraphObs {
    fn intern(&mut self, c: &Config) -> usize {
        if let Some(&i) = self.id.get(c) {
            return i;
        }

        let i = self.nodes.len();
        self.nodes.push(c.clone());
        self.id.insert(c.clone(), i);
        self.succ.push(vec![]);
        self.states.insert(c.state);

        i
    }

    fn dedup_edges(&mut self) {
        for outs in &mut self.succ {
            outs.sort_unstable();
            outs.dedup();
        }
    }

    fn cant_quasihalt_functional_fastpath(&self) -> Option<bool> {
        if self.nodes.is_empty() {
            return Some(false);
        }

        // If any node has 0 or >1 successors, not functional.
        if self.succ.iter().any(|outs| outs.len() != 1) {
            return None;
        }
        // Follow successors from start node 0 to find the eventual cycle.
        let n = self.nodes.len();
        let mut seen_step: Vec<Option<usize>> = vec![None; n];
        let mut order: Vec<usize> = vec![];

        let mut cur = 0;

        for _ in 0..=n {
            if let Some(prev) = seen_step[cur] {
                // If all states ever observed appear on the eventual
                // cycle, then cannot quasihalt.
                return Some(order[..prev].iter().all(|&u| {
                    let s = self.nodes[u].state;
                    order[prev..]
                        .iter()
                        .any(|&v| self.nodes[v].state == s)
                }));
            }

            seen_step[cur] = Some(order.len());
            order.push(cur);
            cur = self.succ[cur][0];
        }

        Some(false)
    }

    fn cant_quasihalt_no_avoidable_state_cycle(&self) -> bool {
        if self.nodes.is_empty() {
            return false;
        }

        let reachable = reachable_nodes(&self.succ, 0);

        // For each control-state q, check whether there exists any
        // reachable directed cycle comprised only of nodes whose
        // control-state != q.
        for q in &self.states {
            if self.induced_has_cycle_avoiding_q(&reachable, *q) {
                return false;
            }
        }

        true
    }

    fn induced_has_cycle_avoiding_q(
        &self,
        reachable: &[bool],
        q: u8,
    ) -> bool {
        let n = self.nodes.len();

        let mut include = vec![false; n];
        let mut count = 0;

        for u in 0..n {
            if reachable[u] && self.nodes[u].state != q {
                include[u] = true;
                count += 1;
            }
        }

        if count == 0 {
            return false;
        }

        // Kahn-style cycle detection on the induced subgraph.
        let mut indeg = vec![0; n];

        for u in 0..n {
            if !include[u] {
                continue;
            }

            for &v in &self.succ[u] {
                if include[v] {
                    indeg[v] += 1;
                }
            }
        }

        let mut stack = vec![];

        for u in 0..n {
            if include[u] && indeg[u] == 0 {
                stack.push(u);
            }
        }

        let mut removed = 0;

        while let Some(u) = stack.pop() {
            removed += 1;

            for &v in &self.succ[u] {
                if !include[v] {
                    continue;
                }

                let d = indeg[v];

                if d > 0 {
                    indeg[v] = d - 1;
                    if indeg[v] == 0 {
                        stack.push(v);
                    }
                }
            }
        }

        removed != count
    }
}

fn reachable_nodes(succ: &[Vec<usize>], start: usize) -> Vec<bool> {
    let n = succ.len();

    let mut vis = vec![false; n];
    let mut stack = vec![];

    vis[start] = true;

    stack.push(start);

    while let Some(u) = stack.pop() {
        for &v in &succ[u] {
            if !vis[v] {
                vis[v] = true;
                stack.push(v);
            }
        }
    }
    vis
}

/**************************************/

type SpanId = usize;

type Colors = Vec<Color>;

#[derive(Clone, Copy, PartialEq, Eq)]
struct Continuation {
    color: Color,
    tail_nz: u8,
}

type Continuations = Vec<Continuation>;
type Spans = Dict<SpanId, Continuations>;
type Watch = Dict<SpanId, Vec<Config>>;

/**************************************/

struct SpanPool {
    spans: Vec<Colors>,
    index: Dict<Colors, SpanId>,

    push_cache: Dict<(SpanId, Color, Color), Span>,
    pull_cache: Dict<(SpanId, Color), (Span, Color)>,
}

impl SpanPool {
    fn new() -> Self {
        Self {
            spans: vec![],
            index: Dict::new(),
            push_cache: Dict::new(),
            pull_cache: Dict::new(),
        }
    }

    fn intern(&mut self, colors: Colors) -> SpanId {
        if let Some(&id) = self.index.get(&colors) {
            return id;
        }

        let id = self.spans.len();

        self.spans.push(colors.clone());
        self.index.insert(colors, id);

        id
    }

    fn colors(&self, id: SpanId) -> &Colors {
        &self.spans[id]
    }
}

/**************************************/

struct Configs {
    span_pool: SpanPool,

    lspans: Spans,
    rspans: Spans,

    seen: Set<Config>,
    todo: VecDeque<Config>,

    l_watch: Watch,
    r_watch: Watch,
}

impl Configs {
    fn init(rad: Radius) -> Self {
        let mut configs = Self {
            span_pool: SpanPool::new(),
            lspans: Dict::new(),
            rspans: Dict::new(),
            seen: Set::new(),
            todo: VecDeque::new(),
            l_watch: Dict::new(),
            r_watch: Dict::new(),
        };

        let init = Config::init(rad, &mut configs.span_pool);

        configs.lspans.add_span(&init.tape.lspan, init.tape.left_nz);
        configs
            .rspans
            .add_span(&init.tape.rspan, init.tape.right_nz);

        assert!(configs.intern_config(&init));
        configs.todo.push_back(init);

        configs
    }

    fn intern_config(&mut self, config: &Config) -> bool {
        self.seen.insert(config.clone())
    }

    fn at_capacity(&self) -> bool {
        MAX_DEPTH < self.seen.len()
    }

    fn add_span(&mut self, shift: Shift, span: &Span, tail_nz: u8) {
        let (spans, watch) = if shift {
            (&mut self.lspans, &mut self.l_watch)
        } else {
            (&mut self.rspans, &mut self.r_watch)
        };

        if spans.add_span(span, tail_nz)
            && let Some(waiting) = watch.remove(&span.span)
        {
            self.todo.extend(waiting);
        }
    }
}

/**************************************/

trait AddSpan {
    fn add_span(&mut self, span: &Span, tail_nz: u8) -> bool;
    fn get_continuations(&self, span: &Span) -> &Continuations;
}

impl AddSpan for Spans {
    fn add_span(&mut self, span: &Span, tail_nz: u8) -> bool {
        let continuation = Continuation {
            color: span.last,
            tail_nz,
        };

        if let Some(continuations) = self.get_mut(&span.span) {
            match continuations
                .binary_search_by_key(&span.last, |cnt| cnt.color)
            {
                Ok(pos) => {
                    // Counts are lower bounds.  For the same continuation
                    // color, a smaller bound subsumes every larger one.
                    if tail_nz < continuations[pos].tail_nz {
                        continuations[pos].tail_nz = tail_nz;
                        return true;
                    }
                    return false;
                },
                Err(pos) => {
                    continuations.insert(pos, continuation);
                    return true;
                },
            }
        }

        self.insert(span.span, vec![continuation]);
        true
    }

    fn get_continuations(&self, span: &Span) -> &Continuations {
        self.get(&span.span).unwrap()
    }
}

/**************************************/

type Config = config::Config<Tape>;

impl Config {
    fn init(rad: Radius, pool: &mut SpanPool) -> Self {
        Self {
            state: 0,
            tape: Tape::init(rad, pool),
        }
    }
}

/**************************************/

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Tape {
    scan: Color,
    lspan: Span,
    rspan: Span,
    left_nz: u8,
    right_nz: u8,
}

impl Tape {
    fn init(rad: Radius, pool: &mut SpanPool) -> Self {
        Self {
            scan: 0,
            lspan: Span::init(rad, pool),
            rspan: Span::init(rad, pool),
            left_nz: 0,
            right_nz: 0,
        }
    }

    const fn from_spans(
        scan: Color,
        push: Span,
        pull: Span,
        shift: Shift,
        left_nz: u8,
        right_nz: u8,
    ) -> Self {
        let (lspan, rspan) =
            if shift { (push, pull) } else { (pull, push) };

        Self {
            scan,
            lspan,
            rspan,
            left_nz,
            right_nz,
        }
    }
}

impl config::Scan for Tape {
    fn scan(&self) -> Color {
        self.scan
    }
}

impl fmt::Display for Tape {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "L(pat={}, last={}, nz={}) [{}] R(pat={}, last={}, nz={})",
            self.lspan.span,
            self.lspan.last,
            self.left_nz,
            self.scan,
            self.rspan.span,
            self.rspan.last,
            self.right_nz
        )
    }
}

/**************************************/

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Span {
    span: SpanId,
    last: Color,
}

impl Span {
    fn init(rad: Radius, pool: &mut SpanPool) -> Self {
        assert!(rad > 0);

        Self {
            span: pool.intern(vec![0; rad - 1]),
            last: 0,
        }
    }

    fn push(&mut self, color: Color, pool: &mut SpanPool) -> Color {
        let key = (self.span, self.last, color);

        if let Some(&new_span) = pool.push_cache.get(&key) {
            let dropped = self.last;
            *self = new_span;
            return dropped;
        }

        let mut v = pool.colors(self.span).clone();

        v.insert(0, color);
        let new_last = v.pop().unwrap();

        let new_span = Self {
            span: pool.intern(v),
            last: new_last,
        };

        pool.push_cache.insert(key, new_span);
        let dropped = self.last;
        *self = new_span;
        dropped
    }

    fn pull(&mut self, pool: &mut SpanPool) -> Color {
        let key = (self.span, self.last);

        if let Some(&(new_span, pulled)) = pool.pull_cache.get(&key) {
            *self = new_span;
            return pulled;
        }

        let mut v = pool.colors(self.span).clone();

        v.push(self.last);
        let pulled = v.remove(0);

        let new_span = Self {
            span: pool.intern(v),
            last: self.last,
        };

        pool.pull_cache.insert(key, (new_span, pulled));
        *self = new_span;
        pulled
    }

    fn blank_span(&self, pool: &SpanPool) -> bool {
        pool.colors(self.span).iter().all(|&c| c == 0)
    }

    fn base_blank_span(
        &self,
        prog: &impl GetInstr,
        pool: &SpanPool,
    ) -> bool {
        pool.colors(self.span).iter().all(|&c| prog.is_blank(c))
    }

    fn base_all_blank(
        &self,
        prog: &impl GetInstr,
        pool: &SpanPool,
    ) -> bool {
        prog.is_blank(self.last) && self.base_blank_span(prog, pool)
    }
}

#[test]
fn test_span() {
    let mut pool = SpanPool::new();
    let mut span = Span::init(3, &mut pool);

    assert_eq!(pool.colors(span.span).as_slice(), &[0, 0]);
    assert_eq!(span.last, 0);

    span.push(1, &mut pool);
    span.push(1, &mut pool);
    span.push(0, &mut pool);

    assert_eq!(pool.colors(span.span).as_slice(), &[0, 1]);
    assert_eq!(span.last, 1);
}
