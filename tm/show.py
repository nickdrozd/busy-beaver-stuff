# ruff: noqa: F401, I001
from tm.rust_stuff import (
    show_comp,
    show_slot,
    show_instr,
    show_state,
)

########################################

from math import log10
from typing import Any, Final

TRUNCATE_LIMIT: Final[int] = 10 ** 12


def show_int(num: int) -> str:
    if (abs_val := abs(num)) < TRUNCATE_LIMIT:
        return str(num)

    est = log10(abs_val)
    sign = '-' if num < 0 else ''

    return f'{sign}(~10^{est:.0f})'


def show_number(num: Any) -> str:  # type: ignore[explicit-any]  # noqa: ANN401
    return (
        show_int(num)
        if isinstance(num, int) else
        str(num)
    )
