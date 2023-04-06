use num_bigint::BigInt;
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
    Mult(Num, Num),
}

impl IntoPy<PyObject> for Op {
    fn into_py(self, py: Python) -> PyObject {
        match self {
            Self::Plus(val) => val.into_py(py),
            Self::Mult(val1, val2) => (val1, val2).into_py(py),
        }
    }
}

#[pyfunction]
pub fn calculate_diff(cnt1: Num, cnt2: Num, cnt3: Num) -> PyResult<Option<Op>> {
    if cnt1 == cnt2 && cnt2 == cnt3 {
        return Ok(None);
    }

    let plus = &cnt2 - &cnt1;
    if plus == &cnt3 - &cnt2 {
        return Ok(Some(Op::Plus(plus)));
    }

    let div = &cnt2 / &cnt1;

    if div == &cnt3 / &cnt2 {
        let mod_ = &cnt2 % &cnt1;

        if mod_ == &cnt3 % &cnt2 {
            return Ok(Some(Op::Mult(div, mod_)));
        }
    }

    Err(UnknownRule::new_err(""))
}
