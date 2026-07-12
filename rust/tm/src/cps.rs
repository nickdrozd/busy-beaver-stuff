use core::fmt;
use std::collections::hash_map::Entry;

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
        let mut configs = Configs::new(Halt);
        let mut qh = QuasihaltScratch::default();

        (2..rad).any(|seg| {
            (1..8).any(|tr| {
                cps_cant_quasihalt(
                    &self.make_transcript_macro(tr),
                    seg,
                    ignored_state,
                    &mut configs,
                    &mut qh,
                )
            }) || cps_cant_quasihalt(
                &self.make_lru_macro(),
                seg,
                ignored_state,
                &mut configs,
                &mut qh,
            )
        })
    }

    fn cps_run_macros(&self, rad: Radius, goal: Goal) -> bool {
        assert!(rad > 1);

        let mut configs = Configs::new(goal);

        (2..rad).any(|seg| {
            cps_cant_reach(
                &self.make_transcript_macro(16),
                seg,
                goal,
                &mut configs,
            ) || cps_cant_reach(
                &self.make_transcript_macro(4),
                seg,
                goal,
                &mut configs,
            ) || cps_cant_reach(
                &self.make_lru_macro(),
                seg,
                goal,
                &mut configs,
            ) || cps_cant_reach(self, seg, goal, &mut configs)
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
    configs: &mut Configs,
) -> bool {
    cps_cant_reach_obs(prog, rad, goal, &mut NoObs, configs)
}

fn cps_cant_quasihalt(
    prog: &impl GetInstr,
    rad: Radius,
    ignored_state: Option<u8>,
    configs: &mut Configs,
    scratch: &mut QuasihaltScratch,
) -> bool {
    scratch.graph.reset();

    if !cps_cant_reach_obs(prog, rad, Halt, &mut scratch.graph, configs)
    {
        return false;
    }

    scratch.graph.dedup_edges();

    // A positive functional result is final.  A negative result still
    // goes through SCC refinement, which may reject a spurious monotone
    // CPS cycle that cannot persist on fresh canonical-zero cells.
    if scratch.graph.cant_quasihalt_functional_fastpath(
        ignored_state,
        &mut scratch.analysis,
    ) == Some(true)
    {
        return true;
    }

    scratch.graph.cant_quasihalt_no_avoidable_state_cycle(
        ignored_state,
        &mut scratch.analysis,
    )
}

/**************************************/

const fn parity_lower_bound(nz: usize, odd: bool) -> usize {
    let nz_odd = nz & 1 != 0;
    if nz_odd == odd {
        nz
    } else {
        nz.saturating_add(1)
    }
}

fn cps_cant_reach_obs(
    prog: &impl GetInstr,
    rad: Radius,
    goal: Goal,
    obs: &mut impl CpsObs,
    configs: &mut Configs,
) -> bool {
    configs.reset(rad);

    while configs.todo_head < configs.todo.len() {
        let config_id = configs.todo[configs.todo_head];
        configs.todo_head += 1;
        if !configs.is_active(config_id) {
            continue;
        }

        let (state, mut tape) = {
            let config = &configs.by_id[config_id];
            obs.see(config_id, config);
            (config.state, config.tape)
        };

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

        // The summary on the pushed side describes the hidden tail beyond
        // push.last.  Record it together with that continuation color so a
        // later pull cannot combine the color from one occurrence with the
        // tail evidence from another.
        let push_tail_nz =
            if shift { tape.left_nz } else { tape.right_nz };
        let push_tail_parity = match goal {
            Blank | Spinout if shift => tape.left_parity,
            Blank | Spinout => tape.right_parity,
            Halt => false,
        };

        let (pull, push): (&mut Span, &mut Span) = if shift {
            (&mut tape.rspan, &mut tape.lspan)
        } else {
            (&mut tape.lspan, &mut tape.rspan)
        };

        configs.add_span(shift, push, push_tail_nz, push_tail_parity);

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
                // Keep an uncapped lower bound on how many known nonzero
                // cells are hidden on each side.
                if shift {
                    tape.left_nz =
                        tape.left_nz.saturating_add(dropped_nz.into());

                    tape.left_parity ^= dropped_nz;
                } else {
                    tape.right_nz =
                        tape.right_nz.saturating_add(dropped_nz.into());

                    tape.right_parity ^= dropped_nz;
                }
            },
        }

        // These whole-window predicates do not depend on which
        // continuation enters the represented window.  Computing them once
        // avoids rescanning both spans for every continuation.
        let blank_window = match goal {
            Blank => {
                prog.is_blank(tape.scan)
                    && pull
                        .base_blank_span(prog, &mut configs.span_pool)
                    && push.base_all_blank(prog, &mut configs.span_pool)
            },
            Spinout => {
                init_scan == 0
                    && tape.scan == 0
                    && pull.blank_span(&configs.span_pool)
                    && state == next_state
            },
            Halt => false,
        };

        let Configs {
            lspans,
            rspans,
            interner,
            by_id,
            active,
            active_count,
            todo,
            ..
        } = &mut *configs;

        macro_rules! process_continuation {
            ($color:expr, $tail_nz:expr, $tail_parity:expr) => {{
                let color = $color;
                let tail_nz = $tail_nz;
                let tail_parity = $tail_parity;

                let (left_nz, right_nz, left_parity, right_parity) =
                    match goal {
                        // Halt and quasihalt do not use hidden nonzero evidence.
                        // Keeping all summaries zero avoids splitting otherwise
                        // identical CPS configurations.
                        Halt => (0, 0, false, false),
                        Blank => {
                            let entered_nz = !prog.is_blank(color);

                            // The current hidden ray consists of the entering
                            // color followed by the continuation's hidden tail.
                            // Exact base-nonblank parities must agree.
                            let current_parity = if shift {
                                tape.right_parity
                            } else {
                                tape.left_parity
                            };

                            if current_parity
                                != (entered_nz ^ tail_parity)
                            {
                                continue;
                            }

                            // Normalize each lower bound to the least count with
                            // the required exact parity before removing the
                            // entering color, then combine it with the
                            // continuation's independently learned tail bound.
                            let current_nz = if shift {
                                tape.right_nz
                            } else {
                                tape.left_nz
                            };
                            let current_tail_nz = parity_lower_bound(
                                current_nz,
                                current_parity,
                            )
                            .saturating_sub(entered_nz.into());
                            let continuation_tail_nz =
                                parity_lower_bound(tail_nz, tail_parity);
                            let next_tail_nz =
                                current_tail_nz.max(continuation_tail_nz);

                            // The current count constrains the whole hidden ray:
                            // after its first color enters the represented window,
                            // at least current_nz - entered_nz remain.  The learned
                            // continuation independently supplies a lower bound for
                            // that exact tail.  Both facts hold, so retain their max.
                            if shift {
                                (
                                    tape.left_nz,
                                    next_tail_nz,
                                    tape.left_parity,
                                    tail_parity,
                                )
                            } else {
                                (
                                    next_tail_nz,
                                    tape.right_nz,
                                    tail_parity,
                                    tape.right_parity,
                                )
                            }
                        },
                        Spinout => {
                            let entered_nz = color != 0;

                            // For Spinout, parity counts non-canonical-zero macro
                            // cells.  The current hidden ray consists of the
                            // entering color followed by the continuation tail,
                            // so their exact parities must agree.
                            let current_parity = if shift {
                                tape.right_parity
                            } else {
                                tape.left_parity
                            };

                            if current_parity
                                != (entered_nz ^ tail_parity)
                            {
                                continue;
                            }

                            // As for Blank, normalize the lower bound to its exact
                            // parity before removing the entering color, then
                            // merge that fact with the continuation's tail
                            // evidence.
                            let current_nz = if shift {
                                tape.right_nz
                            } else {
                                tape.left_nz
                            };
                            let current_tail_nz = parity_lower_bound(
                                current_nz,
                                current_parity,
                            )
                            .saturating_sub(entered_nz.into());
                            let continuation_tail_nz =
                                parity_lower_bound(tail_nz, tail_parity);
                            let next_tail_nz =
                                current_tail_nz.max(continuation_tail_nz);

                            if shift {
                                (
                                    tape.left_nz,
                                    next_tail_nz,
                                    tape.left_parity,
                                    tail_parity,
                                )
                            } else {
                                (
                                    next_tail_nz,
                                    tape.right_nz,
                                    tail_parity,
                                    tape.right_parity,
                                )
                            }
                        },
                    };

                let reached_goal = match goal {
                    Blank => {
                        blank_window
                            && prog.is_blank(color)
                            && left_nz == 0
                            && right_nz == 0
                            && !left_parity
                            && !right_parity
                    },
                    Spinout => {
                        // The transition must itself be the state's blank
                        // transition.  Merely entering a blank ray from a
                        // nonblank cell does not imply that the next step
                        // repeats the same instruction.  Any retained nonzero
                        // evidence ahead also rules out a canonical blank ray.
                        let (ahead_nz, ahead_parity) = if shift {
                            (right_nz, right_parity)
                        } else {
                            (left_nz, left_parity)
                        };

                        blank_window
                            && color == 0
                            && ahead_nz == 0
                            && !ahead_parity
                    },
                    Halt => false,
                };

                if reached_goal {
                    return false;
                }

                let mut pull_clone = *pull;
                pull_clone.last = color;

                let next_tape = Tape::from_spans(
                    tape.scan,
                    *push,
                    pull_clone,
                    shift,
                    left_nz,
                    right_nz,
                    left_parity,
                    right_parity,
                );

                let next_config = Config {
                    state: next_state,
                    tape: next_tape,
                };

                let (next_id, is_new) = interner.intern(
                    next_config,
                    by_id,
                    active,
                    active_count,
                );

                obs.edge(config_id, next_id, init_scan, print, shift);

                if is_new {
                    todo.push(next_id);
                }
            }};
        }

        match goal {
            Halt => {
                let colors = if shift {
                    rspans.get_halt_colors(pull)
                } else {
                    lspans.get_halt_colors(pull)
                };

                for &color in colors {
                    process_continuation!(color, 0, false);
                }
            },
            Blank | Spinout => {
                let continuations = if shift {
                    rspans.get_continuations(pull)
                } else {
                    lspans.get_continuations(pull)
                };

                for &Continuation {
                    color,
                    tail_nz,
                    tail_parity,
                } in continuations
                {
                    process_continuation!(color, tail_nz, tail_parity);
                }
            },
        }

        let pull_key = pull.span;

        if configs.is_active(config_id) {
            let watch = if shift {
                &mut configs.r_watch
            } else {
                &mut configs.l_watch
            };

            if watch.len() <= pull_key {
                watch.resize_with(pull_key + 1, Vec::new);
            }
            watch[pull_key].push(config_id);
        }

        if configs
            .interner
            .at_capacity(configs.by_id.len(), configs.active_count)
        {
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
    node_count: usize,
    node_states: Vec<u8>,
    succ: Vec<Vec<CpsEdge>>,
    edge_offsets: Vec<usize>,
    states: Set<u8>,
}

#[derive(Default)]
struct QuasihaltScratch {
    graph: GraphObs,
    analysis: QuasihaltAnalysisScratch,
}

struct QuasihaltAnalysisScratch {
    seen_step: Vec<Option<usize>>,
    order: Vec<usize>,
    reachable: Vec<bool>,
    reach_stack: Vec<usize>,
    active: Vec<bool>,
    in_comp: Vec<bool>,
    outer_scc: SccScratch,
    inner_scc: SccScratch,
    trap: TrapScratch,
}

impl Default for QuasihaltAnalysisScratch {
    fn default() -> Self {
        Self {
            seen_step: vec![],
            order: vec![],
            reachable: vec![],
            reach_stack: vec![],
            active: vec![],
            in_comp: vec![],
            outer_scc: SccScratch::new(0),
            inner_scc: SccScratch::new(0),
            trap: TrapScratch::new(0, 0),
        }
    }
}

impl QuasihaltAnalysisScratch {
    fn prepare_functional(&mut self, n: usize) {
        if self.seen_step.len() < n {
            self.seen_step.resize(n, None);
        }
        self.seen_step[..n].fill(None);
        self.order.clear();
    }

    fn prepare_graph(&mut self, n: usize, edge_count: usize) {
        if self.reachable.len() < n {
            self.reachable.resize(n, false);
            self.active.resize(n, false);
            self.in_comp.resize(n, false);
        }

        self.reachable[..n].fill(false);
        self.active[..n].fill(false);
        self.in_comp[..n].fill(false);
        self.reach_stack.clear();

        self.outer_scc.ensure_size(n);
        self.inner_scc.ensure_size(n);
        self.trap.ensure_size(n, edge_count);
    }
}

impl CpsObs for GraphObs {
    fn see(&mut self, id: ConfigId, c: &Config) {
        self.ensure_node(id);
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
        self.ensure_node(from.max(to));

        self.succ[from].push(CpsEdge {
            to,
            read,
            write,
            shift,
        });
    }
}

impl GraphObs {
    fn reset(&mut self) {
        let old_node_count = self.node_count;

        self.node_count = 0;
        self.node_states.clear();
        self.edge_offsets.clear();
        self.states.clear();

        // Only nodes used by the immediately previous graph can contain
        // live edges. Retain allocations outside that prefix untouched.
        for outs in &mut self.succ[..old_node_count] {
            outs.clear();
        }
    }

    fn ensure_node(&mut self, id: usize) {
        let needed = id + 1;

        if self.succ.len() < needed {
            self.succ.resize_with(needed, Vec::new);
        }
        if self.node_states.len() < needed {
            self.node_states.resize(needed, 0);
        }

        self.node_count = self.node_count.max(needed);
    }

    fn dedup_edges(&mut self) {
        for outs in &mut self.succ[..self.node_count] {
            outs.sort_unstable();
            outs.dedup();
        }

        self.edge_offsets.clear();
        self.edge_offsets.reserve(self.node_count + 1);
        self.edge_offsets.push(0);

        for outs in &self.succ[..self.node_count] {
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
        scratch: &mut QuasihaltAnalysisScratch,
    ) -> Option<bool> {
        let n = self.node_count;
        if n == 0 {
            return Some(false);
        }

        // If any node has 0 or >1 successors, not functional.
        if self.succ[..n].iter().any(|outs| outs.len() != 1) {
            return None;
        }

        scratch.prepare_functional(n);
        let seen_step = &mut scratch.seen_step;
        let order = &mut scratch.order;

        // Follow successors from start node 0 to find the eventual cycle.
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
        scratch: &mut QuasihaltAnalysisScratch,
    ) -> bool {
        let n = self.node_count;
        if n == 0 {
            return false;
        }

        let edge_count = self.edge_offsets.last().copied().unwrap_or(0);
        scratch.prepare_graph(n, edge_count);

        mark_reachable(
            &self.succ[..n],
            0,
            &mut scratch.reachable[..n],
            &mut scratch.reach_stack,
        );

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
                scratch.active[u] = scratch.reachable[u]
                    && ignored_state != Some(state)
                    && state != q;
            }

            scratch.outer_scc.decompose(
                &self.succ[..n],
                &self.edge_offsets,
                &scratch.active[..n],
                None,
            );

            for comp_idx in 0..scratch.outer_scc.comps.len() {
                let comp = &scratch.outer_scc.comps[comp_idx];

                if !scc_has_cycle(comp, &self.succ[..n]) {
                    continue;
                }

                for &u in comp {
                    scratch.in_comp[u] = true;
                }

                let possible_trap = self.scc_can_be_infinite_trap(
                    comp,
                    &scratch.in_comp[..n],
                    &mut scratch.inner_scc,
                    &mut scratch.trap,
                );

                for &u in comp {
                    scratch.in_comp[u] = false;
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

        let n = in_comp.len();

        loop {
            scc.decompose(
                &self.succ[..n],
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
                    &self.succ[..n],
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

    fn ensure_size(&mut self, n: usize) {
        if self.seen_mark.len() < n {
            self.seen_mark.resize(n, 0);
            self.assigned_mark.resize(n, 0);
            self.rev.resize_with(n, Vec::new);
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
        debug_assert!(self.seen_mark.len() >= n);
        debug_assert!(self.assigned_mark.len() >= n);
        debug_assert!(self.rev.len() >= n);

        let generation = self.next_generation();
        self.order.clear();
        self.dfs_stack.clear();
        self.node_stack.clear();
        self.recycle_components();

        // Build only the currently active/live reverse graph while
        // retaining all inner vector capacities between SCC passes.
        for incoming in &mut self.rev[..n] {
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

    fn ensure_size(&mut self, n: usize, edge_count: usize) {
        if self.live.len() < edge_count {
            self.live.resize(edge_count, false);
        }
        self.live.fill(false);

        if self.cyclic_id.len() < n {
            self.cyclic_id.resize(n, usize::MAX);
            self.zero_indeg.resize(n, 0);
        }
        self.cyclic_id[..n].fill(usize::MAX);
        self.zero_indeg[..n].fill(0);

        self.cyclic_nodes.clear();
        self.cyclic_comp_indices.clear();
        self.node_stack.clear();
        for written in &mut self.written {
            written.clear();
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
    tail_nz: usize,
    tail_parity: bool,
}

type Continuations = Vec<Continuation>;
type RichSpans = Vec<Continuations>;
type HaltSpans = Vec<Vec<Color>>;

enum Spans {
    Halt(HaltSpans),
    Rich(RichSpans),
}

type ConfigId = usize;
type Watch = Vec<Vec<ConfigId>>;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct HaltConfigShape {
    state: u8,
    scan: Color,
    lspan: Span,
    rspan: Span,
}

impl From<&Config> for HaltConfigShape {
    fn from(config: &Config) -> Self {
        Self {
            state: config.state,
            scan: config.tape.scan,
            lspan: config.tape.lspan,
            rspan: config.tape.rspan,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct ConfigShape {
    state: u8,
    scan: Color,
    lspan: Span,
    rspan: Span,
    left_parity: bool,
    right_parity: bool,
}

impl From<&Config> for ConfigShape {
    fn from(config: &Config) -> Self {
        Self {
            state: config.state,
            scan: config.tape.scan,
            lspan: config.tape.lspan,
            rspan: config.tape.rspan,
            left_parity: config.tape.left_parity,
            right_parity: config.tape.right_parity,
        }
    }
}

#[derive(Clone, Copy)]
struct AntichainEntry {
    left_nz: usize,
    right_nz: usize,
    id: ConfigId,
}

enum ConfigInterner {
    Exact(Dict<HaltConfigShape, ConfigId>),
    Antichain {
        seen: Dict<ConfigShape, Vec<AntichainEntry>>,
        spare_entries: Vec<Vec<AntichainEntry>>,
    },
}

impl ConfigInterner {
    fn intern(
        &mut self,
        config: Config,
        by_id: &mut Vec<Config>,
        active: &mut Vec<bool>,
        active_count: &mut usize,
    ) -> (ConfigId, bool) {
        match self {
            Self::Exact(seen) => {
                debug_assert_eq!(config.tape.left_nz, 0);
                debug_assert_eq!(config.tape.right_nz, 0);
                debug_assert!(!config.tape.left_parity);
                debug_assert!(!config.tape.right_parity);

                let shape = HaltConfigShape::from(&config);
                match seen.entry(shape) {
                    Entry::Occupied(entry) => (*entry.get(), false),
                    Entry::Vacant(entry) => {
                        let next_id = by_id.len();
                        by_id.push(config);
                        entry.insert(next_id);

                        (next_id, true)
                    },
                }
            },
            Self::Antichain {
                seen,
                spare_entries,
            } => {
                let shape = ConfigShape::from(&config);
                let left_nz = parity_lower_bound(
                    config.tape.left_nz,
                    config.tape.left_parity,
                );
                let right_nz = parity_lower_bound(
                    config.tape.right_nz,
                    config.tape.right_parity,
                );
                let entries = match seen.entry(shape) {
                    Entry::Occupied(entry) => entry.into_mut(),
                    Entry::Vacant(entry) => {
                        let mut entries =
                            spare_entries.pop().unwrap_or_default();
                        entries.clear();
                        entry.insert(entries)
                    },
                };

                // Smaller lower bounds are more general: they represent every
                // tape represented by a configuration with larger bounds.  If
                // an active representative already subsumes this pair,
                // redirect to it.
                if let Some(entry) = entries.iter().find(|entry| {
                    entry.left_nz <= left_nz
                        && entry.right_nz <= right_nz
                }) {
                    return (entry.id, false);
                }

                // This pair is genuinely broader than every existing
                // representative.  Retire all narrower pairs it subsumes,
                // leaving only a componentwise antichain for this structural
                // configuration shape.
                entries.retain(|entry| {
                    let dominated = left_nz <= entry.left_nz
                        && right_nz <= entry.right_nz;

                    if dominated {
                        debug_assert!(active[entry.id]);
                        active[entry.id] = false;
                        *active_count -= 1;
                    }

                    !dominated
                });

                let next_id = active.len();
                debug_assert_eq!(by_id.len(), next_id);
                by_id.push(config);
                active.push(true);
                *active_count += 1;
                entries.push(AntichainEntry {
                    left_nz,
                    right_nz,
                    id: next_id,
                });

                (next_id, true)
            },
        }
    }

    fn clear(&mut self) {
        match self {
            Self::Exact(seen) => seen.clear(),
            Self::Antichain {
                seen,
                spare_entries,
            } => {
                for (_, mut entries) in seen.drain() {
                    entries.clear();
                    spare_entries.push(entries);
                }
            },
        }
    }

    const fn at_capacity(
        &self,
        by_id_len: usize,
        active_count: usize,
    ) -> bool {
        let count = match self {
            Self::Exact(_) => by_id_len,
            Self::Antichain { .. } => active_count,
        };

        MAX_DEPTH < count
    }
}

/**************************************/

#[derive(Clone, Copy)]
struct PushTransition {
    last: Color,
    color: Color,
    next: Span,
}

#[derive(Clone, Copy)]
struct PullTransition {
    last: Color,
    next: Span,
    pulled: Color,
}

struct SpanPool {
    spans: Vec<Colors>,
    span_count: usize,
    index: Dict<Colors, SpanId>,
    blank: Option<Vec<bool>>,
    base_blank: Option<Vec<Option<bool>>>,

    push_cache: Vec<Vec<PushTransition>>,
    pull_cache: Vec<Vec<PullTransition>>,
    spare_colors: Vec<Colors>,
}

impl SpanPool {
    fn new(goal: Goal) -> Self {
        Self {
            spans: vec![],
            span_count: 0,
            index: Dict::new(),
            blank: match goal {
                Spinout => Some(vec![]),
                Halt | Blank => None,
            },
            base_blank: match goal {
                Blank => Some(vec![]),
                Halt | Spinout => None,
            },
            push_cache: Vec::new(),
            pull_cache: Vec::new(),
            spare_colors: Vec::new(),
        }
    }

    fn reset(&mut self) {
        let old_span_count = self.span_count;

        // Only IDs used by the immediately previous run can contain live
        // transitions. Entries above that logical prefix were already
        // cleared when they last belonged to a run.
        for transitions in &mut self.push_cache[..old_span_count] {
            transitions.clear();
        }
        for transitions in &mut self.pull_cache[..old_span_count] {
            transitions.clear();
        }

        self.span_count = 0;

        // The index owns one color vector per interned span.  Retain those
        // allocations for candidate spans in later CPS runs instead of
        // dropping them when the logical span table is reset.
        for (mut colors, _) in self.index.drain() {
            colors.clear();
            self.spare_colors.push(colors);
        }

        if let Some(blank) = &mut self.blank {
            blank.clear();
        }
        if let Some(base_blank) = &mut self.base_blank {
            base_blank.clear();
        }
    }

    fn take_colors(&mut self, len: usize) -> Colors {
        let mut colors = self
            .spare_colors
            .pop()
            .unwrap_or_else(|| Vec::with_capacity(len));
        colors.clear();

        if colors.capacity() < len {
            colors.reserve(len - colors.capacity());
        }

        colors
    }

    fn recycle_colors(&mut self, mut colors: Colors) {
        colors.clear();
        self.spare_colors.push(colors);
    }

    fn intern(&mut self, colors: Colors) -> SpanId {
        if let Some(&id) = self.index.get(&colors) {
            self.recycle_colors(colors);
            return id;
        }

        let id = self.span_count;
        self.span_count += 1;

        if id == self.spans.len() {
            let mut stored = self.take_colors(colors.len());
            stored.extend_from_slice(&colors);
            self.spans.push(stored);
            self.push_cache.push(Vec::new());
            self.pull_cache.push(Vec::new());
        } else {
            let stored = &mut self.spans[id];
            stored.clear();
            stored.extend_from_slice(&colors);
        }
        self.index.insert(colors, id);

        if let Some(blank) = &mut self.blank {
            blank.push(self.spans[id].iter().all(|&color| color == 0));
        }
        if let Some(base_blank) = &mut self.base_blank {
            base_blank.push(None);
        }

        id
    }

    fn colors(&self, id: SpanId) -> &Colors {
        debug_assert!(id < self.span_count);
        &self.spans[id]
    }

    fn blank_span(&self, id: SpanId) -> bool {
        self.blank
            .as_ref()
            .expect("canonical blank cache is only used for Spinout")
            [id]
    }

    fn base_blank_span(
        &mut self,
        prog: &impl GetInstr,
        id: SpanId,
    ) -> bool {
        if let Some(blank) = self
            .base_blank
            .as_ref()
            .expect("base blank cache is only used for Blank")[id]
        {
            return blank;
        }

        let blank =
            self.spans[id].iter().all(|&color| prog.is_blank(color));
        self.base_blank
            .as_mut()
            .expect("base blank cache is only used for Blank")[id] =
            Some(blank);
        blank
    }
}

/**************************************/

struct Configs {
    span_pool: SpanPool,

    lspans: Spans,
    rspans: Spans,

    interner: ConfigInterner,
    by_id: Vec<Config>,
    active: Vec<bool>,
    active_count: usize,
    todo: Vec<ConfigId>,
    todo_head: usize,

    l_watch: Watch,
    r_watch: Watch,
}

impl Configs {
    fn new(goal: Goal) -> Self {
        Self {
            span_pool: SpanPool::new(goal),
            lspans: Spans::new(goal),
            rspans: Spans::new(goal),
            interner: if matches!(goal, Halt) {
                ConfigInterner::Exact(Dict::new())
            } else {
                ConfigInterner::Antichain {
                    seen: Dict::new(),
                    spare_entries: Vec::new(),
                }
            },
            by_id: Vec::new(),
            active: Vec::new(),
            active_count: 0,
            todo: Vec::new(),
            todo_head: 0,
            l_watch: Vec::new(),
            r_watch: Vec::new(),
        }
    }

    fn reset(&mut self, rad: Radius) {
        let old_span_count = self.span_pool.span_count;

        // Span IDs are reassigned from zero. Only the prefix used by the
        // previous run can contain live continuation or watcher entries.
        self.lspans.clear(old_span_count);
        self.rspans.clear(old_span_count);

        let l_watch_count = old_span_count.min(self.l_watch.len());
        for waiting in &mut self.l_watch[..l_watch_count] {
            waiting.clear();
        }

        let r_watch_count = old_span_count.min(self.r_watch.len());
        for waiting in &mut self.r_watch[..r_watch_count] {
            waiting.clear();
        }

        self.span_pool.reset();
        self.interner.clear();
        self.by_id.clear();
        self.active.clear();
        self.active_count = 0;
        self.todo.clear();
        self.todo_head = 0;

        let init = Config::init(rad, &mut self.span_pool);

        self.lspans.add_span(
            &init.tape.lspan,
            init.tape.left_nz,
            init.tape.left_parity,
        );
        self.rspans.add_span(
            &init.tape.rspan,
            init.tape.right_nz,
            init.tape.right_parity,
        );

        let (init_id, is_new) = self.intern_config(init);
        assert!(is_new);
        debug_assert_eq!(init_id, 0);
        self.todo.push(init_id);
    }

    fn intern_config(&mut self, config: Config) -> (ConfigId, bool) {
        self.interner.intern(
            config,
            &mut self.by_id,
            &mut self.active,
            &mut self.active_count,
        )
    }

    fn is_active(&self, id: ConfigId) -> bool {
        match &self.interner {
            ConfigInterner::Exact(_) => true,
            ConfigInterner::Antichain { .. } => {
                self.active.get(id).copied().unwrap_or(false)
            },
        }
    }

    fn add_span(
        &mut self,
        shift: Shift,
        span: &Span,
        tail_nz: usize,
        tail_parity: bool,
    ) {
        let exact = matches!(&self.interner, ConfigInterner::Exact(_));
        let (spans, watch) = if shift {
            (&mut self.lspans, &mut self.l_watch)
        } else {
            (&mut self.rspans, &mut self.r_watch)
        };

        if spans.add_span(span, tail_nz, tail_parity)
            && let Some(waiting) = watch.get_mut(span.span)
        {
            if exact {
                self.todo.append(waiting);
            } else {
                let active = &self.active;
                self.todo.extend(
                    waiting
                        .drain(..)
                        .filter(|id| active.get(*id) == Some(&true)),
                );
            }
        }
    }
}

/**************************************/

impl Spans {
    const fn new(goal: Goal) -> Self {
        match goal {
            Halt => Self::Halt(Vec::new()),
            Blank | Spinout => Self::Rich(Vec::new()),
        }
    }

    fn clear(&mut self, span_count: usize) {
        match self {
            Self::Halt(spans) => {
                for colors in spans.iter_mut().take(span_count) {
                    colors.clear();
                }
            },
            Self::Rich(spans) => {
                for continuations in spans.iter_mut().take(span_count) {
                    continuations.clear();
                }
            },
        }
    }

    fn add_span(
        &mut self,
        span: &Span,
        tail_nz: usize,
        tail_parity: bool,
    ) -> bool {
        match self {
            Self::Halt(spans) => {
                debug_assert_eq!(tail_nz, 0);
                debug_assert!(!tail_parity);

                if spans.len() <= span.span {
                    spans.resize_with(span.span + 1, Vec::new);
                }

                let colors = &mut spans[span.span];
                match colors.binary_search(&span.last) {
                    Ok(_) => false,
                    Err(pos) => {
                        colors.insert(pos, span.last);
                        true
                    },
                }
            },
            Self::Rich(spans) => {
                let tail_nz = parity_lower_bound(tail_nz, tail_parity);
                let continuation = Continuation {
                    color: span.last,
                    tail_nz,
                    tail_parity,
                };

                if spans.len() <= span.span {
                    spans.resize_with(span.span + 1, Vec::new);
                }

                let continuations = &mut spans[span.span];
                match continuations.binary_search_by_key(
                    &(span.last, tail_parity),
                    |cnt| (cnt.color, cnt.tail_parity),
                ) {
                    Ok(pos) => {
                        // Counts are lower bounds.  For the same continuation
                        // color and exact tail parity, a smaller bound subsumes
                        // every larger one.
                        if tail_nz < continuations[pos].tail_nz {
                            continuations[pos].tail_nz = tail_nz;
                            return true;
                        }
                        false
                    },
                    Err(pos) => {
                        continuations.insert(pos, continuation);
                        true
                    },
                }
            },
        }
    }

    fn get_halt_colors(&self, span: &Span) -> &[Color] {
        let Self::Halt(spans) = self else {
            unreachable!(
                "color-only continuations are only used for Halt"
            );
        };
        let colors = &spans[span.span];
        debug_assert!(!colors.is_empty());
        colors
    }

    fn get_continuations(&self, span: &Span) -> &Continuations {
        let Self::Rich(spans) = self else {
            unreachable!("full continuations are not used for Halt");
        };
        let continuations = &spans[span.span];
        debug_assert!(!continuations.is_empty());
        continuations
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
    left_nz: usize,
    right_nz: usize,
    left_parity: bool,
    right_parity: bool,
}

impl Tape {
    fn init(rad: Radius, pool: &mut SpanPool) -> Self {
        Self {
            scan: 0,
            lspan: Span::init(rad, pool),
            rspan: Span::init(rad, pool),
            left_nz: 0,
            right_nz: 0,
            left_parity: false,
            right_parity: false,
        }
    }

    const fn from_spans(
        scan: Color,
        push: Span,
        pull: Span,
        shift: Shift,
        left_nz: usize,
        right_nz: usize,
        left_parity: bool,
        right_parity: bool,
    ) -> Self {
        let (lspan, rspan) =
            if shift { (push, pull) } else { (pull, push) };

        Self {
            scan,
            lspan,
            rspan,
            left_nz,
            right_nz,
            left_parity,
            right_parity,
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
            "L(pat={}, last={}, nz={}, odd={}) [{}] R(pat={}, last={}, nz={}, odd={})",
            self.lspan.span,
            self.lspan.last,
            self.left_nz,
            self.left_parity,
            self.scan,
            self.rspan.span,
            self.rspan.last,
            self.right_nz,
            self.right_parity
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

        let mut colors = pool.take_colors(rad - 1);
        colors.resize(rad - 1, 0);

        Self {
            span: pool.intern(colors),
            last: 0,
        }
    }

    fn push(&mut self, color: Color, pool: &mut SpanPool) -> Color {
        let span_id = self.span;
        let last = self.last;

        if let Some(next) = pool.push_cache[span_id]
            .iter()
            .find(|transition| {
                transition.last == last && transition.color == color
            })
            .map(|transition| transition.next)
        {
            *self = next;
            return last;
        }

        let span_len = pool.colors(span_id).len();
        let mut v = pool.take_colors(span_len);
        let new_last = {
            let colors = pool.colors(span_id);

            #[expect(clippy::shadow_unrelated)]
            if let Some((&last, prefix)) = colors.split_last() {
                v.push(color);
                v.extend_from_slice(prefix);
                last
            } else {
                color
            }
        };

        let next = Self {
            span: pool.intern(v),
            last: new_last,
        };

        pool.push_cache[span_id].push(PushTransition {
            last,
            color,
            next,
        });
        *self = next;
        last
    }

    fn pull(&mut self, pool: &mut SpanPool) -> Color {
        let span_id = self.span;
        let last = self.last;

        if let Some((next, pulled)) = pool.pull_cache[span_id]
            .iter()
            .find(|transition| transition.last == last)
            .map(|transition| (transition.next, transition.pulled))
        {
            *self = next;
            return pulled;
        }

        let span_len = pool.colors(span_id).len();
        let mut v = pool.take_colors(span_len);
        let pulled = {
            let colors = pool.colors(span_id);

            if let Some((&first, rest)) = colors.split_first() {
                v.extend_from_slice(rest);
                v.push(last);
                first
            } else {
                last
            }
        };

        let next = Self {
            span: pool.intern(v),
            last,
        };

        pool.pull_cache[span_id].push(PullTransition {
            last,
            next,
            pulled,
        });
        *self = next;
        pulled
    }

    fn blank_span(&self, pool: &SpanPool) -> bool {
        pool.blank_span(self.span)
    }

    fn base_blank_span(
        &self,
        prog: &impl GetInstr,
        pool: &mut SpanPool,
    ) -> bool {
        pool.base_blank_span(prog, self.span)
    }

    fn base_all_blank(
        &self,
        prog: &impl GetInstr,
        pool: &mut SpanPool,
    ) -> bool {
        prog.is_blank(self.last) && self.base_blank_span(prog, pool)
    }
}

#[test]
fn test_span() {
    let mut pool = SpanPool::new(Halt);
    let mut span = Span::init(3, &mut pool);

    assert_eq!(pool.colors(span.span).as_slice(), &[0, 0]);
    assert_eq!(span.last, 0);

    span.push(1, &mut pool);
    span.push(1, &mut pool);
    span.push(0, &mut pool);

    assert_eq!(pool.colors(span.span).as_slice(), &[0, 1]);
    assert_eq!(span.last, 1);
}
