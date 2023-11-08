from __future__ import annotations

import os
import json
from typing import TYPE_CHECKING

from multiprocessing import cpu_count, Process

from tm.program import Program
from tm.machine import Machine, LinRecMachine

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

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

        if machine.undfnd is None:
            if machine.xlimit or machine.rulapp:
                yield prog
            prog = None
            continue

        _, slot = machine.undfnd

        if len((program := Program(prog)).open_slots) == open_slot_lim:
            yield from program.branch(slot)
            prog = None
            continue

        prog, *branches = program.branch(slot)

        stack.extend(branches)


def worker(
        steps: int,
        halt: bool,
        stack: list[Prog],
        output: Output,
        prep: bool = False,
) -> None:
    pid: str = 'prep' if prep else str(os.getpid())

    def log(msg: str) -> None:
        print(f'{pid}: {msg}')

    def log_stack() -> None:
        log(json.dumps(stack, indent = 4))

    log_stack()

    for prog in tree_gen(steps, halt, stack):
        try:
            output(prog)
        except KeyboardInterrupt:
            log('dumping...')
            log_stack()
            raise
        except Exception as err:  # pylint: disable = broad-exception-caught
            log(f'ERROR: {prog} || {err}')

    log('done')


def prep_branches(
        states: int,
        colors: int,
        halt: bool,
) -> list[Prog]:
    branches = []

    def run(prog: Prog) -> None:
        if LinRecMachine(prog).run(5).linrec:
            return

        branches.append(prog)

    worker(
        steps = 3,
        halt = halt,
        stack = Program.branch_init(states, colors),
        output = run,
        prep = True,
    )

    return sorted(branches)


def distribute_branches(branches: list[Prog]) -> list[list[Prog]]:
    cpus = cpu_count()

    size, rem = divmod(len(branches), cpus)

    start, result = 0, []

    for i in range(cpus):
        end = start + size + (1 if i < rem else 0)
        result.append(branches[start:end])
        start = end

    return result


def run_tree_gen(
        steps: int,
        halt: bool,
        output: Output,
        branches: list[Prog] | None = None,
        states: int | None = None,
        colors: int | None = None,
) -> None:
    if branches is None:
        assert states is not None
        assert colors is not None

        branches = prep_branches(states, colors, halt)

    processes = [
        Process(
            target = worker,
            args = (steps, halt, group, output))
        for group in distribute_branches(branches)
     ]

    for process in processes:
        process.start()

    for process in processes:
        process.join()
