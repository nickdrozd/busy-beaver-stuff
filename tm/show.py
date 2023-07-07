import math

TRUNCATE_COUNT = 10 ** 12

def show_number(num: int) -> str:
    return (
        str(num)
        if num < TRUNCATE_COUNT else
        f"(~10^{math.log10(num):.0f})"
    )


# pylint: disable = unused-import, wrong-import-position
from tm.rust_stuff import show_state
