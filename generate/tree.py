import os
import re
import json

from collections.abc import Callable, Iterator
from multiprocessing import cpu_count, Process

from tm.program import Program
from tm.machine import Machine
from tm.utils import run_variations


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
            prover = True,
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
            if machine.rulapp:  # no-coverage
                yield prog
            prog = None
            continue

        _, slot = machine.undfnd

        if len((program := Program(prog)).open_slots) == open_slot_lim:
            for ext in program.branch(slot):
                yield ext
            prog = None
            continue

        prog = next(branches := program.branch(slot, halt), None)

        for ext in branches:
            stack.append(ext)


def worker(
        steps: int,
        halt: bool,
        stack: list[Prog],
        output: Output,
) -> None:
    print(f'{os.getpid()}: {json.dumps(stack, indent = 4)}')

    for prog in tree_gen(steps, halt, stack):
        output(prog)

    print(f'{os.getpid()}: done')


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

########################################

def filter_run_print(halt: bool) -> Output:
    def cant_halt(prog: Prog) -> bool:
        return Program(prog).cant_halt

    def cant_spin_out(prog: Prog) -> bool:
        return Program(prog).cant_spin_out

    cant_reach = cant_halt if halt else cant_spin_out

    def drop(prog: Prog) -> None:
        if cant_reach(prog):
            return

        for machine in run_variations(prog, 1_000):
            if machine.simple_termination and machine.rulapp > 1_000:
                print(machine)
                return

            if machine.xlimit is None:
                return

        print(prog)

    return drop
