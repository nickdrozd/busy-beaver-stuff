import os
import json

from collections.abc import Callable
from multiprocessing import cpu_count, Process

from tm.program import Program
from tm.machine import Machine


Prog = str

Output  = Callable[[Prog], None]


def worker(
        steps: int,
        halt: bool,
        stack: list[Prog],
        output: Output,
) -> None:
    print(f'{os.getpid()}: {json.dumps(stack, indent = 4)}')

    prog: Prog | None = None

    open_slot_lim = 2 if halt else 1

    while True:  # pylint: disable = while-used
        if prog is None:
            try:
                prog = stack.pop()
            except IndexError:
                break

        machine = Machine(prog).run(
            sim_lim = steps,
            prover = True,
        )

        if machine.infrul:
            prog = None
            continue

        if any(blank < 10 for blank in machine.blanks.values()):
            prog = None
            continue

        if machine.xlimit:
            output(prog)
            prog = None
            continue

        if machine.undfnd is None:
            if machine.rulapp:  # no-coverage
                output(prog)
            prog = None
            continue

        _, slot = machine.undfnd

        if len((program := Program(prog)).open_slots) == open_slot_lim:
            for ext in program.branch(slot):
                output(ext)
            prog = None
            continue

        prog = next(branches := program.branch(slot, halt))

        for ext in branches:
            stack.append(ext)

    print(f'{os.getpid()}: done')


def run_tree_gen(
        states: int,
        colors: int,
        steps: int,
        halt: bool,
        output: Output,
) -> None:
    branches = Program.branch_init(states, colors)

    cpus = cpu_count()

    chunk = (len(branches) + cpus - 1) // cpus

    branch_groups = [
        branches[ i : i + chunk ]
        for i in range(0, len(branches), chunk)
     ]

    processes = [
         Process(
            target = worker,
            args = (steps, halt, group, output),
         )
        for group in branch_groups
     ]

    for process in processes:
        process.start()

    for process in processes:
        process.join()
