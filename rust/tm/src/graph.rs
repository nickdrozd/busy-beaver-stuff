use std::collections::{
    BTreeMap as Dict, BTreeSet as Set, HashMap, VecDeque,
};

use crate::{Color, Prog, State};

/**************************************/

impl<const states: usize, const colors: usize> Prog<states, colors> {
    pub fn is_connected(&self) -> bool {
        if self.instrs().all(|&(_, _, state)| state != 0) {
            return false;
        }

        let exitpoints = self.get_exitpoints();

        if exitpoints.len() < states {
            return false;
        }

        #[expect(clippy::cast_possible_truncation)]
        let last_state = (states as State) - 1;

        let last_exits = &exitpoints[&last_state];

        if last_exits.contains(&0) {
            return true;
        }

        let mut reached: Set<State> = Set::from([last_state]);

        let mut todo: Vec<State> = last_exits.clone();

        for _ in 0..states {
            let Some(state) = todo.pop() else {
                break;
            };

            if state == 0 {
                return true;
            }

            if reached.contains(&state) {
                continue;
            }

            reached.insert(state);

            for &exit in &exitpoints[&state] {
                if !reached.contains(&exit) && !todo.contains(&exit) {
                    todo.push(exit);
                }
            }
        }

        false
    }

    #[expect(clippy::cast_possible_truncation)]
    pub fn is_strict_cycle(&self) -> bool {
        let mut next = Vec::with_capacity(states);

        for st in 0..states as State {
            let mut dst: Option<State> = None;

            for co in 0..(colors as Color) {
                let Some(&(_, _, tr)) = self.get(&(st, co)) else {
                    return false;
                };

                match dst {
                    None => dst = Some(tr),
                    Some(prev) if prev == tr => {},
                    Some(_) => return false,
                }
            }

            let Some(tr) = dst else {
                return false;
            };

            if tr == st {
                return false;
            }

            next.push(tr);
        }

        // Must be a single Hamiltonian cycle (covers all states exactly once).
        let mut seen = vec![false; states];
        let mut cur: State = 0;

        for _ in 0..states {
            let idx = cur as usize;
            if seen[idx] {
                return false;
            }
            seen[idx] = true;
            cur = next[idx];
        }

        if cur != 0 {
            return false;
        }

        if seen.iter().any(|&b| !b) {
            return false;
        }

        // And the cycle order must be strictly increasing or strictly
        // decreasing (with wraparound).
        let forward = (0..states).all(|i| {
            let want =
                if i + 1 == states { 0 } else { (i + 1) as State };
            next[i] == want
        });

        let backward = (0..states).all(|i| {
            let want = if i == 0 {
                (states - 1) as State
            } else {
                (i - 1) as State
            };
            next[i] == want
        });

        forward || backward
    }

    /// Sound (but incomplete) static proof that the program cannot quasihalt
    /// (BusyBeaver convention: start state = 0 on an all-0 blank tape).
    ///
    /// Combines a fast control/SCC proof with a stronger (still static)
    /// abstract-configuration graph refinement. If the refinement exceeds
    /// resource limits, this returns `false` ("can't prove").
    pub fn graph_cant_quasihalt(&self) -> bool {
        if self.graph_cant_quasihalt_fast() {
            return true;
        }
        self.graph_cant_quasihalt_abs()
    }

