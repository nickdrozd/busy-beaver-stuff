from __future__ import annotations

import math
from typing import TYPE_CHECKING

from tm.rust_stuff import show_state

if TYPE_CHECKING:
    from tm.parse import Slot


TRUNCATE_COUNT = 10 ** 12

def show_number(num: int) -> str:
    return (
        str(num)
        if num < TRUNCATE_COUNT else
        f"(~10^{math.log10(num):.0f})"
    )


def show_slot(slot: Slot) -> str:
    state, color = slot

    return show_state(state) + str(color)
