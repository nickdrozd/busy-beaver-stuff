from collections.abc import Iterator

from tm.graph import Graph
from tm.program import Program
from tm.machine import Machine, LinRecMachine
from tm.macro import BlockMacro, BacksymbolMacro, macro_variations


def run_variations(
        prog: str,
        sim_lim: int,
        depth: int,
        max_block: int = 1,
        back_wrap: int = 0,
) -> Iterator[bool]:
    yield LinRecMachine(prog).run(
        step_lim = 50,
        check_rec = 0,
        skip = True,
    ).xlimit is None

    yield from (
        Machine(macro).run(
            sim_lim = sim_lim,
            prover = depth,
        ).xlimit is None
        for macro in macro_variations(
                prog, max_block, back_wrap)
    )


__all__ = [
    'Graph',
    'Program',
    'Machine',
    'BlockMacro',
    'LinRecMachine',
    'BacksymbolMacro',
]
