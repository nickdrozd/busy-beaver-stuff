import os
import re
import json

from collections.abc import Callable, Iterator
from multiprocessing import cpu_count, Process

from tm.program import Program
from tm.machine import Machine


Prog = str

Output  = Callable[[Prog], None]


def tree_gen(
        steps: int,
        halt: bool,
        stack: list[Prog],
) -> Iterator[Prog]:
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
        )

        if machine.infrul:
            prog = None
            continue

        if any(blank < 10 for blank in machine.blanks.values()):
            prog = None
            continue

        if machine.xlimit:
            yield prog
            prog = None
            continue

        if machine.undfnd is None:
            if machine.rulapp:
                yield prog
            prog = None
            continue

        _, slot = machine.undfnd

        if len((program := Program(prog)).open_slots) == open_slot_lim:
            for ext in program.branch(slot):
                yield ext
            prog = None
            continue

        prog, *branches = program.branch(slot)

        for ext in branches:
            stack.append(ext)


def worker(
        steps: int,
        halt: bool,
        stack: list[Prog],
        output: Output,
) -> None:
    pid: int = os.getpid()

    def log(msg: str) -> None:
        print(f'{pid}: {msg}')

    log(json.dumps(stack, indent = 4))

    for prog in tree_gen(steps, halt, stack):
        try:
            output(prog)
        except Exception as err:  # pylint: disable = broad-exception-caught
            log(f'ERROR: {err}')

    log('done')


def run_tree_gen(
        states: int,
        colors: int,
        steps: int,
        halt: bool,
        output: Output,
) -> None:
    branches = [
        prog
        for prog in Program.branch_init(states, colors)
        if re.search('  .[^R]', prog)
    ]

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
