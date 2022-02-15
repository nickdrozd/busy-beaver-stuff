from typing import Optional

from tm.parse import parse
from tm.tape import BlockTape
from tm.machine import Machine

def run_bb(
        prog,
        tape = None,
        step_lim = None,
        skip: bool = True,
        sim_lim: int = 100_000_000,
        watch_tape: bool = False,
        check_rec: Optional[int] = None,
        check_blanks: bool = False,
        samples = None,
) -> Machine:
    machine = Machine(prog)
    machine.run(
        BlockTape([], 0, [], extend_to = tape),
        skip,
        step_lim,
        sim_lim,
        watch_tape,
        check_rec,
        check_blanks,
        samples,
    )

    return machine