    /// Fast sufficient condition (pure control/SCC + one-direction `read=0` cycle lemma).
    fn graph_cant_quasihalt_fast(&self) -> bool {
        let start = 0;

        if start >= states {
            return false;
        }

        // Control adjacency: union of next-states over all read symbols.
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); states];
        for ((src, _read), &(_write, _shift, dst)) in self.iter() {
            let u = src as usize;
            let v = dst as usize;
            if u < states && v < states {
                adj[u].push(v);
            }
        }
        for u in 0..states {
            adj[u].sort_unstable();
            adj[u].dedup();
        }

        let reachable_all = reach_from(states, &adj, start);
        if reachable_all.iter().any(|&b| !b) {
            return false;
        }

        // Shift-aware refinement:
        // For each target state `t`, look for a *realizable* infinite trap that avoids `t`.
        //
        // Any infinite execution that avoids `t` forever must eventually remain inside some SCC
        // not containing `t`. We conservatively treat SCCs as traps, except that we can *rule out*
        // certain SCCs that are strictly one-directional (all internal moves are L or all are R)
        // **and** do not contain a cycle induced by the `read=0` transitions in that direction.
        //
        // Why this extra rule is sound for BB-from-blank:
        // - In a one-direction SCC, the head position is strictly monotone, so no tape cell is revisited.
        // - Starting from a blank tape, every newly visited cell is `0`, so from that point on only
        //   `read=0` transitions can occur. Thus an infinite run inside such an SCC requires a
        //   `read=0` cycle with the same direction.
        for t in 0..states {
            // Active nodes: reachable from start and not equal to t.
            let mut active = vec![false; states];
            for u in 0..states {
                active[u] = reachable_all[u] && u != t;
            }

            // Compute SCCs in the induced subgraph.
            let sccs = sccs_masked(states, &adj, &active);

            for comp in sccs {
                if !scc_has_cycle(&comp, &adj) {
                    continue;
                }

                // Determine whether this SCC contains both L and R internal transitions.
                let mut has_l = false;
                let mut has_r = false;

                // Also record which single direction it has (if any), so we can do the 0-cycle check.
                for ((src, _read), &(_write, sh, dst)) in self.iter() {
                    let u = src as usize;
                    let v = dst as usize;
                    if u >= states || v >= states {
                        continue;
                    }
                    if !active[u] || !active[v] {
                        continue;
                    }
                    if !comp_contains(&comp, u)
                        || !comp_contains(&comp, v)
                    {
                        continue;
                    }

                    if sh {
                        has_r = true;
                    } else {
                        has_l = true;
                    }

                    if has_l && has_r {
                        // Bidirectional SCC: we cannot rule out an infinite bounded "bouncing" run.
                        return false;
                    }
                }

                // If we have a (cyclic) SCC but saw no internal transitions from self.iter(),
                // it's because those transitions are absent; conservatively treat as trap.
                if !has_l && !has_r {
                    return false;
                }

                // One-direction SCC: rule it out unless it contains a feasible `read=0` cycle
                // in that same direction.
                let dir_is_r = has_r; // if not R, must be L
                if has_zero_dir_cycle_in_comp::<states, colors>(
                    self, &comp, dir_is_r,
                ) {
                    return false;
                }

                // Otherwise, this SCC cannot host an infinite avoidance run from blank tape.
            }
        }

        true
    }

    /// Stronger sound proof using a finite abstract-configuration graph.
    ///
    /// This is purely static/symbolic: it explores a finite over-approximation
    /// of reachable local tape windows and then checks for the existence of an
    /// abstract cycle that avoids each control state.
    fn graph_cant_quasihalt_abs(&self) -> bool {
        // Modest defaults: keep the abstraction small and safe.
        // Larger values are more powerful but can explode.
        const MAX_TAPE: usize = 15;
        const MAX_NODES: usize = 1_000;

        // Encode wildcard as `colors`.
        #[expect(clippy::cast_possible_truncation)]
        let wild: u8 = colors as u8;
        #[expect(clippy::cast_possible_truncation)]
        let nstates: u8 = states as u8;
        if states == 0 || colors == 0 {
            return false;
        }

        let (nodes, adj) = build_abs_graph::<states, colors>(
            self, MAX_TAPE, MAX_NODES, wild,
        );

        // If we hit the cap, we conservatively give up (no false proofs).
        if nodes.is_empty() || nodes.len() >= MAX_NODES {
            return false;
        }

        // All abstract nodes are reachable in the full graph by construction.
        // For each control state t, we must consider *any* cycle that avoids t,
        // even if reaching it required visiting t earlier (eventual avoidance).
        // Therefore we MUST NOT recompute reachability on the filtered graph.
        for t in 0..nstates {
            let mut active = vec![true; nodes.len()];
            for (i, cfg) in nodes.iter().enumerate() {
                if cfg.state == t {
                    active[i] = false;
                }
            }
            if dyn_cycle_exists(&adj, &active) {
                return false;
            }
        }

        true
    }

    /// Sound (but incomplete) static proof that the program cannot HALT
    /// (i.e. cannot ever execute an undefined instruction slot), under the
    /// BusyBeaver convention: start state = 0 on an all-0 blank tape.
    ///
    /// This is a *sufficient* condition only:
    /// - `true`  => proved it cannot halt
    /// - `false` => can't prove
    pub fn graph_cant_halt(&self) -> bool {
        if states == 0 {
            return false;
        }

        // If there are no halting slots at all (within the reached bounds),
        // then the program is total and cannot halt.
        let halts = self.halt_slots();
        if halts.is_empty() {
            return true;
        }

        // Build control adjacency: union of next-states over all read symbols.
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); states];
        for ((src, _read), &(_write, _shift, dst)) in self.iter() {
            let u = src as usize;
            let v = dst as usize;
            if u < states && v < states {
                adj[u].push(v);
            }
        }
        for u in 0..states {
            adj[u].sort_unstable();
            adj[u].dedup();
        }

        let reachable = reach_from(states, &adj, 0);

        // Sufficient condition: every halting slot belongs to a control state
        // that is unreachable from the start (even in the over-approx control graph).
        // Then we can never arrive at that missing slot.
        halts.iter().all(|&(st, _co)| {
            let u = st as usize;
            u >= states || !reachable[u]
        })
    }

    /// Sound (but incomplete) static proof that the program cannot reach
    /// the **fully blank tape** condition (all cells 0) at any time *after* the
    /// initial start configuration.
    ///
    /// This matches the usual "blank tape" early-termination rule:
    /// stop if the machine ever returns to an all-0 tape.
    ///
    /// - `true`  => proved it cannot blank (after time 0)
    /// - `false` => can't prove
    pub fn graph_cant_blank(&self) -> bool {
        if self.graph_cant_blank_fast() {
            return true;
        }
        self.graph_cant_blank_abs()
    }

    // Very cheap sufficient condition:
    // If the program has **no erase moves** (no read!=0 -> write=0), then once it ever
    // writes a nonzero symbol, the tape can never be fully blank again.
    // So from the BB start, if (0,0) exists and writes nonzero, we can conclude.
    fn graph_cant_blank_fast(&self) -> bool {
        if states == 0 {
            return false;
        }

        if !self.erase_slots().is_empty() {
            return false;
        }

        let Some(&(pr, _sh, _tr)) = self.get(&(0, 0)) else {
            // Immediate halt; not a "blank return".
            return false;
        };

        pr != 0
    }

    // Stronger (still sound) check using the bounded abstract configuration graph.
    //
    // Soundness rule:
    // If a truly blank tape is reachable, then within *any* finite window the
    // tape contents are all `0`. Our abstraction may contain `wild` (unknown)
    // cells that could also be `0`, so we must treat "all cells are {0,wild}" as
    // compatible with a concrete blank tape.
    fn graph_cant_blank_abs(&self) -> bool {
        const MAX_TAPE: usize = 15;
        const MAX_NODES: usize = 1_000;

        if states == 0 || colors == 0 {
            return false;
        }

        // Encode wildcard as `colors`.
        #[expect(clippy::cast_possible_truncation)]
        let wild: u8 = colors as u8;

        let (nodes, _adj) = build_abs_graph::<states, colors>(
            self, MAX_TAPE, MAX_NODES, wild,
        );

        // If we hit the cap, we conservatively give up.
        if nodes.is_empty() || nodes.len() >= MAX_NODES {
            return false;
        }

        // If any reachable abstract node after the start has no *known* nonzero
        // symbol in the tracked window, then a concrete blank tape is still
        // compatible with that abstract state.
        for cfg in nodes.iter().skip(1) {
            let blank_compatible =
                cfg.tape.iter().all(|&x| x == 0 || x == wild);
            if blank_compatible {
                return false;
            }
        }

        true
    }

    /// Sound (but incomplete) static proof that the program cannot "spin out".
    ///
    /// The `prog.rs` helper `zr_shifts()` identifies *spinout triggers*:
    /// states `s` that, on reading `0`, transition back to themselves
    /// (`(s,0) -> (.., shift, s)`). Once such a transition is taken while
    /// reading `0`, the machine can keep moving in that direction forever
    /// on fresh blank cells.
    ///
    /// We use a simple sufficient condition: if no such trigger state is
    /// reachable from the start (even in the over-approx control graph),
    /// then spinout is impossible.
    pub fn graph_cant_spin_out(&self) -> bool {
        if states == 0 {
            return false;
        }

        let spin_triggers = self.zr_shifts();
        if spin_triggers.is_empty() {
            return true;
        }

        // Control adjacency: union of next-states over all read symbols.
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); states];
        for ((src, _read), &(_write, _shift, dst)) in self.iter() {
            let u = src as usize;
            let v = dst as usize;
            if u < states && v < states {
                adj[u].push(v);
            }
        }
        for u in 0..states {
            adj[u].sort_unstable();
            adj[u].dedup();
        }
        let reachable = reach_from(states, &adj, 0);

        // If any spin-trigger state is reachable, we can't prove it can't spin out.
        for (st, _sh) in spin_triggers {
            let u = st as usize;
            if u < states && reachable[u] {
                return false;
            }
        }

        true
    }
}

