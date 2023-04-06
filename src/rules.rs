use pyo3::exceptions::PyException;
use pyo3::prelude::*;

#[pyclass(extends=PyException)]
pub struct UnknownRule {}

#[pymethods]
impl UnknownRule {
    #[new]
    const fn new() -> Self {
        Self {}
    }
}

#[pyclass(extends=PyException)]
pub struct InfiniteRule {}

#[pymethods]
impl InfiniteRule {
    #[new]
    const fn new() -> Self {
        Self {}
    }
}

#[pyclass(extends=PyException)]
pub struct RuleLimit {}

#[pymethods]
impl RuleLimit {
    #[new]
    const fn new() -> Self {
        Self {}
    }
}
