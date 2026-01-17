use std::collections::{BTreeMap as Dict, BTreeSet as Set};

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

    pub fn graph_cant_quasihalt(&self) -> bool {
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

        // All states must be control-reachable; otherwise we cannot prove non-quasihalt.
        let reachable_all = reach_from(states, &adj, start);
        if reachable_all.iter().any(|&b| !b) {
            return false;
        }

        // State `t` can be visited only finitely many times iff there exists an infinite suffix
        // of the execution that never visits `t` again. Such a suffix must eventually remain
        // within a cyclic SCC in the control graph with `t` removed.
        //
        // We conservatively treat *any* cyclic SCC as a possible trap, except that we can
        // soundly rule out strictly one-directional SCCs (all internal shifts are L or all are R)
        // that do NOT contain a cycle induced by the `read=0` transitions in that same direction.
        //
        // Soundness sketch (BB-from-blank):
        // - In a one-direction SCC, the head position is strictly monotone, so no tape cell is revisited.
        // - Starting from a blank tape, every newly visited cell is `0`, so from that point on only
        //   `read=0` transitions can occur.
        // - Therefore, an infinite run staying inside such an SCC requires a `read=0` cycle.
        for t in 0..states {
            // Active nodes: control-reachable and not equal to t.
            let mut active = vec![false; states];
            for u in 0..states {
                active[u] = reachable_all[u] && u != t;
            }

            // SCCs of the induced subgraph on `active` nodes.
            for comp in sccs_masked(states, &adj, &active) {
                if !scc_has_cycle(&comp, &adj) {
                    continue;
                }

                // Build a membership mask for this SCC.
                let mut in_comp = vec![false; states];
                for &u in &comp {
                    if u < states {
                        in_comp[u] = true;
                    }
                }

                // Check whether the SCC contains both L and R internal shifts.
                let mut has_l = false;
                let mut has_r = false;

                for &u in &comp {
                    #[expect(clippy::cast_possible_truncation)]
                    let su = u as State;
                    for read in 0..(colors as Color) {
                        let Some(&(_write, sh, dst)) =
                            self.get(&(su, read))
                        else {
                            continue;
                        };
                        let v = dst as usize;
                        if v < states && active[v] && in_comp[v] {
                            if sh {
                                has_r = true;
                            } else {
                                has_l = true;
                            }
                            if has_l && has_r {
                                // Bidirectional SCC: cannot be statically ruled out.
                                return false;
                            }
                        }
                    }
                }

                // Cyclic SCC but no internal transitions found: be conservative.
                if !has_l && !has_r {
                    return false;
                }

                // One-direction SCC: potentially a realizable trap only if it has a 0-cycle.
                if has_zero_dir_cycle_in_comp::<states, colors>(
                    self, &comp, has_r,
                ) {
                    return false;
                }
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
