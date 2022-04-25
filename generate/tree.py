from queue import Empty
from multiprocessing import (
    cpu_count,
    Manager,
    Process,
)
from typing import Callable

from tm import Machine
from generate import Program


def tree_worker(steps: int, progs, halt: bool, output: Callable):
    prog = None

    while True:  # pylint: disable = while-used
        if prog is None:
            try:
                instr, prog = progs.get(timeout = .5)
            except Empty:
                break

        try:
            prog = next(Program(prog).branch(instr, halt = halt))
        except StopIteration:
            prog = None
            continue

        progs.put((instr, prog))

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

    progs.put(('B0', str(Program.empty(states, colors))))

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
