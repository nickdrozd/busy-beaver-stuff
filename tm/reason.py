# ruff: noqa: F401, I001
from tm.rust_stuff import (
    BackwardResult,  # noqa: TC001

    py_cant_halt as cant_halt,
    py_cant_blank as cant_blank,
    py_cant_spin_out as cant_spin_out,

    py_segment_cant_halt as segment_cant_halt,
    py_segment_cant_blank as segment_cant_blank,
    py_segment_cant_spin_out as segment_cant_spin_out,

    py_cps_cant_halt as cps_cant_halt,
    py_cps_cant_blank as cps_cant_blank,
    py_cps_cant_spin_out as cps_cant_spin_out,

    py_ctl_cant_halt as ctl_cant_halt,
    py_ctl_cant_blank as ctl_cant_blank,
    py_ctl_cant_spin_out as ctl_cant_spin_out,
)

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Callable

    type BackwardReasoner = Callable[
        [str, int],
        BackwardResult,
    ]
