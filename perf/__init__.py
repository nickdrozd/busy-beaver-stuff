# pylint: skip-file

from __future__ import annotations

import sys

from functools import reduce
from typing import TYPE_CHECKING

from test.utils import read_holdouts  # noqa: F401

if TYPE_CHECKING:
    from collections.abc import Callable

    Null = Callable[[], None]


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


def get_holdouts() -> list[str]:
    return sorted(
        reduce(
            lambda acc, cat: acc | read_holdouts(cat), # type: ignore
            ('32q', '23q', '42h', '42q', '24h'),
            set(),
        )
    )