fn reach_from(
    states: usize,
    adj: &[Vec<usize>],
    start: usize,
) -> Vec<bool> {
    let mut seen = vec![false; states];
    if start >= states {
        return seen;
    }

    let mut stack = vec![start];
    while let Some(u) = stack.pop() {
        if seen[u] {
            continue;
        }
        seen[u] = true;
        for &v in &adj[u] {
            if v >= states || seen[v] {
                continue;
            }
            stack.push(v);
        }
    }

    seen
}

/**************************************/
// Abstract configuration graph (sound static over-approx)

/// Abstract tape symbol: `0..colors-1` are concrete, and `wild` means "unknown".
///
/// `left_unknown/right_unknown` indicate whether cells beyond the stored window
/// may have been modified (unknown) or are still guaranteed blank 0.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct AbsCfg {
    state: u8,
    head: u8,
    tape: Vec<u8>,
    left_unknown: bool,
    right_unknown: bool,
}

impl AbsCfg {
    fn new_blank(start_state: u8) -> Self {
        Self {
            state: start_state,
            head: 0,
            tape: vec![0],
            left_unknown: false,
            right_unknown: false,
        }
    }
}

#[expect(clippy::cast_possible_truncation)]
fn normalize_abs(cfg: &mut AbsCfg, max_tape: usize) {
    if cfg.tape.is_empty() {
        cfg.tape.push(0);
        cfg.head = 0;
        return;
    }
    if cfg.head as usize >= cfg.tape.len() {
        cfg.head = (cfg.tape.len() - 1) as u8;
    }

    if cfg.tape.len() <= max_tape {
        return;
    }

    let len = cfg.tape.len();
    let head = cfg.head as usize;
    let half = max_tape / 2;
    let mut start = head.saturating_sub(half);
    if start + max_tape > len {
        start = len - max_tape;
    }
    let end = start + max_tape;

    if start > 0 {
        cfg.left_unknown = true;
    }
    if end < len {
        cfg.right_unknown = true;
    }

    cfg.tape = cfg.tape[start..end].to_vec();
    cfg.head = (head - start) as u8;
}

