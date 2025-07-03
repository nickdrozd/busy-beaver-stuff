# ruff: noqa: F401, I001
from tm.rust_stuff import (
    BackwardResult,  # noqa: TC001

    cant_halt,
    cant_blank,
    cant_spin_out,

    segment_cant_halt,
    segment_cant_blank,
    segment_cant_spin_out,

    cps_cant_halt,
    cps_cant_blank,
    cps_cant_spin_out,

    ctl_cant_halt,
    ctl_cant_blank,
    ctl_cant_spin_out,
)

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Callable

    type BackwardReasoner = Callable[
        [str, int],
        BackwardResult,
    ]
