from collections.abc import Iterator

from tm.macro import macro_variations
from tm.machine import Machine, LinRecMachine


def run_variations(
        prog: str,
        sim_lim: int,
        max_block: int = 1,
        back_wrap: int = 0,
        lin_rec: int = 50,
) -> Iterator[Machine]:
    yield LinRecMachine(prog).run(
        step_lim = lin_rec,
        check_rec = 0,
        skip = True,
    )

    for macro in macro_variations(prog, max_block, back_wrap):
        yield Machine(macro).run(
            sim_lim = sim_lim,
            prover = True,
        )