#[expect(clippy::cast_possible_truncation)]
fn step_abs<const states: usize, const colors: usize>(
    prog: &Prog<states, colors>,
    cfg: &AbsCfg,
    wild: u8,
) -> Vec<AbsCfg> {
    let head = cfg.head as usize;
    if head >= cfg.tape.len() {
        return vec![];
    }

    let cur = cfg.tape[head];
    let mut reads: Vec<u8> = Vec::new();
    if cur == wild {
        for r in 0..(colors as u8) {
            reads.push(r);
        }
    } else {
        reads.push(cur);
    }

    let mut out: Vec<AbsCfg> = Vec::new();
    for read in reads {
        let st = cfg.state;
        let co = read;
        let Some(&(write, sh, dst)) = prog.get(&(st, co)) else {
            continue;
        };

        let mut nxt = cfg.clone();
        nxt.state = dst;
        nxt.tape[head] = write;

        // Move head and extend window if necessary.
        if sh {
            // Right
            let new_head = head + 1;
            if new_head >= nxt.tape.len() {
                let new_cell = if nxt.right_unknown { wild } else { 0 };
                nxt.tape.push(new_cell);
            }
            nxt.head = new_head as u8;
        } else {
            // Left
            if head == 0 {
                let new_cell = if nxt.left_unknown { wild } else { 0 };
                nxt.tape.insert(0, new_cell);
                nxt.head = 0;
            } else {
                nxt.head = (head - 1) as u8;
            }
        }

        out.push(nxt);
    }

    out
}

