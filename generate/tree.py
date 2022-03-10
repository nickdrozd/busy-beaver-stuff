import re
from queue import Empty
from multiprocessing import (
    cpu_count,
    Manager,
    Process,
)
from typing import Callable

from tm import Machine
from generate import Program  # type: ignore


def tree_worker(steps: int, progs, halt: bool, output: Callable):
    prog = None

    while True:  # pylint: disable = while-used
        if prog is None:
            try:
                count, prog = progs.get(timeout = .5)
            except Empty:
                break

        machine = Machine(prog).run(
            sim_lim = steps,
            check_blanks = True,
        )

        if machine.final.xlimit is not None:
            output(prog)
            prog = None
            continue

        if machine.final.undfnd is None:
            prog = None
            continue

        _step, instr = machine.final.undfnd

        branches = Program(prog).branch(instr, halt = halt)

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
}


def run_tree_gen(
        states: int,
        colors: int,
        halt: bool = False,
        output: Callable = print):
    progs = Manager().Queue()

    init_prog = re.sub(
        r'^\.\.\.',
        '1RB',
        '  '.join([
            ' '.join(
                ['...'] * colors)
        ] * states))

    progs.put((0, init_prog))

    try:
        steps = DEFAULT_STEPS[(states, colors)]
    except KeyError:
        steps = 500

    processes = [
        Process(
            target = tree_worker,
            args = (steps, progs, halt, output)
        )
        for _ in range(cpu_count())
    ]

    for process in processes:
        process.start()

    for process in processes:
        process.join()
