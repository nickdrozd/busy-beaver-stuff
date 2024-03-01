from __future__ import annotations

import re
from typing import TYPE_CHECKING
from itertools import product
from functools import cached_property

from tm.show import show_instr
from tm.parse import parse, read_slot

if TYPE_CHECKING:
    from typing import Self

    from tm.parse import Color, State, Slot, Instr, Switch

    ProgStr = str


class Program:
    prog: dict[State, Switch]

    def __init__(self, program: ProgStr):
        self.prog = {
            state: dict(enumerate(instrs))
            for state, instrs in enumerate(parse(program))
        }

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

    def get_switch(self, state: State) -> Switch:
        return self.prog[state]

    def __setitem__(self, slot: Slot, instr: Instr | None) -> None:
        state, color = slot

        self.prog[state][color] = instr

    def __eq__(self, other: object) -> bool:
        return str(self) == str(other)

    def __hash__(self) -> int:
        return hash(str(self))

    @classmethod
    def init(cls, states: int, colors: int) -> Self:
        return cls(
            re.sub(
                r'^\.\.\.',
                '1RB',
                '  '.join([
                    ' '.join(
                        ['...'] * colors)
                ] * states)))

    @classmethod
    def branch_init(cls, states: int, colors: int) -> list[str]:
        return cls.init(states, colors).branch_read('B0')

    @cached_property
    def states(self) -> set[State]:
        return set(self.prog.keys())

    @cached_property
    def colors(self) -> set[Color]:
        return set(range(len(self.prog[0])))

    @property
    def instr_slots(self) -> list[tuple[Slot, Instr | None]]:
        return [
            ((state, color), instr)
            for state, instrs in self.prog.items()
            for color, instr in instrs.items()
        ]

    @property
    def used_instr_slots(self) -> list[tuple[Slot, Instr]]:
        return [
            (slot, instr)
            for slot, instr in self.instr_slots
            if instr is not None
        ]

    @property
    def instructions(self) -> list[Instr | None]:
        return [
            instr
            for instrs in self.prog.values()
            for instr in instrs.values()
        ]

    @property
    def used_instructions(self) -> list[Instr]:
        return [
            instr
            for instr in self.instructions
            if instr
        ]

    @property
    def slots(self) -> tuple[Slot, ...]:
        return tuple(slot for slot, _ in self.instr_slots)

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
    def used_states(self) -> set[State]:
        return {
            state
            for _, _, state in self.used_instructions
        }

    @property
    def available_states(self) -> set[State]:
        used = self.used_states | { 0 }
        diff = sorted(self.states.difference(used))

        return used | { diff[0] } if diff else used

    @property
    def used_colors(self) -> set[Color]:
        return {
            color
            for color, _, _ in self.used_instructions
        }

    @property
    def available_colors(self) -> set[Color]:
        used = self.used_colors | { 0 }
        diff = sorted(self.colors.difference(used))

        return used | { diff[0] } if diff else used

    @property
    def available_instrs(self) -> list[Instr]:
        return sorted(
            product(
                self.available_colors,
                (False, True),
                self.available_states),
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
