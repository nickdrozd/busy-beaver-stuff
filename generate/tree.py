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
    prog = None

    while True:  # pylint: disable = while-used
        if prog is None:
            try:
                count, prog = progs.get(timeout = .5)
            except Empty:
                break

        machine = run_bb(
            prog,
            sim_lim = steps,
            check_blanks = True,
        )

        if machine.final.xlimit is not None:
            check_rec = run_bb(
                prog,
                sim_lim = steps,
                check_rec = 0,
                skip = False,  # !!! Macro skip bug !!!
            )

            if check_rec.final.xlimit is not None:
                output(prog)

            prog = None

            continue

        if machine.final.undfnd is None:
            prog = None
            continue

        _step, instr = machine.final.undfnd

        branches = Program(prog).branch(instr)

        for _ in range(count):
            _ = next(branches)

        try:
            ext = next(branches)
        except StopIteration:
            prog = None
            continue

        try:
            _ = next(branches)
        except StopIteration:
            pass
        else:
            progs.put((1 + count, prog))

        count, prog = 0, ext


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
