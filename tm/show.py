from __future__ import annotations

import math
from typing import TYPE_CHECKING


# pylint: disable = unused-import
from tm.rust_stuff import show_state
from tm.rust_stuff import show_slot
from tm.rust_stuff import show_instr


if TYPE_CHECKING:
    from tm.rules import Count

TRUNCATE_COUNT = 10 ** 12

def show_number(num: Count) -> str:
    return (
        str(num)
        if not isinstance(num, int) or num < TRUNCATE_COUNT else
        f"(~10^{math.log10(num):.0f})"
    )
