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

    /// Sound-but-incomplete CPS-based certifier for non-quasihalting,
    /// except that a one-time initial visit to state A is ignored when
    /// the raw control-flow graph proves A cannot be reached again after
    /// the first step.
    ///
    /// Strategy:
    /// 1) Run CPS closure for non-halting (Goal::Halt) while
    ///    recording the CPS transition graph.
    /// 2) For every relevant control-state q, decompose the reachable CPS
    ///    graph induced by state != q into SCCs.  Any eventual path
    ///    avoiding q must become trapped in a cyclic SCC.  If A is
    ///    statically unreachable after the first step, both A nodes and
    ///    the q = A check are omitted from this cycle analysis.
    /// 3) Repeatedly discard recurrent edges whose nonzero read color
    ///    is not written by any remaining recurrent edge.
    /// 4) Reject strictly one-directional SCCs unless their canonical
    ///    `read=0` edges contain a cycle in that direction.  On a blank
    ///    tape, a monotone infinite run must eventually read only fresh
    ///    canonical-zero cells.
    pub fn cps_cant_quasihalt(&self, rad: Radius) -> bool {
        assert!(rad > 1);

        let ignored_state = self.quasihalt_ignored_start_state();

        (2..rad).any(|seg| {
            cps_cant_quasihalt(
                &self.make_transcript_macro(4),
                seg,
                ignored_state,
            ) || cps_cant_quasihalt(
                &self.make_lru_macro(),
                seg,
                ignored_state,
            ) || cps_cant_quasihalt(self, seg, ignored_state)
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

    /// State A is an ignorable initial transient exactly when the concrete
    /// blank-tape first step leaves A and the raw control-flow graph cannot
    /// reach A from that post-step state.  This is computed on the base
    /// program so the same decision can be used by every tape macro.
    fn quasihalt_ignored_start_state(&self) -> Option<u8> {
        let Ok(Some((_, _, post_start))) = self.get_instr(&(0, 0))
        else {
            return None;
        };

        let mut seen = vec![false; s];
        let mut todo = vec![post_start];

        while let Some(state) = todo.pop() {
            let state_idx = usize::from(state);

            // Be conservative for a malformed/out-of-range program.
            if state_idx >= s {
                return None;
            }
            if seen[state_idx] {
                continue;
            }
            if state == 0 {
                return None;
            }

            seen[state_idx] = true;

            #[expect(clippy::cast_possible_truncation)]
            for color in 0..c as Color {
                match self.get_instr(&(state, color)) {
                    Ok(Some((_, _, next_state))) => {
                        todo.push(next_state);
                    },
                    Ok(None) => {},
                    Err(_) => return None,
                }
            }
        }

        Some(0)
    }
}

fn cps_cant_reach(
    prog: &impl GetInstr,
    rad: Radius,
    goal: Goal,
) -> bool {
    cps_cant_reach_obs(prog, rad, goal, &mut NoObs)
}

fn cps_cant_quasihalt(
    prog: &impl GetInstr,
    rad: Radius,
    ignored_state: Option<u8>,
) -> bool {
    let mut g = GraphObs::default();

    if !cps_cant_reach_obs(prog, rad, Halt, &mut g) {
        return false;
    }

    g.dedup_edges();

    // A positive functional result is final.  A negative result still
    // goes through SCC refinement, which may reject a spurious monotone
    // CPS cycle that cannot persist on fresh canonical-zero cells.
    if g.cant_quasihalt_functional_fastpath(ignored_state) == Some(true)
    {
        return true;
    }

    g.cant_quasihalt_no_avoidable_state_cycle(ignored_state)
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

        let continuation_count = if shift {
            configs.rspans.get_continuations(pull).len()
        } else {
            configs.lspans.get_continuations(pull).len()
        };

        for i in 0..continuation_count {
            let Continuation { color, tail_nz } = {
                let continuations = if shift {
                    configs.rspans.get_continuations(pull)
                } else {
                    configs.lspans.get_continuations(pull)
                };

                continuations[i]
            };
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

            obs.edge(&config, &next_config, init_scan, print, shift);

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
    fn edge(
        &mut self,
        _from: &Config,
        _to: &Config,
        _read: Color,
        _write: Color,
        _shift: Shift,
    ) {
    }
}

struct NoObs;
impl CpsObs for NoObs {}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct CpsEdge {
    to: usize,
    read: Color,
    write: Color,
    shift: Shift,
}

#[derive(Default)]
struct GraphObs {
    id: Dict<Config, usize>,
    nodes: Vec<Config>,
    succ: Vec<Vec<CpsEdge>>,
    states: Set<u8>,
}

impl CpsObs for GraphObs {
    fn see(&mut self, c: &Config) {
        self.intern(c);
    }

    fn edge(
        &mut self,
        from: &Config,
        to: &Config,
        read: Color,
        write: Color,
        shift: Shift,
    ) {
        let u = self.intern(from);
        let v = self.intern(to);
        self.succ[u].push(CpsEdge {
            to: v,
            read,
            write,
            shift,
        });
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

    fn cant_quasihalt_functional_fastpath(
        &self,
        ignored_state: Option<u8>,
    ) -> Option<bool> {
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
                // If every non-ignored state seen before the eventual
                // cycle also appears on it, then cannot quasihalt.
                return Some(order[..prev].iter().all(|&u| {
                    let state = self.nodes[u].state;

                    ignored_state == Some(state)
                        || order[prev..]
                            .iter()
                            .any(|&v| self.nodes[v].state == state)
                }));
            }

            seen_step[cur] = Some(order.len());
            order.push(cur);
            cur = self.succ[cur][0].to;
        }

        Some(false)
    }

    fn cant_quasihalt_no_avoidable_state_cycle(
        &self,
        ignored_state: Option<u8>,
    ) -> bool {
        if self.nodes.is_empty() {
            return false;
        }

        let reachable = reachable_nodes(&self.succ, 0);
        let n = self.nodes.len();
        let mut active = vec![false; n];
        let mut in_comp = vec![false; n];
        let mut zero_indeg = vec![0; n];

        // Any infinite path that eventually avoids q must eventually stay
        // inside a cyclic SCC of the graph induced by state != q, with an
        // optional statically transient start state removed throughout.
        for q in self
            .states
            .iter()
            .copied()
            .filter(|&state| ignored_state != Some(state))
        {
            for u in 0..n {
                let state = self.nodes[u].state;
                active[u] = reachable[u]
                    && ignored_state != Some(state)
                    && state != q;
            }

            for comp in sccs_masked(&self.succ, &active) {
                if !scc_has_cycle(&comp, &self.succ) {
                    continue;
                }

                for &u in &comp {
                    in_comp[u] = true;
                }

                let possible_trap = self.scc_can_be_infinite_trap(
                    &comp,
                    &in_comp,
                    &mut zero_indeg,
                );

                for &u in &comp {
                    in_comp[u] = false;
                }

                if possible_trap {
                    return false;
                }
            }
        }

        true
    }

    /// Keep only a greatest fixed point of recurrently supportable edges.
    /// A nonzero color read infinitely often must also be written
    /// infinitely often: only finitely many nonzero cells exist when the
    /// eventual SCC is entered.  Edges outside cyclic SCCs are discarded
    /// before collecting writes so transient writes cannot supply support.
    fn scc_can_be_infinite_trap(
        &self,
        comp: &[usize],
        in_comp: &[bool],
        zero_indeg: &mut [usize],
    ) -> bool {
        let n = self.nodes.len();
        let mut live = vec![vec![]; n];

        for &u in comp {
            live[u] = self.succ[u]
                .iter()
                .map(|edge| in_comp[edge.to])
                .collect();
        }

        loop {
            let subcomps =
                sccs_masked_filtered(&self.succ, in_comp, &live);
            let mut cyclic_id = vec![usize::MAX; n];
            let mut cyclic_comps = vec![];

            for subcomp in subcomps {
                if !scc_has_cycle_filtered(&subcomp, &self.succ, &live)
                {
                    continue;
                }

                let id = cyclic_comps.len();
                for &u in &subcomp {
                    cyclic_id[u] = id;
                }
                cyclic_comps.push(subcomp);
            }

            if cyclic_comps.is_empty() {
                return false;
            }

            // Only edges internal to a cyclic SCC can recur infinitely.
            for &u in comp {
                for (edge_idx, edge) in self.succ[u].iter().enumerate()
                {
                    if live[u][edge_idx]
                        && (cyclic_id[u] == usize::MAX
                            || cyclic_id[u] != cyclic_id[edge.to])
                    {
                        live[u][edge_idx] = false;
                    }
                }
            }

            let mut written: Vec<Set<Color>> = (0..cyclic_comps.len())
                .map(|_| Set::default())
                .collect();
            for &u in comp {
                for (edge_idx, edge) in self.succ[u].iter().enumerate()
                {
                    if live[u][edge_idx] && edge.write != 0 {
                        written[cyclic_id[u]].insert(edge.write);
                    }
                }
            }

            let mut pruned = false;
            for &u in comp {
                for (edge_idx, edge) in self.succ[u].iter().enumerate()
                {
                    if live[u][edge_idx]
                        && edge.read != 0
                        && !written[cyclic_id[u]].contains(&edge.read)
                    {
                        live[u][edge_idx] = false;
                        pruned = true;
                    }
                }
            }

            if pruned {
                continue;
            }

            return cyclic_comps.iter().enumerate().any(
                |(id, recurrent_comp)| {
                    self.recurrent_scc_can_be_infinite_trap(
                        recurrent_comp,
                        &cyclic_id,
                        id,
                        &live,
                        zero_indeg,
                    )
                },
            );
        }
    }

    /// A bidirectional recurrent SCC remains a possible trap.  A strictly
    /// one-directional SCC can trap a run from a blank tape only if its
    /// canonical-zero edges contain a directed cycle in that direction.
    fn recurrent_scc_can_be_infinite_trap(
        &self,
        comp: &[usize],
        cyclic_id: &[usize],
        id: usize,
        live: &[Vec<bool>],
        zero_indeg: &mut [usize],
    ) -> bool {
        let mut has_l = false;
        let mut has_r = false;

        for &u in comp {
            for (edge_idx, edge) in self.succ[u].iter().enumerate() {
                if !live[u][edge_idx] || cyclic_id[edge.to] != id {
                    continue;
                }

                if edge.shift {
                    has_r = true;
                } else {
                    has_l = true;
                }
            }
        }

        if has_l && has_r {
            return true;
        }

        // A cyclic SCC necessarily has an internal edge, so one of these
        // directions must be present.  Keep the conservative answer if an
        // inconsistent graph ever violates that invariant.
        if !has_l && !has_r {
            return true;
        }

        self.has_zero_dir_cycle_in_comp(
            comp, cyclic_id, id, live, zero_indeg, has_r,
        )
    }

    fn has_zero_dir_cycle_in_comp(
        &self,
        comp: &[usize],
        cyclic_id: &[usize],
        id: usize,
        live: &[Vec<bool>],
        indeg: &mut [usize],
        dir_is_r: bool,
    ) -> bool {
        for &u in comp {
            indeg[u] = 0;
        }

        for &u in comp {
            for (edge_idx, edge) in self.succ[u].iter().enumerate() {
                if live[u][edge_idx]
                    && cyclic_id[edge.to] == id
                    && edge.read == 0
                    && edge.shift == dir_is_r
                {
                    indeg[edge.to] += 1;
                }
            }
        }

        let mut stack = Vec::with_capacity(comp.len());
        for &u in comp {
            if indeg[u] == 0 {
                stack.push(u);
            }
        }

        let mut removed = 0;

        while let Some(u) = stack.pop() {
            removed += 1;

            for (edge_idx, edge) in self.succ[u].iter().enumerate() {
                if !live[u][edge_idx]
                    || cyclic_id[edge.to] != id
                    || edge.read != 0
                    || edge.shift != dir_is_r
                {
                    continue;
                }

                indeg[edge.to] -= 1;
                if indeg[edge.to] == 0 {
                    stack.push(edge.to);
                }
            }
        }

        removed != comp.len()
    }
}

fn reachable_nodes(succ: &[Vec<CpsEdge>], start: usize) -> Vec<bool> {
    let n = succ.len();

    let mut vis = vec![false; n];
    let mut stack = vec![];

    vis[start] = true;
    stack.push(start);

    while let Some(u) = stack.pop() {
        for edge in &succ[u] {
            if !vis[edge.to] {
                vis[edge.to] = true;
                stack.push(edge.to);
            }
        }
    }

    vis
}

/// Iterative Kosaraju decomposition restricted to active nodes.
fn sccs_masked(
    succ: &[Vec<CpsEdge>],
    active: &[bool],
) -> Vec<Vec<usize>> {
    sccs_masked_impl(succ, active, None)
}

/// Iterative Kosaraju decomposition restricted to active nodes and live
/// edges.  `live[u]` has the same indexing as `succ[u]` for active nodes.
fn sccs_masked_filtered(
    succ: &[Vec<CpsEdge>],
    active: &[bool],
    live: &[Vec<bool>],
) -> Vec<Vec<usize>> {
    sccs_masked_impl(succ, active, Some(live))
}

fn sccs_masked_impl(
    succ: &[Vec<CpsEdge>],
    active: &[bool],
    live: Option<&[Vec<bool>]>,
) -> Vec<Vec<usize>> {
    let n = succ.len();
    let mut rev = vec![vec![]; n];

    for u in 0..n {
        if !active[u] {
            continue;
        }

        for (edge_idx, edge) in succ[u].iter().enumerate() {
            if edge_is_live(live, u, edge_idx) && active[edge.to] {
                rev[edge.to].push(u);
            }
        }
    }

    let mut seen = vec![false; n];
    let mut order = Vec::with_capacity(n);

    for start in 0..n {
        if !active[start] || seen[start] {
            continue;
        }

        seen[start] = true;
        let mut stack = vec![(start, 0)];

        while let Some((u, edge_idx)) = stack.pop() {
            if edge_idx == succ[u].len() {
                order.push(u);
                continue;
            }

            stack.push((u, edge_idx + 1));
            if !edge_is_live(live, u, edge_idx) {
                continue;
            }

            let v = succ[u][edge_idx].to;

            if active[v] && !seen[v] {
                seen[v] = true;
                stack.push((v, 0));
            }
        }
    }

    let mut assigned = vec![false; n];
    let mut comps = vec![];

    while let Some(start) = order.pop() {
        if assigned[start] {
            continue;
        }

        assigned[start] = true;
        let mut stack = vec![start];
        let mut comp = vec![];

        while let Some(u) = stack.pop() {
            comp.push(u);

            for &v in &rev[u] {
                if !assigned[v] {
                    assigned[v] = true;
                    stack.push(v);
                }
            }
        }

        comps.push(comp);
    }

    comps
}

fn edge_is_live(
    live: Option<&[Vec<bool>]>,
    u: usize,
    edge_idx: usize,
) -> bool {
    live.is_none_or(|live| live[u][edge_idx])
}

fn scc_has_cycle(comp: &[usize], succ: &[Vec<CpsEdge>]) -> bool {
    if comp.len() > 1 {
        return true;
    }

    let u = comp[0];
    succ[u].iter().any(|edge| edge.to == u)
}

fn scc_has_cycle_filtered(
    comp: &[usize],
    succ: &[Vec<CpsEdge>],
    live: &[Vec<bool>],
) -> bool {
    if comp.len() > 1 {
        return true;
    }

    let u = comp[0];
    succ[u]
        .iter()
        .enumerate()
        .any(|(edge_idx, edge)| live[u][edge_idx] && edge.to == u)
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

        let (v, new_last) = {
            let colors = pool.colors(self.span);
            let mut v = Vec::with_capacity(colors.len());

            let new_last =
                if let Some((&last, prefix)) = colors.split_last() {
                    v.push(color);
                    v.extend_from_slice(prefix);
                    last
                } else {
                    color
                };

            (v, new_last)
        };

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

        let (v, pulled) = {
            let colors = pool.colors(self.span);
            let mut v = Vec::with_capacity(colors.len());

            let pulled =
                if let Some((&first, rest)) = colors.split_first() {
                    v.extend_from_slice(rest);
                    v.push(self.last);
                    first
                } else {
                    self.last
                };

            (v, pulled)
        };

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
