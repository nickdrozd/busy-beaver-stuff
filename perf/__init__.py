# pylint: skip-file

from __future__ import annotations

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
        stats.save('yappi.callgrind', type = 'callgrind')

    return wrapper
