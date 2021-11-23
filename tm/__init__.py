from typing import Optional

from tm.parse import parse
from tm.machine import Machine

def run_bb(
        prog,
        tape = None,
        x_limit: int = 100_000_000,
        watch_tape: bool = False,
        check_rec: Optional[int] = None,
        check_blanks: bool = False,
        samples = None,
) -> Machine:
    if tape is None:
        tape = [], 0, []
    elif isinstance(tape, int):
        tape = [0] * (tape // 2), 0, [0] * (tape // 2)

    machine = Machine(prog)
    machine.run(
        tape,
        x_limit,
        watch_tape,
        check_rec,
        check_blanks,
        samples,
    )

    return machine
