from __future__ import annotations

import re
from typing import TYPE_CHECKING
from itertools import product
from functools import cached_property

from tm.graph import Graph
from tm.show import show_instr
from tm.parse import parse, read_slot

if TYPE_CHECKING:
    from typing import Self

    from tm.parse import Color, State, Slot, Instr

    # ruff: noqa: F401
    from tm.parse import Shift

    ProgStr = str

    Switch = dict[Color, Instr | None]


class Program:
    prog: dict[State, Switch]

    graph: Graph

    def __init__(self, program: ProgStr):
        self.prog = {
            state: dict(enumerate(instrs))
            for state, instrs in enumerate(parse(program))
        }

        self.graph = Graph(program)

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
    def state_switches(self) -> list[tuple[State, Switch]]:
        return sorted(self.prog.items())

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
    def halt_slots(self) -> tuple[Slot, ...]:
        return tuple(
            slot
            for slot, instr in self.instr_slots
            if instr is None or instr[2] == -1
        )

    @property
    def erase_slots(self) -> tuple[Slot, ...]:
        return tuple(
            slot
            for slot, instr in self.used_instr_slots
            if slot[1] != 0 and instr[0] == 0
        )

    @property
    def spinout_slots(self) -> tuple[Slot, ...]:
        return tuple(
            (state, 0)
            for state in self.graph.zero_reflexive_states
        )

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

    def swap_states(self, st1: State, st2: State) -> Self:
        self.prog[st1], self.prog[st2] = self.prog[st2], self.prog[st1]

        for slot, (color, shift, state) in self.used_instr_slots:
            self[slot] = color, shift, (
                st1 if state == st2 else
                st2 if state == st1 else
                state
            )

        return self

    def swap_colors(self, co1: Color, co2: Color) -> Self:
        for state_str in self.states:
            st_key = self.get_switch(state_str)
            st_key[co1], st_key[co2] = st_key[co2], st_key[co1]

        for slot, (color, shift, state) in self.used_instr_slots:
            self[slot] = (
                co1 if color == co2 else
                co2 if color == co1 else
                color
            ), shift, state

        return self

    def normalize_states(self) -> Self:
        for _ in self.states:  # no-branch
            todo = sorted(self.states)[1:]

            for _, _, state in self.used_instructions:
                if state not in todo:
                    continue

                norm, *rest = todo

                if state != norm:
                    self.swap_states(state, norm)
                    break

                todo = rest
            else:
                break

        return self

    def normalize_colors(self) -> Self:
        for _ in self.colors:  # no-branch
            todo = sorted(self.colors)[1:]

            for color, _, _ in self.used_instructions:
                if color not in todo:
                    continue

                norm, *rest = todo

                if color != norm:
                    self.swap_colors(color, norm)

                todo = rest
            else:
                break

        return self

    def normalize_directions(self) -> Self:
        if (index := self[0, 0]) is None or index[1]:
            return self

        for slot, (color, shift, state) in self.used_instr_slots:
            self[slot] = color, not shift, state

        return self

    def normalize(self) -> Self:
        for _ in self.colors:
            self.normalize_states()
            self.normalize_colors()

        return self.normalize_directions()
