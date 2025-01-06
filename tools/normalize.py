from __future__ import annotations

from typing import TYPE_CHECKING

from tm.show import show_instr
from tools import parse

if TYPE_CHECKING:
    from collections.abc import Iterator
    from typing import Self

    from tm.parse import Color, Instr, Slot, State
    from tools import Switch


def expand(prog: str) -> str:
    assert ' ' not in prog

    return '  '.join([
        ' '.join([
            '...'
            if 'Z' in (instr := state[i:i+3]) or '-' in instr else
            instr
            for i in range(0, len(state), 3)
         ])
        for state in prog.strip().split('_')
    ])


def init_prog(states: int, colors: int) -> str:
    prog = [
        ["..." for _ in range(colors)]
        for _ in range(states)
    ]

    prog[0][0] = "1RB"

    return '  '.join(' '.join(state) for state in prog)


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

    @classmethod
    def init(cls, states: int, colors: int) -> Self:
        return cls(init_prog(states, colors))

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
            else:  # noqa: PLW0120
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
                and all(instr is None or instr[2] != state
                      for sw in self.prog.values()
                      for instr in sw.values())
        }

        for state in states_to_del:
            del self.prog[state]

        for state in sorted(self.states, reverse = True)[:-1]:
            if not any(state in instr
                       for instr in self.used_instructions):
                del self.prog[state]

    def cut_unused_colors(self) -> None:
        for color in self.colors:
            if any(sw[color] is not None for sw in self.prog.values()):
                continue

            if any(instr is not None and instr[0] == color
                   for sw in self.prog.values()
                   for instr in sw.values()):
                continue

            for switch in self.prog.values():
                del switch[color]

    def normalize(self) -> Self:
        for _ in self.colors:
            self.normalize_states()
            self.normalize_colors()
            self.cut_unused_states()
            self.cut_unused_colors()
            self.cinch()

        return self.normalize_directions()
