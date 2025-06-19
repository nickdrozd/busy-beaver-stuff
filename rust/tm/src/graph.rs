use std::collections::{BTreeMap as Dict, BTreeSet as Set};

use crate::instrs::Prog;

type State = u8;

/**************************************/

pub fn is_connected(prog: &Prog, states: State) -> bool {
    if prog.instrs.values().all(|&(_, _, state)| state != 0) {
        return false;
    }

    let exitpoints = get_exitpoints(prog);

    if exitpoints.len() < states as usize {
        return false;
    }

    let last_state = states - 1;

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

#[cfg(test)]
use crate::instrs::Parse as _;

#[cfg(test)]
const UNCONNECTED: [(&str, State); 2] = [
    ("1RB 1LB  1LA 1LC  1RC 0LC", 3),
    ("1RB 0LC  1LA 0LD  1LA ...  1LE 0RE  1RD 0LD", 5),
];

#[cfg(test)]
const CONNECTED: [(&str, State); 3] = [
    ("1RB 1LC  1RD 1RB  0RD 0RC  1LD 1LA", 4),
    ("1RB 0LB  0LC 0RD  1RD 1LB  1LE 0RA  ... 1LA", 5),
    ("1RB ...  0RC 0RE  0LD 1RC  1LB 0RA  1RD 1LC", 5),
];

#[test]
fn test_connected() {
    for (prog, states) in UNCONNECTED {
        assert!(!is_connected(&Prog::read(prog), states));
    }

    for (prog, states) in CONNECTED {
        assert!(is_connected(&Prog::read(prog), states));
    }
}

/**************************************/

type Exitpoints = Dict<State, Vec<State>>;

fn get_exitpoints(prog: &Prog) -> Exitpoints {
    let mut exitpoints = Exitpoints::new();

    for (&(src, _), &(_, _, dst)) in &prog.instrs {
        if src == dst {
            continue;
        }

        exitpoints
            .entry(src as State)
            .or_default()
            .push(dst as State);
    }

    for conns in exitpoints.values_mut() {
        conns.sort_unstable();
        conns.dedup();
    }

    exitpoints
}

#[cfg(test)]
macro_rules! dict_from {
    ($( $key:expr => [ $( $val:expr ),* ] ),* $(,)?) => {
        Dict::from(
            [$(($key, vec![$($val),*]),)*]
        )
    }
}

#[cfg(test)]
macro_rules! assert_exitpoints {
    ($input:expr, { $($key:expr => [$($val:expr),*]),* $(,)? }) => {
        assert_eq!(
            get_exitpoints(&Prog::read($input)),
            dict_from! { $($key => [$($val),*]),* },
        );
    }
}

#[test]
fn test_exitpoints() {
    assert_exitpoints!(
        "1RB 1LB  1LA 1LC  1RC 0LC",
        { 0 => [1], 1 => [0, 2] }
    );

    assert_exitpoints!(
        "1RB 1LC  1RD 1RB  0RD 0RC  1LD 1LA",
        { 0 => [1, 2], 1 => [3], 2 => [3], 3 => [0] }
    );

    assert_exitpoints!(
        "1RB ...  0RC 0RE  0LD 1RC  1LB 0RA  1RD 1LC",
        { 0 => [1], 1 => [2, 4], 2 => [3], 3 => [0, 1], 4 => [2, 3] }
    );
}
