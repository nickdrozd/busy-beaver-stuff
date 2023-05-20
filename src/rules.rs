use pyo3::create_exception;
use pyo3::exceptions::PyException;

create_exception!(rules, UnknownRule, PyException);
create_exception!(rules, InfiniteRule, PyException);
create_exception!(rules, RuleLimit, PyException);
