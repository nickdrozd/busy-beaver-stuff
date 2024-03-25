from __future__ import annotations

import os
import json
import signal
from itertools import product
from typing import TYPE_CHECKING

from multiprocessing import cpu_count, Process

from tm.show import show_instr
from tm.parse import parse, read_slot, init_prog
from tm.machine import quick_term_or_rec
from tm.rust_stuff import run_for_undefined, TreeSkip

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    from tm.parse import Color, State, Slot, Instr, Switch

    ProgStr = str

    Output = Callable[[ProgStr], None]

########################################

def tree_gen(
        steps: int,
        stack: list[ProgStr],
        open_slot_lim: int,
) -> Iterator[ProgStr]:
    while True:  # pylint: disable = while-used
        try:
            prog: ProgStr = stack.pop()
        except IndexError:
            break

        try:
            slot = run_for_undefined(prog, steps)
        except TreeSkip:
            continue

        if slot is None:
            yield prog
            continue

        branches = (program := Program(prog)).branch(slot)

        if len(program.open_slots) == open_slot_lim:
            yield from branches
        else:
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

    def handle_interrupt(_,__):# type: ignore[no-untyped-def] # no-cover
        log('interrupted...', dump_stack = True)

        raise KeyboardInterrupt

    signal.signal(
        signal.SIGINT,
        handle_interrupt)

    log('starting...', dump_stack = True)

    for prog in tree_gen(steps, stack, 2 if halt else 1):
        try:
            output(prog)
        # pylint: disable-next = broad-exception-caught
        except Exception as err:  # no-cover
            log(f'ERROR: {prog} || {err}')

    log('done')

########################################

class Program:
    prog: dict[State, Switch]

    states: set[State]
    colors: set[Color]

    max_state: State
    max_color: Color

    max_used_state: State
    max_used_color: Color

    def __init__(self, program: ProgStr):
        parsed = parse(program)

        self.prog = {
            state: dict(enumerate(instrs))
            for state, instrs in enumerate(parsed)
        }

        self.max_state = len(parsed) - 1
        self.max_color = len(parsed[0]) - 1

        max_used_color = 1
        max_used_state = 1

        for color, _, state in self.used_instructions:
            # pylint: disable = consider-using-max-builtin
            if color > max_used_color:
                max_used_color = color

            if state > max_used_state:
                max_used_state = state

        self.max_used_color = max_used_color
        self.max_used_state = max_used_state

        self._available_instrs = sorted(
            product(
                range(1 + max_used_color),
                (False, True),
                range(1 + max_used_state)))

    def __repr__(self) -> ProgStr:
        return '  '.join([
            ' '.join(
                show_instr(instr)
                for instr in instrs.values()
            )
            for instrs in self.prog.values()
        ])

    def __getitem__(self, slot: Slot) -> Instr:
        state, color = slot

        if (instr := self.prog[state][color]) is None:
            raise KeyError(slot)

        return instr

    def __setitem__(self, slot: Slot, instr: Instr | None) -> None:
        state, color = slot

        self.prog[state][color] = instr

    @property
    def instr_slots(self) -> list[tuple[Slot, Instr | None]]:
        return [
            ((state, color), instr)
            for state, instrs in self.prog.items()
            for color, instr in instrs.items()
        ]

    @property
    def used_instructions(self) -> Iterator[Instr]:
        return (
            instr
            for instrs in self.prog.values()
            for instr in instrs.values()
            if instr
        )

    @property
    def open_slots(self) -> tuple[Slot, ...]:
        return tuple(
            slot
            for slot, instr in self.instr_slots
            if instr is None
        )

    @property
    def available_instrs(self) -> list[Instr]:
        if self.max_used_state < self.max_state:
            self.max_used_state += 1

            self._available_instrs.extend(list(
                product(
                    range(1 + self.max_used_color),
                    (False, True),
                    (self.max_used_state,),
                )
            ))

            self._available_instrs.sort()

        elif self.max_used_color < self.max_color:
            self.max_used_color += 1

            self._available_instrs.extend(list(
                product(
                    (self.max_used_color,),
                    (False, True),
                    range(1 + self.max_used_state),
                )
            ))

            self._available_instrs.sort()

        return self._available_instrs

    def branch(self, slot: Slot) -> list[ProgStr]:
        branches = []

        try:
            orig = self[slot]
        except KeyError:
            orig = None

        for instr in self.available_instrs:
            if orig is not None and instr >= orig:
                continue

            self[slot] = instr

            branches.append(str(self))

        self[slot] = orig

        return branches

########################################

def branch_read(prog: str, slot: str) -> list[ProgStr]:
    return Program(prog).branch(
        read_slot(slot))


def init_branches(states: int, colors: int) -> list[str]:
    return branch_read(
        init_prog(states, colors),
        'B0')

########################################

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
