from queue import Empty
from multiprocessing import (
    cpu_count,
    Manager,
    Process,
)

from tm.run_bb import run_bb
from tm.program import Program


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
            check_blanks = True,
        )

        if machine.final.undfnd is not None:
            _step, instr = machine.final.undfnd

            for ext in Program(prog).branch(instr):
                progs.put(ext)

            continue

        if machine.final.xlimit is not None:
            output(prog)


def run_tree_gen(output=print):
    PROGS = Manager().Queue()
    PROGS.put('1RB ... ... ... ... ...')

    processes = [
        Process(target=tree_worker, args=(126, PROGS, output))
        for _ in range(cpu_count())
    ]

    for process in processes:
        process.start()

    for process in processes:
        process.join()
