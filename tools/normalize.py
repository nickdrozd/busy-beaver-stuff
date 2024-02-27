from __future__ import annotations

from typing import TYPE_CHECKING

from tm.program import Program

if TYPE_CHECKING:
    from typing import Self

    from tm.parse import Color, State

class Normalizer(Program):
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