fn build_abs_graph<const states: usize, const colors: usize>(
    prog: &Prog<states, colors>,
    max_tape: usize,
    max_nodes: usize,
    wild: u8,
) -> (Vec<AbsCfg>, Vec<Vec<usize>>) {
    let mut nodes: Vec<AbsCfg> = Vec::new();
    let mut adj: Vec<Vec<usize>> = Vec::new();
    let mut map: HashMap<AbsCfg, usize> = HashMap::new();
    let mut q: VecDeque<usize> = VecDeque::new();

    let mut start = AbsCfg::new_blank(0);
    normalize_abs(&mut start, max_tape);
    nodes.push(start.clone());
    adj.push(Vec::new());
    map.insert(start, 0);
    q.push_back(0);

    while let Some(u) = q.pop_front() {
        if nodes.len() >= max_nodes {
            break;
        }

        let succs = step_abs::<states, colors>(prog, &nodes[u], wild);
        for mut vcfg in succs {
            normalize_abs(&mut vcfg, max_tape);
            let vid = if let Some(&id) = map.get(&vcfg) {
                id
            } else {
                let id = nodes.len();
                nodes.push(vcfg.clone());
                adj.push(Vec::new());
                map.insert(vcfg, id);
                q.push_back(id);
                id
            };
            adj[u].push(vid);
        }
        adj[u].sort_unstable();
        adj[u].dedup();
    }

    (nodes, adj)
}

/// Iterative 3-color DFS cycle check on the induced subgraph of `active` nodes.
///
/// Returns true iff there is a directed cycle using only active nodes.
fn dyn_cycle_exists(adj: &[Vec<usize>], active: &[bool]) -> bool {
    let n = adj.len();
    let mut color = vec![0; n]; // 0=unseen,1=visiting,2=done

    for start in 0..n {
        if !active[start] || color[start] != 0 {
            continue;
        }

        // stack of (node, next_edge_index)
        let mut stack: Vec<(usize, usize)> = Vec::new();
        stack.push((start, 0));
        color[start] = 1;

        while let Some((u, ei)) = stack.pop() {
            if ei >= adj[u].len() {
                color[u] = 2;
                continue;
            }

            // resume node u at next edge index
            stack.push((u, ei + 1));
            let v = adj[u][ei];
            if v >= n || !active[v] {
                continue;
            }
            match color[v] {
                0 => {
                    color[v] = 1;
                    stack.push((v, 0));
                },
                1 => {
                    // back-edge => cycle
                    return true;
                },
                _ => {},
            }
        }
    }

    false
}

