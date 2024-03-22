from __future__ import annotations

from typing import TYPE_CHECKING
from itertools import product

from tm.show import show_instr
from tm.parse import parse, read_slot, init_prog

if TYPE_CHECKING:
    from collections.abc import Iterator

    from tm.parse import Color, State, Slot, Instr, Switch

    ProgStr = str

########################################

def branch_read(prog: str, slot: str) -> list[ProgStr]:
    return Program(prog).branch(
        read_slot(slot))


def init_branches(states: int, colors: int) -> list[str]:
    return branch_read(
        init_prog(states, colors),
        'B0')

########################################

class Program:
    prog: dict[State, Switch]

    states: set[State]
    colors: set[Color]

    def __init__(self, program: ProgStr):
        parsed = parse(program)

        self.prog = {
            state: dict(enumerate(instrs))
            for state, instrs in enumerate(parsed)
        }

        self.max_state = len(parsed) - 1
        self.max_color = len(parsed[0]) - 1

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
    def last_slot(self) -> Slot | None:
        if len((slots := self.open_slots)) != 1:
            return None

        return slots[0]

    @property
    def available_instrs(self) -> list[Instr]:
        max_used_color = 1
        max_used_state = 1

        for color, _, state in self.used_instructions:
            # pylint: disable = consider-using-max-builtin
            if color > max_used_color:
                max_used_color = color

            if state > max_used_state:
                max_used_state = state

        if max_used_color < self.max_color:
            max_used_color += 1

        if max_used_state < self.max_state:
            max_used_state += 1

        return sorted(
            product(
                range(1 + max_used_color),
                (False, True),
                range(1 + max_used_state)),
        )

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
