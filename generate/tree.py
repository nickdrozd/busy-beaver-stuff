import re
from queue import Empty
from multiprocessing import (
    cpu_count,
    Manager,
    Process,
)

from tm import run_bb
from generate.program import Program


def tree_worker(steps, progs, output):
    while True:  # pylint: disable = while-used
        try:
            prog = progs.get(timeout=.5)
        except Empty:
            break

        machine = run_bb(
            prog,
            x_limit = steps,
            check_rec = 0,
        )

        if machine.final.undfnd is not None:
            _step, instr = machine.final.undfnd

            for ext in Program(prog).branch(instr):
                progs.put(ext)

            continue

        if machine.final.xlimit is not None:
            output(prog)


DEFAULT_STEPS = {
    (2, 2): 40,
    (3, 2): 126,
    (2, 3): 223,  # 220
    (4, 2): 107,
}


def run_tree_gen(states, colors, output=print):
    progs = Manager().Queue()

    progs.put(
        re.sub(
            r'^\.\.\.',
            '1RB',
            '  '.join([
                ' '.join(
                    ['...'] * colors)
            ] * states)))

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
