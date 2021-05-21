from queue import Empty
from multiprocessing import (
    cpu_count,
    Manager,
    Process,
)

from turing import run_bb
from tm.program import Program


def tree_gen(steps, progs):
    while True:
        try:
            prog = progs.get(timeout=.5)
        except Empty:
            break

        program = Program(prog)

        machine = run_bb(
            program,
            x_limit = steps,
            check_rec = 0,
            check_blanks = True,
        )

        status, _step, instr = machine.final

        if status != 'UNDFND':
            if status == 'XLIMIT':
                print(prog)

            continue

        for ext in program.branch(instr):
            progs.put(ext)


if __name__ == '__main__':
    progs = Manager().Queue()
    progs.put('1RB ... ... ... ... ...')

    processes = [
        Process(target=tree_gen, args=(126, progs))
        for _ in range(cpu_count())
    ]

    for process in processes:
        process.start()

    for process in processes:
        process.join()
