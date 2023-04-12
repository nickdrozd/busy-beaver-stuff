use std::collections::HashMap;

use num_bigint::BigInt;
use num_integer::Integer;
use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;

create_exception!(rules, UnknownRule, PyException);
create_exception!(rules, InfiniteRule, PyException);
create_exception!(rules, RuleLimit, PyException);

type Num = BigInt;

#[derive(Debug)]
pub enum Op {
    Plus(Num),
    Mult((Num, Num)),
}

type Index = (usize, usize);

type Rule = HashMap<Index, Op>;

type Counts = (Vec<Num>, Vec<Num>);

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
pub fn calculate_diff(cnt1: Num, cnt2: Num, cnt3: Num) -> PyResult<Option<Op>> {
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
            rule.insert((0, i), op);
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
            rule.insert((1, i), op);
        } else if let Err(err) = diff {
            return Err(err);
        }
    }

    let all_plus_positive = rule.values().all(|op| match op {
        Op::Plus(val) => val >= &BigInt::from(0),
        Op::Mult(_) => true,
    });

    if all_plus_positive {
        Err(InfiniteRule::new_err(""))
    } else {
        Ok(rule)
    }
}

#[allow(dead_code)]
fn log10_limit(mut num: Num) -> bool {
    for _ in 0..10 {
        num /= 10;

        if num == BigInt::from(0) {
            return false;
        }
    }

    true
}
