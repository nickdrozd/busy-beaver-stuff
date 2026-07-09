use core::fmt;
use std::collections::{VecDeque, hash_map::Entry};

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
            (1..8).any(|tr| {
                cps_cant_quasihalt(
                    &self.make_transcript_macro(tr),
                    seg,
                    ignored_state,
                )
            }) || cps_cant_quasihalt(
                &self.make_lru_macro(),
                seg,
                ignored_state,
            )
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

    while let Some((config_id, config @ Config { state, mut tape })) =
        configs.todo.pop_front()
    {
        obs.see(config_id, &config);

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

            let (next_id, is_new) = configs.intern_config(&next_config);

            obs.edge(config_id, next_id, init_scan, print, shift);

            if is_new {
                configs.todo.push_back((next_id, next_config));
            }
        }

        let pull_key = pull.span;

        let waiting = (config_id, config);

        if shift {
            configs.r_watch.entry(pull_key).or_default().push(waiting);
        } else {
            configs.l_watch.entry(pull_key).or_default().push(waiting);
        }

        if configs.at_capacity() {
            return false;
        }
    }

    true
}

/**************************************/

trait CpsObs {
    fn see(&mut self, _id: ConfigId, _c: &Config) {}
    fn edge(
        &mut self,
        _from: ConfigId,
        _to: ConfigId,
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
    node_states: Vec<u8>,
    succ: Vec<Vec<CpsEdge>>,
    edge_offsets: Vec<usize>,
    states: Set<u8>,
}

impl CpsObs for GraphObs {
    fn see(&mut self, id: ConfigId, c: &Config) {
        if self.succ.len() <= id {
            self.succ.resize_with(id + 1, Vec::new);
            self.node_states.resize(id + 1, 0);
        }

        self.node_states[id] = c.state;
        self.states.insert(c.state);
    }

    fn edge(
        &mut self,
        from: ConfigId,
        to: ConfigId,
        read: Color,
        write: Color,
        shift: Shift,
    ) {
        let needed = from.max(to) + 1;
        if self.succ.len() < needed {
            self.succ.resize_with(needed, Vec::new);
            self.node_states.resize(needed, 0);
        }

        self.succ[from].push(CpsEdge {
            to,
            read,
            write,
            shift,
        });
    }
}

impl GraphObs {
    fn dedup_edges(&mut self) {
        for outs in &mut self.succ {
            outs.sort_unstable();
            outs.dedup();
        }

        self.edge_offsets.clear();
        self.edge_offsets.reserve(self.succ.len() + 1);
        self.edge_offsets.push(0);

        for outs in &self.succ {
            let next =
                self.edge_offsets.last().copied().unwrap() + outs.len();
            self.edge_offsets.push(next);
        }
    }

    #[inline]
    fn edge_id(&self, u: usize, edge_idx: usize) -> usize {
        self.edge_offsets[u] + edge_idx
    }

