from __future__ import annotations

from typing import TYPE_CHECKING
from itertools import product

from tm.show import show_instr
from tm.parse import parse, read_slot, init_prog

if TYPE_CHECKING:
    from collections.abc import Iterator

    from tm.parse import Color, State, Slot, Instr, Switch

    ProgStr = str


def init_branches(states: int, colors: int) -> list[str]:
    return Program(init_prog(states, colors)).branch_read('B0')


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

        self.states = set(range(len(parsed)))
        self.colors = set(range(len(parsed[0])))

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

    def available_states(self, used: set[State]) -> set[State]:
        diff = sorted(self.states.difference(used))

        return used | { diff[0] } if diff else used

    def available_colors(self, used: set[Color]) -> set[Color]:
        diff = sorted(self.colors.difference(used))

        return used | { diff[0] } if diff else used

    @property
    def available_instrs(self) -> list[Instr]:
        used_colors = { 0 }
        used_states = { 0 }

        for color, _, state in self.used_instructions:
            used_colors.add(color)
            used_states.add(state)

        return sorted(
            product(
                self.available_colors(used_colors),
                (False, True),
                self.available_states(used_states)),
            reverse = True,
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

    def branch_read(self, slot: str) -> list[ProgStr]:
        return self.branch(read_slot(slot))
