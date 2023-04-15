use std::collections::HashMap;

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::sign::Signed;
use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;

use crate::instrs::Shift;

create_exception!(rules, UnknownRule, PyException);
create_exception!(rules, InfiniteRule, PyException);
create_exception!(rules, RuleLimit, PyException);

pub type Count = BigInt;

#[derive(Debug, FromPyObject)]
pub enum Op {
    Plus(Count),
    Mult((Count, Count)),
}

pub type Index = (Shift, usize);

pub type Rule = HashMap<Index, Op>;

type Counts = (Vec<Count>, Vec<Count>);

impl IntoPy<PyObject> for Op {
    fn into_py(self, py: Python) -> PyObject {
        match self {
            Self::Plus(val) => val.into_py(py),
            Self::Mult(val) => val.into_py(py),
        }
    }
}

#[pyfunction]
#[allow(clippy::needless_pass_by_value)]
pub fn calculate_diff(cnt1: Count, cnt2: Count, cnt3: Count) -> PyResult<Option<Op>> {
    if cnt1 == cnt2 && cnt2 == cnt3 {
        return Ok(None);
    }

    let plus = &cnt2 - &cnt1;
    if plus == &cnt3 - &cnt2 {
        return Ok(Some(Op::Plus(plus)));
    }

    let div_rem = cnt2.div_rem(&cnt1);

    if div_rem == cnt3.div_rem(&cnt2) {
        return Ok(Some(Op::Mult(div_rem)));
    }

    Err(UnknownRule::new_err(""))
}

#[pyfunction]
#[allow(clippy::needless_pass_by_value)]
pub fn make_rule(counts1: Counts, counts2: Counts, counts3: Counts) -> PyResult<Rule> {
    let mut rule = HashMap::new();

    for i in 0..counts1.0.len() {
        let diff = calculate_diff(
            counts1.0[i].clone(),
            counts2.0[i].clone(),
            counts3.0[i].clone(),
        );

        if let Ok(Some(op)) = diff {
            rule.insert((false, i), op);
        } else if let Err(err) = diff {
            return Err(err);
        }
    }

    for i in 0..counts1.1.len() {
        let diff = calculate_diff(
            counts1.1[i].clone(),
            counts2.1[i].clone(),
            counts3.1[i].clone(),
        );

        if let Ok(Some(op)) = diff {
            rule.insert((true, i), op);
        } else if let Err(err) = diff {
            return Err(err);
        }
    }

    let all_plus_positive = rule.values().all(|op| match op {
        Op::Plus(val) => val >= &Count::from(0),
        Op::Mult(_) => true,
    });

    if all_plus_positive {
        Err(InfiniteRule::new_err(""))
    } else {
        Ok(rule)
    }
}

#[allow(dead_code)]
fn log10_limit(mut num: Count) -> bool {
    for _ in 0..10 {
        num /= 10;

        if num == Count::from(0) {
            return false;
        }
    }

    true
}

pub trait ApplyRule {
    fn __getitem__(&self, index: &Index) -> Count;

    fn __setitem__(&mut self, index: &Index, val: Count);

    fn count_apps(&self, rule: &Rule) -> Option<Count> {
        let mut divs: Vec<Count> = vec![];

        let zero = BigInt::from(0);

        for (pos, diff) in rule {
            if let Op::Plus(plus_diff) = diff {
                if plus_diff >= &zero {
                    continue;
                }

                let abs_diff = plus_diff.abs();

                let count = self.__getitem__(pos);

                if abs_diff >= count {
                    return None;
                }

                let (div, rem) = count.div_rem(&abs_diff);
                divs.push(if rem > zero { div } else { div - 1 });
            }
        }

        Some(divs.iter().min()?.clone())
    }

    fn apply_rule_rs(&mut self, rule: &Rule) -> PyResult<Option<Count>> {
        let Some(times) = self.count_apps(rule) else { return Ok(None) };

        if rule.values().any(|op| !matches!(op, Op::Plus(_))) && log10_limit(times.clone()) {
            return Err(RuleLimit::new_err(""));
        }

        for (pos, diff) in rule {
            match diff {
                Op::Plus(plus_diff) => {
                    let new_val = self.__getitem__(pos) + plus_diff * &times;
                    self.__setitem__(pos, new_val);
                }
                Op::Mult((div, rem)) => {
                    let term = &times * (1 + (&times - div) / (div - 1));
                    let new_val = self.__getitem__(pos) * &times + rem * term;
                    self.__setitem__(pos, new_val);
                }
            }
        }

        Ok(Some(times))
    }
}
