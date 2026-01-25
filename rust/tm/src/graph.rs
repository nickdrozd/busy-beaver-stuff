use std::collections::{
    BTreeMap as Dict, BTreeSet as Set, HashMap, VecDeque,
};

use crate::{Color, Prog, State};

/**************************************/

const MAX_TAPE: usize = 15;
const MAX_NODES: usize = 1_000;

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

    pub fn graph_cant_quasihalt(&self) -> bool {
        self.graph_cant_quasihalt_fast()
            || self.graph_cant_quasihalt_abs()
    }

    /// Fast sufficient condition (pure control/SCC + one-direction `read=0` cycle lemma).
    fn graph_cant_quasihalt_fast(&self) -> bool {
        // Control adjacency: union of next-states over all read symbols.
        let mut adj: Vec<Vec<usize>> = vec![vec![]; states];

        for ((src, _), &(_, _, dst)) in self.iter() {
            adj[src as usize].push(dst as usize);
        }

        for st in 0..states {
            adj[st].sort_unstable();
            adj[st].dedup();
        }

        let reachable_all = Self::reach_from(&adj);

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

                let mut has_l = false;
                let mut has_r = false;

                // Also record which single direction it has (if any),
                // so we can do the 0-cycle check.
                for ((src, _), &(_, sh, dst)) in self.iter() {
                    let u = src as usize;
                    let v = dst as usize;

                    if !active[u] || !active[v] {
                        continue;
                    }

                    if !comp.contains(&u) || !comp.contains(&v) {
                        continue;
                    }

                    if sh {
                        has_r = true;
                    } else {
                        has_l = true;
                    }

                    // Bidirectional SCC: we cannot rule out an
                    // infinite bounded "bouncing" run.
                    if has_l && has_r {
                        return false;
                    }
                }

                // If we have a (cyclic) SCC but saw no internal
                // transitions from self.iter(), it's because those
                // transitions are absent; conservatively treat as
                // trap.
                if !has_l && !has_r {
                    return false;
                }

                // One-direction SCC: rule it out unless it contains a
                // feasible `read=0` cycle in that same direction.
                if self.has_zero_dir_cycle_in_comp(&comp, has_r) {
                    return false;
                }
            }
        }

        true
    }

    /// Stronger sound proof using a finite abstract-configuration graph.
    ///
    /// This is purely static/symbolic: it explores a finite
    /// over-approximation of reachable local tape windows and then
    /// checks for the existence of an abstract cycle that avoids each
    /// control state.
    fn graph_cant_quasihalt_abs(&self) -> bool {
        let (nodes, adj) = self.build_abs_graph();

        // If we hit the cap, we conservatively give up (no false proofs).
        if nodes.is_empty() || nodes.len() >= MAX_NODES {
            return false;
        }

        // All abstract nodes are reachable in the full graph by
        // construction. For each control state t, we must consider
        // *any* cycle that avoids t, even if reaching it required
        // visiting t earlier (eventual avoidance). Therefore we MUST
        // NOT recompute reachability on the filtered graph.
        for st in 0..states {
            let mut active = vec![true; nodes.len()];

            for (i, cfg) in nodes.iter().enumerate() {
                if cfg.state as usize == st {
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
        let halts = self.halt_slots();

        if halts.is_empty() {
            return true;
        }

        // Build control adjacency: union of next-states over all read symbols.
        let mut adj: Vec<Vec<usize>> = vec![vec![]; states];

        for ((src, _), &(_, _, dst)) in self.iter() {
            adj[src as usize].push(dst as usize);
        }

        for u in 0..states {
            adj[u].sort_unstable();
            adj[u].dedup();
        }

        let reachable = Self::reach_from(&adj);

        // Sufficient condition: every halting slot belongs to a
        // control state that is unreachable from the start (even in
        // the over-approx control graph). Then we can never arrive at
        // that missing slot.
        halts.iter().all(|&(st, _)| !reachable[st as usize])
    }

    pub fn graph_cant_blank(&self) -> bool {
        self.graph_cant_blank_fast() || self.graph_cant_blank_abs()
    }

    fn graph_cant_blank_fast(&self) -> bool {
        if !self.erase_slots().is_empty() {
            return false;
        }

        let Some(&(pr, _, _)) = self.get(&(0, 0)) else {
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
        let (nodes, _adj) = self.build_abs_graph();

        // If we hit the cap, we conservatively give up.
        if nodes.is_empty() || nodes.len() >= MAX_NODES {
            return false;
        }

        // If any reachable abstract node after the start has no *known* nonzero
        // symbol in the tracked window, then a concrete blank tape is still
        // compatible with that abstract state.
        for cfg in nodes.iter().skip(1) {
            if cfg.tape.iter().all(|&x| x == 0 || x as usize == colors)
            {
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
        let spin_triggers = self.zr_shifts();

        if spin_triggers.is_empty() {
            return true;
        }

        // Control adjacency: union of next-states over all read symbols.
        let mut adj: Vec<Vec<usize>> = vec![vec![]; states];

        for ((src, _), &(_, _, dst)) in self.iter() {
            adj[src as usize].push(dst as usize);
        }

        for u in 0..states {
            adj[u].sort_unstable();
            adj[u].dedup();
        }

        let reachable = Self::reach_from(&adj);

        for (st, _) in spin_triggers {
            if reachable[st as usize] {
                return false;
            }
        }

        true
    }

    fn build_abs_graph(&self) -> (Vec<AbsCfg>, Vec<Vec<usize>>) {
        #[expect(clippy::cast_possible_truncation)]
        let wild = colors as u8;
        let mut nodes: Vec<AbsCfg> = vec![];
        let mut adj: Vec<Vec<usize>> = vec![];
        let mut map: HashMap<AbsCfg, usize> = HashMap::new();
        let mut q: VecDeque<usize> = VecDeque::new();

        let start = AbsCfg::new_blank();
        nodes.push(start.clone());
        adj.push(vec![]);
        map.insert(start, 0);
        q.push_back(0);

        while let Some(u) = q.pop_front() {
            if nodes.len() >= MAX_NODES {
                break;
            }

            let succs = self.step_abs(&nodes[u], wild);

            for mut vcfg in succs {
                vcfg.normalize();

                let vid = if let Some(&id) = map.get(&vcfg) {
                    id
                } else {
                    let id = nodes.len();
                    nodes.push(vcfg.clone());
                    adj.push(vec![]);
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

    #[expect(clippy::cast_possible_truncation)]
    fn step_abs(&self, cfg: &AbsCfg, wild: u8) -> Vec<AbsCfg> {
        let head = cfg.head as usize;

        if head >= cfg.tape.len() {
            return vec![];
        }

        let cur = cfg.tape[head];
        let mut reads: Vec<u8> = vec![];

        if cur == wild {
            for r in 0..(colors as u8) {
                reads.push(r);
            }
        } else {
            reads.push(cur);
        }

        let mut out: Vec<AbsCfg> = vec![];

        for read in reads {
            let Some(&(write, sh, dst)) = self.get(&(cfg.state, read))
            else {
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
                    let new_cell =
                        if nxt.right_unknown { wild } else { 0 };
                    nxt.tape.push(new_cell);
                }
                nxt.head = new_head as u8;
            } else {
                // Left
                if head == 0 {
                    let new_cell =
                        if nxt.left_unknown { wild } else { 0 };
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

    fn reach_from(adj: &[Vec<usize>]) -> Vec<bool> {
        let mut seen = vec![false; states];

        let mut stack = vec![0];

        while let Some(u) = stack.pop() {
            if seen[u] {
                continue;
            }

            seen[u] = true;

            for &v in &adj[u] {
                if seen[v] {
                    continue;
                }

                stack.push(v);
            }
        }

        seen
    }

    /// In a strictly one-direction SCC, an infinite run from a blank tape
    /// is possible only if there is a `read=0` cycle whose transitions
    /// all have that same direction and stay within the SCC.
    fn has_zero_dir_cycle_in_comp(
        &self,
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
            if let Some(&(_, sh, dst)) = self.get(&(u as State, 0))
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
            let mut stack: Vec<usize> = vec![];

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
}

/**************************************/
// Abstract configuration graph (sound static over-approx)

/// Abstract tape symbol: `0..colors-1` are concrete,
/// and `wild` means "unknown".
///
/// `left_unknown/right_unknown` indicate whether cells beyond the
/// stored window may have been modified (unknown) or are still
/// guaranteed blank 0.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct AbsCfg {
    state: u8,
    head: u8,
    tape: Vec<u8>,
    left_unknown: bool,
    right_unknown: bool,
}

impl AbsCfg {
    fn new_blank() -> Self {
        let mut cfg = Self {
            state: 0,
            head: 0,
            tape: vec![0],
            left_unknown: false,
            right_unknown: false,
        };

        cfg.normalize();

        cfg
    }

    #[expect(clippy::cast_possible_truncation)]
    fn normalize(&mut self) {
        if self.tape.is_empty() {
            self.tape.push(0);
            self.head = 0;
            return;
        }
        if self.head as usize >= self.tape.len() {
            self.head = (self.tape.len() - 1) as u8;
        }

        if self.tape.len() <= MAX_TAPE {
            return;
        }

        let len = self.tape.len();
        let head = self.head as usize;
        let half = MAX_TAPE / 2;
        let mut start = head.saturating_sub(half);
        if start + MAX_TAPE > len {
            start = len - MAX_TAPE;
        }
        let end = start + MAX_TAPE;

        if start > 0 {
            self.left_unknown = true;
        }
        if end < len {
            self.right_unknown = true;
        }

        self.tape = self.tape[start..end].to_vec();
        self.head = (head - start) as u8;
    }
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
        let mut stack: Vec<(usize, usize)> = vec![];
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

/// Kosaraju SCC decomposition on a graph given as adjacency lists,
/// restricted to `active` nodes.
#[expect(clippy::items_after_statements)]
fn sccs_masked(
    states: usize,
    adj: &[Vec<usize>],
    active: &[bool],
) -> Vec<Vec<usize>> {
    let mut rev: Vec<Vec<usize>> = vec![vec![]; states];
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
    let mut order = vec![];

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

    let mut comps: Vec<Vec<usize>> = vec![];
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
        let mut comp = vec![];
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
