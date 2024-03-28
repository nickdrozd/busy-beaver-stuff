#[cfg(test)]
use std::collections::HashMap;

use pyo3::create_exception;
use pyo3::exceptions::PyException;

#[cfg(test)]
use crate::instrs::Shift;

/**************************************/

create_exception!(rules, UnknownRule, PyException);
create_exception!(rules, InfiniteRule, PyException);
create_exception!(rules, RuleLimit, PyException);

/**************************************/

pub type Count = u64;

#[cfg(test)]
pub type Diff = i32;

#[cfg(test)]
pub type Index = (Shift, usize);

#[cfg(test)]
#[derive(PartialEq, Eq)]
pub enum Op {
    Plus(Diff),
}

#[cfg(test)]
pub type Rule = HashMap<Index, Op>;

/**************************************/

#[cfg(test)]
pub trait ApplyRule {
    fn get_count(&self, index: &Index) -> Count;

    fn set_count(&mut self, index: &Index, val: Count);

    fn count_apps(&self, rule: &Rule) -> Option<(Count, Index, Count)> {
        let mut apps: Option<(Count, Index, Count)> = None;

        for (pos, diff) in rule {
            let Op::Plus(diff) = diff;

            if *diff >= 0 {
                continue;
            }

            let count = self.get_count(pos);
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

    fn apply_rule(&mut self, rule: &Rule) -> Option<Count> {
        let (times, min_pos, min_res) = self.count_apps(rule)?;

        for (pos, diff) in rule {
            let result = match diff {
                Op::Plus(plus) => {
                    if *pos == min_pos {
                        assert!(*plus < 0);
                        min_res
                    } else {
                        apply_plus(self.get_count(pos), *plus, times)
                    }
                }
            };

            self.set_count(pos, result);
        }

        Some(times)
    }
}

#[cfg(test)]
fn apply_plus(count: Count, diff: Diff, times: Count) -> Count {
    let diff: Count = diff.unsigned_abs().into();

    count + (diff * times)
}
