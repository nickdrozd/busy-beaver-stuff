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

    #[expect(clippy::cast_possible_truncation)]
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

        let reachable_all = reach_from(states, &adj, start);
        if reachable_all.iter().any(|&b| !b) {
            return false;
        }

        // For each state t, if there exists a reachable directed
        // cycle that avoids t, then there exists an infinite
        // execution that (after some time) never returns to t.
        for t in 0..states {
            let mut ep: Exitpoints = Exitpoints::new();

            for u in 0..states {
                if u == t || !reachable_all[u] {
                    continue;
                }
                let su = u as State;
                for &v in &adj[u] {
                    if v == t || !reachable_all[v] {
                        continue;
                    }
                    ep.entry(su).or_default().push(v as State);
                }
            }

            for conns in ep.values_mut() {
                conns.sort_unstable();
                conns.dedup();
            }

            if has_cycle(states, &ep) {
                return false;
            }
        }

        true
    }
}

fn has_cycle(states: usize, adj: &Exitpoints) -> bool {
    fn dfs(u: usize, adj: &Exitpoints, mark: &mut [u8]) -> bool {
        mark[u] = 1;

        #[expect(clippy::cast_possible_truncation)]
        let su = u as State;
        if let Some(neis) = adj.get(&su) {
            for &v_state in neis {
                let v = v_state as usize;

                // Back-edge => cycle
                if mark[v] == 1 {
                    return true;
                }

                if mark[v] == 0 && dfs(v, adj, mark) {
                    return true;
                }
            }
        }

        mark[u] = 2;
        false
    }

    // 0 = unvisited, 1 = visiting, 2 = done
    let mut mark = vec![0; states];

    for u in 0..states {
        if mark[u] == 0 && dfs(u, adj, &mut mark) {
            return true;
        }
    }

    false
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
