from __future__ import annotations

import re
from typing import TYPE_CHECKING
from itertools import product
from functools import cached_property

from tm.graph import Graph
from tm.show import show_instr
from tm.parse import parse, read_slot

from tm.rust_stuff import Program as ProgramRust

if TYPE_CHECKING:
    from typing import Self

    from tm.parse import Color, Shift, State, Slot, Instr

    ProgStr = str

    Switch = dict[Color, Instr | None]


class Program(ProgramRust):
    prog: dict[State, Switch]

    graph: Graph

    def __repr__(self) -> ProgStr:
        return '  '.join([
            ' '.join(
                show_instr(instr)
                for instr in instrs.values()
            )
            for instrs in self.prog.values()
        ])

    def __len__(self) -> int:
        return len(self.states) * len(self.colors)

    def __getitem__(self, slot: Slot) -> Instr | None:
        state, color = slot

        return self.prog[state][color]

    def get_switch(self, state: State) -> Switch:
        return self.prog[state]

    def __eq__(self, other: object) -> bool:
        return str(self) == str(other)

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

        return sorted(
            product(
                self.available_colors,
                (False, True),
                self.available_states),
            reverse = True,
        )

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
