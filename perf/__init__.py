# pylint: skip-file

from __future__ import annotations

import sys

from typing import TYPE_CHECKING

from test.utils import read_progs

if TYPE_CHECKING:
    from collections.abc import Callable

    Null = Callable[[], None]


HOLDOUTS = (
    read_progs('holdouts_32q')
    | read_progs('holdouts_23q')
    | read_progs('holdouts_42h')
    | read_progs('holdouts_42q')
    | read_progs('holdouts_24h')
)


def profile(function: Null) -> Null:
    def wrapper() -> None:
        import yappi  # type: ignore

        yappi.set_clock_type('cpu')
        yappi.start()

        function()

        stats = yappi.get_func_stats()

        stats.save(
            f"{sys.argv[0].split('.')[0]}.callgrind",
            type = 'callgrind')

    return wrapper
