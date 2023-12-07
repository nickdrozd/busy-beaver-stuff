# pylint: skip-file

from __future__ import annotations

import sys

from typing import TYPE_CHECKING

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
