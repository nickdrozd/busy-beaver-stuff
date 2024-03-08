from __future__ import annotations

import os
import json
import signal
from typing import TYPE_CHECKING

from multiprocessing import cpu_count, Process

from tm.program import Program, init_branches
from tm.machine import quick_term_or_rec
from tm.rust_stuff import run_for_undefined, TreeSkip

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    ProgStr = str

    Output = Callable[[ProgStr], None]


def tree_gen(
        steps: int,
        halt: bool,
        stack: list[ProgStr],
) -> Iterator[ProgStr]:
    prog: ProgStr | None = None

    open_slot_lim = 2 if halt else 1

    while True:  # pylint: disable = while-used
        if prog is None:
            try:
                prog = stack.pop()
            except IndexError:
                break

        try:
            slot = run_for_undefined(prog, steps)
        except TreeSkip:
            prog = None
            continue

        if slot is None:
            yield prog
            prog = None
            continue

        if len((program := Program(prog)).open_slots) == open_slot_lim:
            yield from program.branch(slot)
            prog = None
            continue

        prog, *branches = program.branch(slot)

        stack.extend(branches)


def worker(
        steps: int,
        halt: bool,
        stack: list[ProgStr],
        output: Output,
        prep: bool = False,
) -> None:
    pid: str = 'prep' if prep else str(os.getpid())

    def log(msg: str, dump_stack: bool = False) -> None:
        msg = f'{pid}: {msg}'

        if dump_stack:
            msg = '\n'.join([
                msg,
                json.dumps(
                    stack,
                    indent = 4,
                )
            ])

        print(msg)

    def handle_interrupt(_, __) -> None:  # type: ignore[no-untyped-def]
        log('interrupted...', dump_stack = True)

        raise KeyboardInterrupt

    signal.signal(
        signal.SIGINT,
        handle_interrupt)

    log('starting...', dump_stack = True)

    for prog in tree_gen(steps, halt, stack):
        try:
            output(prog)
        except Exception as err:  # pylint: disable = broad-exception-caught
            log(f'ERROR: {prog} || {err}')

    log('done')


def prep_branches(
        states: int,
        colors: int,
        halt: bool,
) -> list[ProgStr]:
    branches = []

    def run(prog: ProgStr) -> None:
        if quick_term_or_rec(prog, 10):
            return

        branches.append(prog)

    worker(
        steps = 3,
        halt = halt,
        stack = init_branches(states, colors),
        output = run,
        prep = True,
    )

    return sorted(branches)


def distribute_branches(branches: list[ProgStr]) -> list[list[ProgStr]]:
    cpus = cpu_count()

    branch_groups: list[list[ProgStr]] = [[] for _ in range(cpus)]

    branch_counts = [0] * cpus

    for branch in branches:
        min_index = branch_counts.index(min(branch_counts))
        branch_groups[min_index].append(branch)
        branch_counts[min_index] += branch.count('...')

    for group in branch_groups:
        group.sort(key = lambda x: x.count('...'), reverse = True)

    return branch_groups


def run_tree_gen(
        steps: int,
        halt: bool,
        output: Output,
        branches: list[ProgStr] | None = None,
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
