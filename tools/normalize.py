from __future__ import annotations

from typing import TYPE_CHECKING

from tm.parse import parse
from tm.show import show_instr

if TYPE_CHECKING:
    from typing import Self
    from collections.abc import Iterator

    from tm.parse import Color, State, Slot, Instr, Switch


def normalize(prog: str) -> str:
    return str(Normalizer(prog).normalize())


class Normalizer:
    prog: dict[State, Switch]

    def __init__(self, program: str):
        parsed = parse(program)

        self.prog = {
            state: dict(enumerate(instrs))
            for state, instrs in enumerate(parsed)
        }

    @property
    def states(self) -> set[State]:
        return set(range(len(self.prog)))

    @property
    def colors(self) -> set[Color]:
        return set(range(len(self.prog[0])))

    def __repr__(self) -> str:
        return '  '.join([
            ' '.join(
                show_instr(instr)
                for instr in instrs.values()
            )
            for instrs in self.prog.values()
        ])

    def __getitem__(self, slot: Slot) -> Instr:
        state, color = slot

        if (instr := self.prog[state][color]) is None:  # no-cover
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
    def used_instr_slots(self) -> list[tuple[Slot, Instr]]:
        return [
            (slot, instr)
            for slot, instr in self.instr_slots
            if instr is not None
        ]

    @property
    def used_instructions(self) -> Iterator[Instr]:
        return (
            instr
            for instrs in self.prog.values()
            for instr in instrs.values()
            if instr
        )

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
        for state in self.states:
            switch = self.prog[state]
            switch[co1], switch[co2] = switch[co2], switch[co1]

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

    def cinch(self) -> None:
        for state in self.states:  # no-branch
            if (instr := self.prog[state][0]) is None or instr[0] == 0:
                continue

            if state != 0:
                self.swap_states(0, state)

            return

    def cut_unused_states(self) -> None:
        states_to_del = {
            state
            for state, switch in self.prog.items()
            if all(switch[color] is None for color in self.colors)
        }

        for state in states_to_del:
            del self.prog[state]

    def normalize(self) -> Self:
        for _ in self.colors:
            self.normalize_states()
            self.normalize_colors()
            self.cut_unused_states()
            self.cinch()

        return self.normalize_directions()