/**************************************/

fn comp_contains(comp: &[usize], x: usize) -> bool {
    // SCC sizes here are tiny (BB state counts), so linear scan is fine.
    comp.contains(&x)
}

/// Kosaraju SCC decomposition on a graph given as adjacency lists,
/// restricted to `active` nodes.
#[expect(clippy::items_after_statements)]
fn sccs_masked(
    states: usize,
    adj: &[Vec<usize>],
    active: &[bool],
) -> Vec<Vec<usize>> {
    let mut rev: Vec<Vec<usize>> = vec![Vec::new(); states];
    for u in 0..states {
        if !active[u] {
            continue;
        }
        for &v in &adj[u] {
            if v < states && active[v] {
                rev[v].push(u);
            }
        }
    }

    let mut seen = vec![false; states];
    let mut order = Vec::new();

    fn dfs1(
        u: usize,
        adj: &[Vec<usize>],
        active: &[bool],
        seen: &mut [bool],
        order: &mut Vec<usize>,
    ) {
        seen[u] = true;
        for &v in &adj[u] {
            if v < active.len() && active[v] && !seen[v] {
                dfs1(v, adj, active, seen, order);
            }
        }
        order.push(u);
    }

    for u in 0..states {
        if active[u] && !seen[u] {
            dfs1(u, adj, active, &mut seen, &mut order);
        }
    }

    let mut comps: Vec<Vec<usize>> = Vec::new();
    let mut comp_id = vec![usize::MAX; states];

    fn dfs2(
        u: usize,
        rev: &[Vec<usize>],
        active: &[bool],
        cid: usize,
        comp_id: &mut [usize],
        comp: &mut Vec<usize>,
    ) {
        comp_id[u] = cid;
        comp.push(u);
        for &v in &rev[u] {
            if active[v] && comp_id[v] == usize::MAX {
                dfs2(v, rev, active, cid, comp_id, comp);
            }
        }
    }

    while let Some(u) = order.pop() {
        if !active[u] || comp_id[u] != usize::MAX {
            continue;
        }
        let cid = comps.len();
        let mut comp = Vec::new();
        dfs2(u, &rev, active, cid, &mut comp_id, &mut comp);
        comps.push(comp);
    }

    comps
}

/// Whether an SCC contains a directed cycle in the induced subgraph.
fn scc_has_cycle(comp: &[usize], adj: &[Vec<usize>]) -> bool {
    if comp.len() >= 2 {
        return true;
    }
    let u = comp[0];
    adj[u].contains(&u)
}

/// In a strictly one-direction SCC, an infinite run from a blank tape
/// is possible only if there is a `read=0` cycle whose transitions
/// all have that same direction and stay within the SCC.
fn has_zero_dir_cycle_in_comp<
    const states: usize,
    const colors: usize,
>(
    prog: &Prog<states, colors>,
    comp: &[usize],
    dir_is_r: bool,
) -> bool {
    let mut in_comp = vec![false; states];
    for &u in comp {
        if u < states {
            in_comp[u] = true;
        }
    }

    // next[u] = dst on read=0 if it stays in comp and moves in `dir_is_r`.
    let mut next: Vec<Option<usize>> = vec![None; states];
    for &u in comp {
        #[expect(clippy::cast_possible_truncation)]
        let su = u as State;
        if let Some(&(_write, sh, dst)) = prog.get(&(su, 0))
            && sh == dir_is_r
        {
            let v = dst as usize;
            if v < states && in_comp[v] {
                next[u] = Some(v);
            }
        }
    }

    // Detect a directed cycle in this partial functional graph.
    // 0 = unvisited, 1 = visiting, 2 = done
    let mut mark = vec![0; states];

    for &start in comp {
        if mark[start] != 0 {
            continue;
        }
        let mut u = start;
        let mut stack: Vec<usize> = Vec::new();

        while in_comp[u] {
            if mark[u] == 1 {
                // back-edge in functional walk => cycle
                return true;
            }
            if mark[u] == 2 {
                break;
            }

            mark[u] = 1;
            stack.push(u);

            let Some(v) = next[u] else {
                break;
            };
            u = v;
        }

        for x in stack {
            mark[x] = 2;
        }
    }

    false
}

