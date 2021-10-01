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
    2: 40,
    3: 126,
    4: 107,
}


def run_tree_gen(states, output=print):
    progs = Manager().Queue()

    init_prog = '1RB' + ' ...' * (states * 2 - 1)

    progs.put(init_prog)

    processes = [
        Process(
            target = tree_worker,
            args = (
                DEFAULT_STEPS[states],
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
