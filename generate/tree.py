import re
from queue import Empty
from multiprocessing import (
    cpu_count,
    Manager,
    Process,
)
from typing import Callable

from tm import run_bb
# from py_lin_rado_turing.tools import run_bb
from generate.program import Program  # type: ignore


def tree_worker(steps: int, progs, output: Callable):
    while True:  # pylint: disable = while-used
        try:
            count, prog = progs.get(timeout = .5)
        except Empty:
            break

        machine = run_bb(
            prog,
            step_lim = steps,
            check_rec = 0,
            check_blanks = True,
            skip = False,  # !!! Macro skip bug !!!
        )

        if machine.final.xlimit is not None:
            output(prog)
            continue

        if machine.final.undfnd is None:
            continue

        _step, instr = machine.final.undfnd

        branches = Program(prog).branch(instr)

        for _ in range(count):
            _ = next(branches)

        try:
            ext = next(branches)
        except StopIteration:
            continue

        progs.put((0, ext))

        try:
            _ = next(branches)
        except StopIteration:
            pass
        else:
            progs.put((1 + count, prog))


DEFAULT_STEPS = {
    (2, 2): 40,
    (3, 2): 126,
    (2, 3): 223,  # 220
    (4, 2): 107,
}


def run_tree_gen(states: int, colors: int, output: Callable = print):
    progs = Manager().Queue()

    init_prog = re.sub(
        r'^\.\.\.',
        '1RB',
        '  '.join([
            ' '.join(
                ['...'] * colors)
        ] * states))

    progs.put((0, init_prog))

    processes = [
        Process(
            target = tree_worker,
            args = (
                DEFAULT_STEPS[
                    (states, colors)],
                progs,
                output,
            )
        )
        for _ in range(cpu_count())
    ]

    for process in processes:
        process.start()

    for process in processes:
        process.join()
