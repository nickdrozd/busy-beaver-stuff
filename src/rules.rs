use std::collections::HashMap;

use pyo3::{create_exception, exceptions::PyException};

use crate::tape::{Count, Counts, Index, IndexTape};

/**************************************/

create_exception!(rules, UnknownRule, PyException);
create_exception!(rules, InfiniteRule, PyException);
create_exception!(rules, RuleLimit, PyException);

/**************************************/

pub type Diff = i32;

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Op {
    Plus(Diff),
}

pub type Rule = HashMap<Index, Op>;

/**************************************/

enum DiffResult {
    Got(Op),
    Unknown,
}

const fn calculate_diff(
    a: Count,
    b: Count,
    c: Count,
    d: Count,
) -> Option<DiffResult> {
    if a == b && b == c && c == d {
        return None;
    }

    let diff_1 = (b - a) as Diff;
    let diff_2 = (c - b) as Diff;

    if diff_1 != diff_2 || diff_2 != (d - c) as Diff {
        return Some(DiffResult::Unknown);
    }

    Some(DiffResult::Got(Op::Plus(diff_1)))
}

pub fn make_rule(
    (l1, r1): &Counts,
    (l2, r2): &Counts,
    (l3, r3): &Counts,
    (l4, r4): &Counts,
) -> Option<Rule> {
    let countses: Vec<Vec<_>> = vec![
        l1.iter()
            .zip(l2.iter())
            .zip(l3.iter())
            .zip(l4.iter())
            .map(|(((a, b), c), d)| (*a, *b, *c, *d))
            .collect(),
        r1.iter()
            .zip(r2.iter())
            .zip(r3.iter())
            .zip(r4.iter())
            .map(|(((a, b), c), d)| (*a, *b, *c, *d))
            .collect(),
    ];

    let mut rule = Rule::new();

    for (s, spans) in countses.iter().enumerate() {
        for (i, &(a, b, c, d)) in spans.iter().enumerate() {
            match calculate_diff(a, b, c, d)? {
                DiffResult::Unknown => {
                    return None;
                },
                DiffResult::Got(op) => {
                    rule.insert((s == 1, i), op);
                },
            }
        }
    }

    Some(rule)
}

/**************************************/

fn count_apps(
    rule: &Rule,
    tape: &impl IndexTape,
) -> Option<(Count, Index, Count)> {
    let mut apps: Option<(Count, Index, Count)> = None;

    for (pos, diff) in rule {
        let Op::Plus(diff) = *diff;

        if diff >= 0 {
            continue;
        }

        let count = tape.get_count(pos);
        let absdiff: Count = diff.unsigned_abs().into();

        if absdiff >= count {
            return None;
        }

        let div = count / absdiff;
        let rem = count % absdiff;

        let (times, min_res) = if rem > 0 {
            (div, rem)
        } else {
            (div - 1, absdiff)
        };

        if let Some((curr, _, _)) = apps {
            if times < curr {
                apps = Some((times, *pos, min_res));
            }
        } else {
            apps = Some((times, *pos, min_res));
        }
    }

    apps
}

pub fn apply_rule(
    rule: &Rule,
    tape: &mut impl IndexTape,
) -> Option<Count> {
    let (times, min_pos, min_res) = count_apps(rule, tape)?;

    for (pos, diff) in rule {
        let result = match *diff {
            Op::Plus(plus) => {
                if *pos == min_pos {
                    assert!(plus < 0);
                    min_res
                } else {
                    apply_plus(tape.get_count(pos), plus, times)
                }
            },
        };

        tape.set_count(pos, result);
    }

    Some(times)
}

fn apply_plus(count: Count, diff: Diff, times: Count) -> Count {
    let diff: Count = diff.unsigned_abs().into();

    count + (diff * times)
}
