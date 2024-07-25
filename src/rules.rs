use std::collections::BTreeMap as Dict;

use crate::tape::{Count, Counts, Index, IndexTape};

/**************************************/

pub type Diff = i32;

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Op {
    Plus(Diff),
    Mult((Diff, Diff)),
}

pub type Rule = Dict<Index, Op>;

/**************************************/

enum DiffResult {
    Got(Op),
    Unknown,
}

fn calculate_diff(
    a: Diff,
    b: Diff,
    c: Diff,
    d: Diff,
) -> Option<DiffResult> {
    if a == b && b == c && c == d {
        return None;
    }

    let Some(diff_1) = b.checked_sub(a) else {
        return Some(DiffResult::Unknown);
    };

    let Some(diff_2) = c.checked_sub(b) else {
        return Some(DiffResult::Unknown);
    };

    if diff_1 == diff_2 && diff_2 == (d - c) {
        return Some(DiffResult::Got(Op::Plus(diff_1)));
    }

    if a == 0 || b == 0 {
        return Some(DiffResult::Unknown);
    }

    let divmod1 = (b / a, b % a);
    let divmod2 = (c / b, c % b);

    if divmod1 == divmod2 && divmod2 == (d / c, d % c) {
        return Some(DiffResult::Got(Op::Mult(divmod1)));
    }

    Some(DiffResult::Unknown)
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
            match calculate_diff(
                a as Diff, b as Diff, c as Diff, d as Diff,
            ) {
                None => continue,
                Some(DiffResult::Unknown) => {
                    return None;
                },
                Some(DiffResult::Got(op)) => {
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
        let Op::Plus(diff) = *diff else {
            unimplemented!()
        };

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
                    apply_plus(tape.get_count(pos), plus, times)?
                }
            },
            Op::Mult(_) => unimplemented!(),
        };

        tape.set_count(pos, result);
    }

    Some(times)
}

fn apply_plus(count: Count, diff: Diff, times: Count) -> Option<Count> {
    let diff: Count = diff.unsigned_abs().into();

    let mult = diff.checked_mul(times)?;

    Some(count + mult)
}
