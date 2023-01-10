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
        lin_rec: int = 50,
) -> Iterator[bool]:
    yield LinRecMachine(prog).run(
        step_lim = lin_rec,
        check_rec = 0,
        skip = True,
    ).xlimit is None

    for macro in macro_variations(prog, max_block, back_wrap):
        yield Machine(macro).run(
            sim_lim = sim_lim,
            prover = depth,
        ).xlimit is None


__all__ = [
    'Graph',
    'Program',
    'Machine',
    'BlockMacro',
    'LinRecMachine',
    'BacksymbolMacro',
]