    fn cant_quasihalt_functional_fastpath(
        &self,
        ignored_state: Option<u8>,
    ) -> Option<bool> {
        if self.node_states.is_empty() {
            return Some(false);
        }

        // If any node has 0 or >1 successors, not functional.
        if self.succ.iter().any(|outs| outs.len() != 1) {
            return None;
        }
        // Follow successors from start node 0 to find the eventual cycle.
        let n = self.node_states.len();
        let mut seen_step: Vec<Option<usize>> = vec![None; n];
        let mut order: Vec<usize> = vec![];

        let mut cur = 0;

        for _ in 0..=n {
            if let Some(prev) = seen_step[cur] {
                // If every non-ignored state seen before the eventual
                // cycle also appears on it, then cannot quasihalt.
                return Some(order[..prev].iter().all(|&u| {
                    let state = self.node_states[u];

                    ignored_state == Some(state)
                        || order[prev..]
                            .iter()
                            .any(|&v| self.node_states[v] == state)
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
        if self.node_states.is_empty() {
            return false;
        }

        let n = self.node_states.len();
        let edge_count = self.edge_offsets.last().copied().unwrap_or(0);

        let mut reachable = vec![false; n];
        let mut reach_stack = Vec::with_capacity(n);
        mark_reachable(&self.succ, 0, &mut reachable, &mut reach_stack);

        let mut active = vec![false; n];
        let mut in_comp = vec![false; n];
        let mut outer_scc = SccScratch::new(n);
        let mut inner_scc = SccScratch::new(n);
        let mut trap = TrapScratch::new(n, edge_count);

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
                let state = self.node_states[u];
                active[u] = reachable[u]
                    && ignored_state != Some(state)
                    && state != q;
            }

            outer_scc.decompose(
                &self.succ,
                &self.edge_offsets,
                &active,
                None,
            );

            for comp_idx in 0..outer_scc.comps.len() {
                let comp = &outer_scc.comps[comp_idx];

                if !scc_has_cycle(comp, &self.succ) {
                    continue;
                }

                for &u in comp {
                    in_comp[u] = true;
                }

                let possible_trap = self.scc_can_be_infinite_trap(
                    comp,
                    &in_comp,
                    &mut inner_scc,
                    &mut trap,
                );

                for &u in comp {
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
        scc: &mut SccScratch,
        scratch: &mut TrapScratch,
    ) -> bool {
        // Reuse the graph-wide liveness bit-vector for every candidate.
        scratch.live.fill(false);

        for &u in comp {
            for (edge_idx, edge) in self.succ[u].iter().enumerate() {
                if in_comp[edge.to] {
                    let edge_id = self.edge_id(u, edge_idx);
                    scratch.live[edge_id] = true;
                }
            }
        }

        loop {
            scc.decompose(
                &self.succ,
                &self.edge_offsets,
                in_comp,
                Some(&scratch.live),
            );

            // Reset only nodes assigned a cyclic ID in the prior iteration.
            while let Some(u) = scratch.cyclic_nodes.pop() {
                scratch.cyclic_id[u] = usize::MAX;
            }
            scratch.cyclic_comp_indices.clear();

            for (comp_idx, subcomp) in scc.comps.iter().enumerate() {
                if !scc_has_cycle_filtered(
                    subcomp,
                    &self.succ,
                    &self.edge_offsets,
                    &scratch.live,
                ) {
                    continue;
                }

                let id = scratch.cyclic_comp_indices.len();
                for &u in subcomp {
                    scratch.cyclic_id[u] = id;
                    scratch.cyclic_nodes.push(u);
                }
                scratch.cyclic_comp_indices.push(comp_idx);
            }

            let cyclic_count = scratch.cyclic_comp_indices.len();
            if cyclic_count == 0 {
                return false;
            }

            // Only edges internal to a cyclic SCC can recur infinitely.
            for &u in comp {
                for (edge_idx, edge) in self.succ[u].iter().enumerate()
                {
                    let edge_id = self.edge_id(u, edge_idx);
                    if scratch.live[edge_id]
                        && (scratch.cyclic_id[u] == usize::MAX
                            || scratch.cyclic_id[u]
                                != scratch.cyclic_id[edge.to])
                    {
                        scratch.live[edge_id] = false;
                    }
                }
            }

            while scratch.written.len() < cyclic_count {
                scratch.written.push(Set::default());
            }
            for written in &mut scratch.written[..cyclic_count] {
                written.clear();
            }

            for &u in comp {
                for (edge_idx, edge) in self.succ[u].iter().enumerate()
                {
                    if scratch.live[self.edge_id(u, edge_idx)]
                        && edge.write != 0
                    {
                        scratch.written[scratch.cyclic_id[u]]
                            .insert(edge.write);
                    }
                }
            }

            let mut pruned = false;
            for &u in comp {
                for (edge_idx, edge) in self.succ[u].iter().enumerate()
                {
                    let edge_id = self.edge_id(u, edge_idx);
                    if scratch.live[edge_id]
                        && edge.read != 0
                        && !scratch.written[scratch.cyclic_id[u]]
                            .contains(&edge.read)
                    {
                        scratch.live[edge_id] = false;
                        pruned = true;
                    }
                }
            }

            if pruned {
                continue;
            }

            for pos in 0..cyclic_count {
                let comp_idx = scratch.cyclic_comp_indices[pos];
                let recurrent_comp = &scc.comps[comp_idx];

                if self.recurrent_scc_can_be_infinite_trap(
                    recurrent_comp,
                    &scratch.cyclic_id,
                    pos,
                    &scratch.live,
                    &mut scratch.zero_indeg,
                    &mut scratch.node_stack,
                ) {
                    return true;
                }
            }

            return false;
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
        live: &[bool],
        zero_indeg: &mut [usize],
        node_stack: &mut Vec<usize>,
    ) -> bool {
        let mut has_l = false;
        let mut has_r = false;

        for &u in comp {
            for (edge_idx, edge) in self.succ[u].iter().enumerate() {
                if !live[self.edge_id(u, edge_idx)]
                    || cyclic_id[edge.to] != id
                {
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
            comp, cyclic_id, id, live, zero_indeg, node_stack, has_r,
        )
    }

    fn has_zero_dir_cycle_in_comp(
        &self,
        comp: &[usize],
        cyclic_id: &[usize],
        id: usize,
        live: &[bool],
        indeg: &mut [usize],
        stack: &mut Vec<usize>,
        dir_is_r: bool,
    ) -> bool {
        for &u in comp {
            indeg[u] = 0;
        }

        for &u in comp {
            for (edge_idx, edge) in self.succ[u].iter().enumerate() {
                if live[self.edge_id(u, edge_idx)]
                    && cyclic_id[edge.to] == id
                    && edge.read == 0
                    && edge.shift == dir_is_r
                {
                    indeg[edge.to] += 1;
                }
            }
        }

        stack.clear();
        if stack.capacity() < comp.len() {
            stack.reserve(comp.len() - stack.capacity());
        }
        for &u in comp {
            if indeg[u] == 0 {
                stack.push(u);
            }
        }

        let mut removed = 0;

        while let Some(u) = stack.pop() {
            removed += 1;

            for (edge_idx, edge) in self.succ[u].iter().enumerate() {
                if !live[self.edge_id(u, edge_idx)]
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

fn mark_reachable(
    succ: &[Vec<CpsEdge>],
    start: usize,
    vis: &mut [bool],
    stack: &mut Vec<usize>,
) {
    vis.fill(false);
    stack.clear();

    if succ.is_empty() {
        return;
    }

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
}

/// Reusable storage for one iterative Kosaraju decomposition.  Reverse
/// adjacency is rebuilt for the current node/edge mask, but its vectors,
/// DFS stacks, visit marks, finishing order, and component buffers retain
/// their allocations across calls.
struct SccScratch {
    generation: u32,
    seen_mark: Vec<u32>,
    assigned_mark: Vec<u32>,
    order: Vec<usize>,
    dfs_stack: Vec<(usize, usize)>,
    node_stack: Vec<usize>,
    rev: Vec<Vec<usize>>,
    comps: Vec<Vec<usize>>,
    spare_comps: Vec<Vec<usize>>,
}

impl SccScratch {
    fn new(n: usize) -> Self {
        Self {
            generation: 0,
            seen_mark: vec![0; n],
            assigned_mark: vec![0; n],
            order: Vec::with_capacity(n),
            dfs_stack: Vec::with_capacity(n),
            node_stack: Vec::with_capacity(n),
            rev: (0..n).map(|_| Vec::new()).collect(),
            comps: Vec::with_capacity(n),
            spare_comps: Vec::with_capacity(n),
        }
    }

    fn next_generation(&mut self) -> u32 {
        self.generation = self.generation.wrapping_add(1);

        if self.generation == 0 {
            self.seen_mark.fill(0);
            self.assigned_mark.fill(0);
            self.generation = 1;
        }

        self.generation
    }

    fn recycle_components(&mut self) {
        while let Some(mut comp) = self.comps.pop() {
            comp.clear();
            self.spare_comps.push(comp);
        }
    }

    /// Iterative Kosaraju decomposition restricted to active nodes and,
    /// when supplied, flat edge liveness indexed by
    /// `edge_offsets[u] + edge_idx`.
    fn decompose(
        &mut self,
        succ: &[Vec<CpsEdge>],
        edge_offsets: &[usize],
        active: &[bool],
        live: Option<&[bool]>,
    ) {
        let n = succ.len();
        debug_assert_eq!(active.len(), n);
        debug_assert_eq!(self.seen_mark.len(), n);
        debug_assert_eq!(self.assigned_mark.len(), n);
        debug_assert_eq!(self.rev.len(), n);

        let generation = self.next_generation();
        self.order.clear();
        self.dfs_stack.clear();
        self.node_stack.clear();
        self.recycle_components();

        // Build only the currently active/live reverse graph while
        // retaining all inner vector capacities between SCC passes.
        for incoming in &mut self.rev {
            incoming.clear();
        }
        for u in 0..n {
            if !active[u] {
                continue;
            }

            for (edge_idx, edge) in succ[u].iter().enumerate() {
                if active[edge.to]
                    && edge_is_live(live, edge_offsets, u, edge_idx)
                {
                    self.rev[edge.to].push(u);
                }
            }
        }

        for start in 0..n {
            if !active[start] || self.seen_mark[start] == generation {
                continue;
            }

            self.seen_mark[start] = generation;
            self.dfs_stack.push((start, 0));

            while let Some((u, edge_idx)) = self.dfs_stack.pop() {
                if edge_idx == succ[u].len() {
                    self.order.push(u);
                    continue;
                }

                self.dfs_stack.push((u, edge_idx + 1));
                if !edge_is_live(live, edge_offsets, u, edge_idx) {
                    continue;
                }

                let v = succ[u][edge_idx].to;
                if active[v] && self.seen_mark[v] != generation {
                    self.seen_mark[v] = generation;
                    self.dfs_stack.push((v, 0));
                }
            }
        }

        while let Some(start) = self.order.pop() {
            if self.assigned_mark[start] == generation {
                continue;
            }

            self.assigned_mark[start] = generation;
            self.node_stack.push(start);

            let mut comp = self.spare_comps.pop().unwrap_or_default();
            comp.clear();

            while let Some(u) = self.node_stack.pop() {
                comp.push(u);

                for &from in &self.rev[u] {
                    if self.assigned_mark[from] == generation {
                        continue;
                    }

                    self.assigned_mark[from] = generation;
                    self.node_stack.push(from);
                }
            }

            self.comps.push(comp);
        }
    }
}

struct TrapScratch {
    live: Vec<bool>,
    cyclic_id: Vec<usize>,
    cyclic_nodes: Vec<usize>,
    cyclic_comp_indices: Vec<usize>,
    written: Vec<Set<Color>>,
    zero_indeg: Vec<usize>,
    node_stack: Vec<usize>,
}

impl TrapScratch {
    fn new(n: usize, edge_count: usize) -> Self {
        Self {
            live: vec![false; edge_count],
            cyclic_id: vec![usize::MAX; n],
            cyclic_nodes: Vec::with_capacity(n),
            cyclic_comp_indices: Vec::with_capacity(n),
            written: vec![],
            zero_indeg: vec![0; n],
            node_stack: Vec::with_capacity(n),
        }
    }
}

#[inline]
fn edge_is_live(
    live: Option<&[bool]>,
    edge_offsets: &[usize],
    u: usize,
    edge_idx: usize,
) -> bool {
    live.is_none_or(|live| live[edge_offsets[u] + edge_idx])
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
    edge_offsets: &[usize],
    live: &[bool],
) -> bool {
    if comp.len() > 1 {
        return true;
    }

    let u = comp[0];
    succ[u].iter().enumerate().any(|(edge_idx, edge)| {
        live[edge_offsets[u] + edge_idx] && edge.to == u
    })
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
type ConfigId = usize;
type PendingConfig = (ConfigId, Config);
type Watch = Dict<SpanId, Vec<PendingConfig>>;

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

    seen: Dict<Config, ConfigId>,
    todo: VecDeque<PendingConfig>,

    l_watch: Watch,
    r_watch: Watch,
}

impl Configs {
    fn init(rad: Radius) -> Self {
        let mut configs = Self {
            span_pool: SpanPool::new(),
            lspans: Dict::new(),
            rspans: Dict::new(),
            seen: Dict::new(),
            todo: VecDeque::new(),
            l_watch: Dict::new(),
            r_watch: Dict::new(),
        };

        let init = Config::init(rad, &mut configs.span_pool);

        configs.lspans.add_span(&init.tape.lspan, init.tape.left_nz);
        configs
            .rspans
            .add_span(&init.tape.rspan, init.tape.right_nz);

        let (init_id, is_new) = configs.intern_config(&init);
        assert!(is_new);
        debug_assert_eq!(init_id, 0);
        configs.todo.push_back((init_id, init));

        configs
    }

    fn intern_config(&mut self, config: &Config) -> (ConfigId, bool) {
        let next_id = self.seen.len();

        match self.seen.entry(config.clone()) {
            Entry::Occupied(entry) => (*entry.get(), false),
            Entry::Vacant(entry) => {
                entry.insert(next_id);
                (next_id, true)
            },
        }
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
