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
    #[expect(clippy::excessive_nesting)]
    fn graph_cant_quasihalt_fast(&self) -> bool {
        // Control adjacency: union of next-states over all read colors.
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

        // CEGAR-style refinement: when the coarse control/SCC logic finds a potential
        // avoid-`t` trap SCC, try to confirm it in the bounded abstract configuration graph.
        //
        // If the abstract graph cannot witness *any* cycle that stays within the SCC's control
        // states while avoiding `t`, we treat the SCC as spurious and keep searching.
        //
        // Soundness note: `build_abs_graph()` is an over-approx of BB-from-blank reachability, so
        // a concrete trapped cycle implies an abstract trapped cycle. Therefore, absence of an
        // abstract cycle is enough to rule the SCC out.
        let mut abs_cache: Option<(Vec<AbsCfg>, Vec<Vec<usize>>)> =
            None;

        let mut cegar_confirms_trap = |comp: &[usize],
                                       t: usize|
         -> bool {
            // If we can't build the abstract graph (cap hit), fall back to conservative behavior:
            // treat as confirmed.
            let (nodes, a_adj) = if let Some((nodes, a_adj)) =
                &abs_cache
            {
                (nodes, a_adj)
            } else {
                let built = self.build_abs_graph();
                if built.0.is_empty() || built.0.len() >= MAX_NODES {
                    return true;
                }
                abs_cache = Some(built);
                let (nodes, a_adj) = abs_cache.as_ref().unwrap();
                (nodes, a_adj)
            };

            // Control-state membership mask for the SCC.
            let mut in_comp = vec![false; states];
            for &u in comp {
                if u < states {
                    in_comp[u] = true;
                }
            }

            // Active abstract nodes: state in SCC and not equal to target t.
            let mut active_abs = vec![false; nodes.len()];
            for (i, cfg) in nodes.iter().enumerate() {
                let st = cfg.state as usize;
                if st < states && st != t && in_comp[st] {
                    active_abs[i] = true;
                }
            }

            dyn_cycle_exists(a_adj, &active_abs)
        };

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
                let mut has_nonzero_read = false;
                let mut has_nonzero_write = false;

                // Also record which single direction it has (if any),
                // so we can do the 0-cycle check.
                for ((src, read), &(write, sh, dst)) in self.iter() {
                    let u = src as usize;
                    let v = dst as usize;

                    if !active[u] || !active[v] {
                        continue;
                    }

                    if !comp.contains(&u) || !comp.contains(&v) {
                        continue;
                    }

                    if read != 0 {
                        has_nonzero_read = true;
                    }
                    if write != 0 {
                        has_nonzero_write = true;
                    }

                    if sh {
                        has_r = true;
                    } else {
                        has_l = true;
                    }
                }

                // If we have a (cyclic) SCC but saw no internal
                // transitions from self.iter(), it's because those
                // transitions are absent; conservatively treat as
                // trap.
                if !has_l && !has_r {
                    if cegar_confirms_trap(comp.as_slice(), t) {
                        return false;
                    }
                    continue;
                }

                // Bidirectional SCC: try a stronger (still sound)
                // drift-only check.
                //
                // If *every* directed cycle in the SCC has strictly
                // positive net displacement (or strictly negative),
                // then any infinite run trapped in the SCC must drift
                // unboundedly and thus visit infinitely many fresh
                // blank cells. Each first-visit to a fresh cell
                // forces a `read=0` transition, so an infinite
                // trapped run requires a cycle in the `read=0`
                // induced subgraph.
                //
                // If we can prove the SCC is drift-only (no
                // 0/opp-sign displacement cycles), we can safely rule
                // it out unless it contains a `read=0` cycle
                // (direction-free).
                if has_l && has_r {
                    if self.comp_has_uniform_drift(&comp) {
                        if self.has_zero_cycle_in_comp(&comp) {
                            if cegar_confirms_trap(comp.as_slice(), t) {
                                return false;
                            }
                            // Abstract refinement could not confirm a trapped cycle inside this SCC.
                            continue;
                        }
                        // No `read=0` cycle => cannot be
                        // an infinite trap from blank.
                        continue;
                    }

                    // Bounce-feasibility refinement (sound):
                    // If the SCC cannot *ever* read a nonzero symbol, then from a blank tape
                    // any trapped execution in the SCC can only traverse `read=0` transitions.
                    // Likewise, if the SCC can read nonzero but can never *write* a nonzero,
                    // then nonzero reads are impossible from blank.
                    // In either case, the SCC is a trap only if the `read=0` induced subgraph
                    // contains a directed cycle.
                    if !has_nonzero_read || !has_nonzero_write {
                        if self.has_zero_cycle_in_comp(&comp) {
                            if cegar_confirms_trap(comp.as_slice(), t) {
                                return false;
                            }
                            // Abstract refinement could not confirm a trapped cycle inside this SCC.
                            continue;
                        }
                        // No `read=0` cycle => cannot sustain an infinite run from blank.
                        continue;
                    }

                    // Otherwise, conservatively treat as a possible bounded bounce trap,
                    // but try to confirm it in the abstract graph first.
                    if cegar_confirms_trap(comp.as_slice(), t) {
                        return false;
                    }
                    continue;
                }

                // One-direction SCC: rule it out unless it contains a
                // feasible `read=0` cycle in that same direction.
                if self.has_zero_dir_cycle_in_comp(&comp, has_r)
                    && cegar_confirms_trap(comp.as_slice(), t)
                {
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

        // Stronger (still sound) sufficient condition:
        // prove that no reachable configuration can ever be in a halting *slot*
        // (state + color-under-head).
        //
        // We use the abstract configuration graph to compute which head-read
        // color sets are reachable in each state. If even this over-approx
        // never reaches (st, co), then the concrete machine cannot halt there.
        let (nodes, _adj) = self.build_abs_graph();

        // If we hit the cap, conservatively give up.
        if nodes.is_empty() || nodes.len() >= MAX_NODES {
            return false;
        }

        #[expect(clippy::cast_possible_truncation)]
        let wild: u8 = colors as u8;

        let mut reachable_head: Vec<Vec<bool>> =
            vec![vec![false; colors]; states];

        for cfg in &nodes {
            let st = cfg.state as usize;
            if st >= states {
                continue;
            }
            let head = cfg.head as usize;
            if head >= cfg.tape.len() {
                continue;
            }
            let cell = cfg.tape[head];
            if cell == wild {
                for co in 0..colors {
                    reachable_head[st][co] = true;
                }
            } else {
                let co = cell as usize;
                if co < colors {
                    reachable_head[st][co] = true;
                }
            }
        }

        // Every halting slot must be unreachable as a (state, head-color) pair.
        for (st, co) in halts {
            if reachable_head[st as usize][co as usize] {
                return false;
            }
        }

        true
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
    fn graph_cant_blank_abs(&self) -> bool {
        #[expect(clippy::cast_possible_truncation)]
        let wild: u8 = colors as u8;

        let (nodes, _adj) = self.build_abs_graph();

        if nodes.is_empty() || nodes.len() >= MAX_NODES {
            return false;
        }

        // If a configuration is compatible with a blank tape (all cells are 0
        // or unknown), we cannot refute blanking.
        for cfg in nodes.iter().skip(1) {
            if cfg.tape.iter().all(|&x| x == 0 || x == wild) {
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
    pub fn graph_cant_spinout(&self) -> bool {
        let spin_triggers = self.zr_shifts();

        if spin_triggers.is_empty() {
            return true;
        }

        #[expect(clippy::cast_possible_truncation)]
        let wild: u8 = colors as u8;

        // Stronger (still sound): rule out spinout triggers by proving
        // that the trigger *slot* (state, read=0 under head) is unreachable
        // in the bounded abstract graph.
        let (nodes, _adj) = self.build_abs_graph();

        // If we hit the cap, conservatively give up.
        if nodes.is_empty() || nodes.len() >= MAX_NODES {
            return false;
        }

        for (st, _dir) in spin_triggers {
            let target = st;
            for cfg in &nodes {
                if cfg.state != target {
                    continue;
                }
                let head = cfg.head as usize;
                if head >= cfg.tape.len() {
                    continue;
                }
                let cell = cfg.tape[head];
                // If reading 0 is possible here, the trigger slot might be reached.
                if cell == 0 || cell == wild {
                    return false;
                }
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
            // Unknown color at head: branch over all possible reads.
            reads.extend(0..(colors as u8));
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

    /// In an SCC where the head must drift unboundedly (all directed
    /// cycles have strictly nonzero net displacement with the same
    /// sign), an infinite trap run from blank requires a directed
    /// cycle in the `read=0` induced subgraph restricted to that SCC.
    fn has_zero_cycle_in_comp(&self, comp: &[usize]) -> bool {
        let mut in_comp = vec![false; states];

        for &u in comp {
            if u < states {
                in_comp[u] = true;
            }
        }

        // next[u] = dst on read=0 if it stays in comp (direction ignored).
        let mut next: Vec<Option<usize>> = vec![None; states];

        for &u in comp {
            #[expect(clippy::cast_possible_truncation)]
            if let Some(&(_, _, dst)) = self.get(&(u as State, 0)) {
                let v = dst as usize;

                if in_comp[v] {
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

    /// Returns true iff every directed cycle in `comp` has strictly
    /// nonzero net displacement with the same sign (i.e. either all
    /// cycles drift right, or all cycles drift left).
    ///
    /// This is a conservative check based only on internal transition
    /// directions.
    fn comp_has_uniform_drift(&self, comp: &[usize]) -> bool {
        // Helper: compute the minimum cycle weight under a signed displacement.
        // sign=+1 => weights are +1 for R, -1 for L
        // sign=-1 => weights are -1 for R, +1 for L (inverted)
        fn min_cycle_weight_in_comp<
            const states: usize,
            const colors: usize,
        >(
            prog: &Prog<states, colors>,
            comp: &[usize],
            sign: i32,
        ) -> i32 {
            const INF: i32 = 1_000_000;
            let m = comp.len();
            if m == 0 {
                return i32::MIN;
            }

            let mut idx = vec![usize::MAX; states];
            for (i, &u) in comp.iter().enumerate() {
                if u < states {
                    idx[u] = i;
                }
            }

            let mut dist = vec![vec![INF; m]; m];
            for i in 0..m {
                dist[i][i] = 0;
            }

            // Direct edges
            for ((src, _), &(_, sh, dst)) in prog.iter() {
                let u = src as usize;
                let v = dst as usize;
                if u >= states || v >= states {
                    continue;
                }
                let iu = idx[u];
                let iv = idx[v];
                if iu == usize::MAX || iv == usize::MAX {
                    continue;
                }

                let w0: i32 = if sh { 1 } else { -1 };
                let w = sign * w0;
                if w < dist[iu][iv] {
                    dist[iu][iv] = w;
                }
            }

            // Floyd–Warshall
            for k in 0..m {
                for i in 0..m {
                    if dist[i][k] >= INF / 2 {
                        continue;
                    }
                    for j in 0..m {
                        if dist[k][j] >= INF / 2 {
                            continue;
                        }
                        let cand = dist[i][k] + dist[k][j];
                        if cand < dist[i][j] {
                            dist[i][j] = cand;
                        }
                    }
                }
            }

            // Negative cycle => minimum cycle weight is effectively -∞ (certainly <= 0).
            for i in 0..m {
                if dist[i][i] < 0 {
                    return i32::MIN;
                }
            }

            // Minimum cycle weight: min over edges (u->v) of w(u,v) + dist[v][u].
            let mut best = INF;
            for ((src, _), &(_, sh, dst)) in prog.iter() {
                let u = src as usize;
                let v = dst as usize;
                if u >= states || v >= states {
                    continue;
                }
                let iu = idx[u];
                let iv = idx[v];
                if iu == usize::MAX || iv == usize::MAX {
                    continue;
                }

                if dist[iv][iu] >= INF / 2 {
                    continue;
                }

                let w0: i32 = if sh { 1 } else { -1 };
                let w = sign * w0;
                let cyc = w + dist[iv][iu];
                if cyc < best {
                    best = cyc;
                }
            }

            if best >= INF / 2 {
                // Couldn't form any cycle weight via edge + return path.
                // Conservatively treat as "not uniformly drifting".
                i32::MIN
            } else {
                best
            }
        }

        // All cycles drift right  <=> minimum cycle displacement is > 0.
        let min_pos = min_cycle_weight_in_comp(self, comp, 1);
        if min_pos > 0 {
            return true;
        }

        // All cycles drift left <=> minimum cycle displacement of inverted weights is > 0.
        let min_neg = min_cycle_weight_in_comp(self, comp, -1);
        min_neg > 0
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

/// Abstract tape window for the symbolic execution graph.
///
/// Each tracked tape cell stores a *set* of possible colors (bitmask).
///
/// `left_unknown/right_unknown` indicate whether cells beyond the stored
/// window may have been modified (unknown) or are still guaranteed blank 0.
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
