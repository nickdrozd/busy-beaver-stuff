#[cfg(test)]
use std::collections::BTreeMap as Dict;

use pyo3::{create_exception, exceptions::PyException};

#[cfg(test)]
use crate::tape::{Count, Index, IndexTape};

/**************************************/

create_exception!(rules, UnknownRule, PyException);
create_exception!(rules, InfiniteRule, PyException);
create_exception!(rules, RuleLimit, PyException);

/**************************************/

#[cfg(test)]
pub type Diff = i32;

#[cfg(test)]
#[derive(PartialEq, Eq)]
pub enum Op {
    Plus(Diff),
}

#[cfg(test)]
pub type Rule = Dict<Index, Op>;

/**************************************/

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
fn apply_plus(count: Count, diff: Diff, times: Count) -> Count {
    let diff: Count = diff.unsigned_abs().into();

    count + (diff * times)
}
