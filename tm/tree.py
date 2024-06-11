from __future__ import annotations

from typing import TYPE_CHECKING
from multiprocessing import Pool

from tm.rust_stuff import tree_progs

if TYPE_CHECKING:
    from collections.abc import Callable
    from tm.parse import Params


def run_tree_gen(
        steps: int,
        halt: bool,
        params: Params,
        output: Callable[[str], None],
) -> None:
    with Pool() as pool:
        pool.map(
            output,
            tree_progs(
                params, halt, steps))
