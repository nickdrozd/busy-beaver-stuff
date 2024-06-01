from __future__ import annotations

import os
import json
import signal
from itertools import product
from typing import TYPE_CHECKING
from collections import defaultdict

from multiprocessing import cpu_count, Process

from tm.show import show_comp
from tm.parse import init_prog, tcompile
from tm.machine import quick_term_or_rec
from tm.rust_stuff import run_for_undefined, TreeSkip

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    from tm.parse import Params, Slot, Instr, CompProg

    Output = Callable[[str], None]

########################################

class Stack:
    progs: dict[int, list[str]]

    def __init__(self, progs: list[str]):
        self.progs = defaultdict(list)
        self.extend(progs)

    def extend(self, progs: list[str]) -> None:
        for prog in progs:
            self.progs[prog.count('...')].append(prog)

    def pop(self) -> str:
        for _, progs in sorted(self.progs.items()):
            if not progs:
                continue

            return progs.pop()

        raise IndexError


def tree_gen(
        steps: int,
        stack: Stack,
        params: Params,
        open_slot_lim: int,
) -> Iterator[str]:
    while True:  # pylint: disable = while-used
        try:
            prog: str = stack.pop()
        except IndexError:
            break

        try:
            slot = run_for_undefined(prog, steps)
        except TreeSkip:
            continue

        if slot is None:
            yield prog
            continue

        branches = (program := Program(params, prog)).branch(slot)

        if program.open_slot_count == open_slot_lim:
            yield from branches
        else:
            stack.extend(branches)


def worker(
        steps: int,
        halt: bool,
        stack: list[str],
        params: Params,
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

    def handle_interrupt(_,__):# type: ignore[no-untyped-def] # no-cover
        log('interrupted...', dump_stack = True)

        raise KeyboardInterrupt

    signal.signal(
        signal.SIGINT,
        handle_interrupt)

    log('starting...', dump_stack = True)

    for prog in tree_gen(steps, Stack(stack), params, 2 if halt else 1):
        try:
            output(prog)
        # pylint: disable-next = broad-exception-caught
        except Exception as err:  # no-cover
            log(f'ERROR: {prog} || {err}')

    log('done')

########################################

class Program:
    prog: CompProg

    states: int
    colors: int

    avail_states: int
    avail_colors: int

    def __init__(self, params: Params, program: str):
        self.prog = tcompile(program)

        self.states, self.colors = params

        max_used_color = 1
        max_used_state = 1

        for color, _, state in self.prog.values():
            # pylint: disable = consider-using-max-builtin
            if color > max_used_color:
                max_used_color = color

            if state > max_used_state:
                max_used_state = state

        self.avail_states = min(self.states, 2 + max_used_state)
        self.avail_colors = min(self.colors, 2 + max_used_color)

    def __repr__(self) -> str:
        return show_comp(self.prog, (self.states, self.colors))

    @property
    def open_slot_count(self) -> int:
        return (self.states * self.colors) - len(self.prog)

    @property
    def open_slots(self) -> list[Slot]:
        return [
            slot
            for slot in product(range(self.states), range(self.colors))
            if slot not in self.prog
        ]

    @property
    def available_instrs(self) -> list[Instr]:
        return sorted(
            product(
                range(self.avail_colors),
                (False, True),
                range(self.avail_states)))

    def branch(self, slot: Slot) -> list[str]:
        branches = []

        for instr in self.available_instrs:
            self.prog[slot] = instr

            branches.append(str(self))

        del self.prog[slot]

        return branches

########################################

def init_branches(params: Params) -> list[str]:
    return Program(
        params,
        init_prog(*params),
    ).branch((1, 0))  # B0


def prep_branches(params: Params, halt: bool) -> list[str]:
    branches = []

    def run(prog: str) -> None:
        if quick_term_or_rec(prog, 10):
            return

        branches.append(prog)

    worker(
        steps = 3,
        halt = halt,
        stack = init_branches(params),
        params = params,
        output = run,
        prep = True,
    )

    return sorted(branches)


def distribute(
        cpus: int,
        branches: list[str],
) -> list[list[str]]:
    branch_groups: list[list[str]] = [[] for _ in range(cpus)]

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
        params: Params,
        output: Output,
        branches: list[str] | None = None,
) -> None:
    if branches is None:
        branches = prep_branches(params, halt)

    processes = [
        Process(
            target = worker,
            args = (steps, halt, group, params, output))
        for group in distribute(cpu_count(), branches)
     ]

    for process in processes:
        process.start()

    for process in processes:
        process.join()