/**************************************/

#[cfg(test)]
macro_rules! assert_connected {
    ($prog:literal, ($s:literal, $c:literal), $conn:literal) => {{
        let result = Prog::<$s, $c>::from($prog).is_connected();
        assert!(if $conn { result } else { !result });
    }};
}

#[test]
fn test_connected() {
    assert_connected!("1RB 1LB  1LA 1LC  1RC 0LC", (3, 2), false);
    assert_connected!(
        "1RB 0LC  1LA 0LD  1LA ...  1LE 0RE  1RD 0LD",
        (5, 2),
        false
    );

    assert_connected!(
        "1RB 1LC  1RD 1RB  0RD 0RC  1LD 1LA",
        (4, 2),
        true
    );
    assert_connected!(
        "1RB 0LB  0LC 0RD  1RD 1LB  1LE 0RA  ... 1LA",
        (5, 2),
        true
    );
    assert_connected!(
        "1RB ...  0RC 0RE  0LD 1RC  1LB 0RA  1RD 1LC",
        (5, 2),
        true
    );
}

/**************************************/

#[cfg(test)]
macro_rules! assert_strict_cycle {
    ($prog:literal, ($s:literal, $c:literal), $ok:literal) => {{
        let result = Prog::<$s, $c>::from($prog).is_strict_cycle();
        assert!(if $ok { result } else { !result });
    }};
}

#[test]
fn test_strict_direction_cycle() {
    assert_strict_cycle!(
        "1RB 1LB  1RC 1LC  0LD 0RD  1RA 0LA",
        (4, 2),
        true
    );

    assert_strict_cycle!(
        "1RB 1LB  0LC 0RD  0LC 1LA  1RA 0LA",
        (4, 2),
        false
    );
}

/**************************************/

type Exitpoints = Dict<State, Vec<State>>;

impl<const s: usize, const c: usize> Prog<s, c> {
    fn get_exitpoints(&self) -> Exitpoints {
        let mut exitpoints = Exitpoints::new();

        for ((src, _), &(_, _, dst)) in self.iter() {
            if src == dst {
                continue;
            }

            exitpoints.entry(src).or_default().push(dst);
        }

        for conns in exitpoints.values_mut() {
            conns.sort_unstable();
            conns.dedup();
        }

        exitpoints
    }
}

#[cfg(test)]
macro_rules! assert_exitpoints {
    ( $( ($prog:literal, ($s:literal, $c:literal)) => { $($key:literal => [$($val:expr),* $(,)?]),* $(,)? } ),* $(,)? ) => { $(
        assert_eq!(
            Prog::<$s, $c>::from($prog).get_exitpoints(),
            Dict::from( [$(($key, vec![$($val),*]),)*] ),
        );
    )* };
}

#[test]
fn test_exitpoints() {
    assert_exitpoints!(
        ("1RB 1LB  1LA 1LC  1RC 0LC", (3, 2)) => {
            0 => [1],
            1 => [0, 2],
        },
        ("1RB 1LC  1RD 1RB  0RD 0RC  1LD 1LA", (4, 2)) => {
            0 => [1, 2],
            1 => [3],
            2 => [3],
            3 => [0],
        },
        ("1RB ...  0RC 0RE  0LD 1RC  1LB 0RA  1RD 1LC", (5, 2)) => {
            0 => [1],
            1 => [2, 4],
            2 => [3],
            3 => [0, 1],
            4 => [2, 3],
        },
    );
}
