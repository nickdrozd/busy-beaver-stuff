from __future__ import annotations

import re
from typing import overload
from itertools import product
from functools import cached_property
from collections import defaultdict
from collections.abc import Iterator

from tm.tape import Tape
from tm.graph import Graph
from tm.machine import Machine
from tm.recurrence import History
from tm.parse import parse, st_str, str_st, tcompile, comp_instr
from tm.instrs import (
    Color, Shift, State, Slot, Instr, CompSlot, CompInstr)

ProgStr = str

SHIFTS: tuple[Shift, Shift] = 'L', 'R'

class Program:
    def __init__(self, program: ProgStr):
        self.prog: dict[State, dict[Color, Instr | None]] = {
            st_str(state): dict(enumerate(instrs))
            for state, instrs in enumerate(parse(program))
        }

        self.graph = Graph(program)

    def __repr__(self) -> ProgStr:
        return '  '.join([
            ' '.join(
                ''.join(map(str, instr)) if instr else '...'
                for instr in instrs.values()
            )
            for instrs in self.prog.values()
        ])

    @overload
    def __getitem__(self, slot: State) -> dict[Color, Instr | None]: ...

    @overload
    def __getitem__(self, slot: Slot) -> Instr | None: ...

    @overload
    def __getitem__(self, slot: CompSlot) -> CompInstr | None: ...

    def __getitem__(
            self,
            slot: State | Slot | CompSlot,
    ) -> dict[Color, Instr | None] | Instr | CompInstr | None:
        if isinstance(slot, State):
            return self.prog[slot]

        state, color = slot

        if isinstance(state, State):
            return self.prog[state][color]

        return comp_instr(
            self.prog[st_str(state)][color]
        )

    def __setitem__(self, slot: Slot, instr: Instr | None) -> None:
        state, color = slot

        self.prog[state][color] = instr

    def __eq__(self, other: object) -> bool:
        return str(self) == str(other)

    @staticmethod
    def empty(states: int, colors: int) -> Program:
        return Program(
            re.sub(
                r'^\.\.\.',
                '1RB',
                '  '.join([
                    ' '.join(
                        ['...'] * colors)
                ] * states)))

    @cached_property
    def states(self) -> set[State]:
        return set(self.prog.keys())

    @cached_property
    def colors(self) -> set[Color]:
        return set(range(len(self.prog['A'])))

    @property
    def instr_slots(self) -> Iterator[tuple[Slot, Instr | None]]:
        for state, instrs in self.prog.items():
            for color, instr in instrs.items():
                yield (state, color), instr

    @property
    def used_instr_slots(self) -> Iterator[tuple[Slot, Instr]]:
        yield from (
            (slot, instr)
            for slot, instr in self.instr_slots
            if instr is not None
        )

    @property
    def instructions(self) -> Iterator[Instr | None]:
        for instrs in self.prog.values():
            yield from instrs.values()

    @property
    def used_instructions(self) -> Iterator[Instr]:
        yield from(instr for instr in self.instructions if instr)

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
            if instr is None or instr[2] == '_'
        )

    @property
    def erase_slots(self) -> tuple[Slot, ...]:
        return tuple(
            slot
            for slot, instr in self.used_instr_slots
            if slot[1] != 0 and instr[0] == 0
        )

    @property
    def used_states(self) -> Iterator[State]:
        yield from (state for _, _, state in self.used_instructions)

    @property
    def available_states(self) -> set[State]:
        used = set(self.used_states) | { 'A' }
        diff = sorted(self.states.difference(used))

        return used | { diff[0] } if diff else used

    @property
    def used_colors(self) -> Iterator[Color]:
        yield from (color for color, _, _ in self.used_instructions)

    @property
    def available_colors(self) -> set[Color]:
        used = set(self.used_colors) | { 0 }
        diff = sorted(self.colors.difference(used))

        return used | { diff[0] } if diff else used

    @property
    def available_instrs(self) -> Iterator[tuple[Color, Shift, State]]:
        return product(
            self.available_colors,
            SHIFTS,
            self.available_states)

    @property
    def instr_seq(self) -> Iterator[tuple[ProgStr, int, Slot]]:
        partial = Program.empty(len(self.states), len(self.colors))

        for _ in range(len(self.states) * len(self.colors) - 1):
            if (result := Machine(partial).run().undfnd) is None:
                return

            step, slot = result

            yield str(partial), step, slot

            partial[slot] = self[slot]

    def branch(
            self,
            slot: Slot,
            halt: bool = False,
    ) -> Iterator[ProgStr]:
        if halt and self.last_slot:
            return

        orig = self[slot]

        for instr in sorted(self.available_instrs, reverse = True):
            if orig is not None and instr >= orig:
                continue

            self[slot] = instr

            yield str(self)

        self[slot] = orig

    def swap_states(self, st1: State, st2: State) -> Program:
        self.prog[st1], self.prog[st2] = self.prog[st2], self.prog[st1]

        for slot, (color, shift, state) in self.used_instr_slots:
            self[slot] = color, shift, (
                st1 if state == st2 else
                st2 if state == st1 else
                state
            )

        return self

    def swap_colors(self, co1: Color, co2: Color) -> Program:
        for state_str in self.states:
            st_key = self[state_str]
            st_key[co1], st_key[co2] = st_key[co2], st_key[co1]

        for slot, (color, shift, state) in self.used_instr_slots:
            self[slot] = (
                co1 if color == co2 else
                co2 if color == co1 else
                color
            ), shift, state

        return self

    def normalize_states(self) -> Program:
        for _ in self.states:
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

    def normalize_colors(self) -> Program:
        for _ in self.colors:
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

    def normalize_directions(self) -> Program:
        # pylint: disable = unsubscriptable-object
        if (index := self['A', 0]) is None or index[1] == 'R':
            return self

        for slot, (color, shift, state) in self.used_instr_slots:
            self[slot] = color, ('L' if shift == 'R' else 'R'), state

        return self

    def normalize(self) -> Program:
        for _ in self.colors:
            self.normalize_states()
            self.normalize_colors()

        return self.normalize_directions()

    @property
    def cant_halt(self) -> bool:
        return self._cant_reach(
            'halted',
            self.halt_slots,
        )

    @property
    def cant_blank(self) -> bool:
        return self._cant_reach(
            'blanks',
            tuple(
                (slot[0], slot[1])
                for slot in self.erase_slots),
        )

    @property
    def cant_spin_out(self) -> bool:
        return self._cant_reach(
            'spnout',
            tuple(
                (state, 0)
                for state in self.graph.zero_reflexive_states),
        )

    def _cant_reach(
            self,
            final_prop: str,
            slots: tuple[Slot, ...],
            max_attempts: int = 24,
    ) -> bool:
        configs: list[
            tuple[int, State, Tape, int, History]
        ] = [
            (
                1,
                state,
                Tape([], color, []),
                0,
                History(tapes = {}),
            )
            for state, color in sorted(slots)
        ]

        comp = tcompile(str(self))

        max_repeats = max_attempts // 2

        seen: dict[State, set[Tape]] = defaultdict(set)

        while configs:  # pylint: disable = while-used
            step, state, tape, repeat, history = configs.pop()

            if step > max_attempts:
                return False

            if state == 'A' and tape.blank:
                return False

            if tape in seen[state]:
                continue

            seen[state].add(tape)

            history.add_state_at_step(step, str_st(state))
            history.add_tape_at_step(step, tape)

            if history.check_rec(
                    step,
                    action := (str_st(state), tape.scan)) is None:
                repeat = 0
            else:
                repeat += 1

                if repeat > max_repeats:
                    continue

            history.add_action_at_step(step, action)

            # print(step, state, tape)

            for entry in sorted(self.graph.entry_points[state]):
                for _, instr in self[entry].items():
                    if instr is None:
                        continue

                    _, shift, trans = instr

                    if trans != state:
                        continue

                    for color in self.colors:
                        next_tape = tape.copy()

                        _ = next_tape.step(
                            not (0 if shift == 'L' else 1),
                            next_tape.scan,
                            False,
                        )

                        next_tape.scan = color

                        run = Machine(comp).run(
                            step_lim = step + 1,
                            tape = next_tape.copy(),
                            state = str_st(entry),
                        )

                        # pylint: disable = bad-builtin
                        if not (result := getattr(run, final_prop)):
                            continue

                        if final_prop == 'blanks':
                            result = min(run.blanks.values())

                        if abs(result - step) > 1:
                            continue

                        configs.append((
                            step + 1,
                            entry,
                            next_tape,
                            repeat,
                            history.copy(),
                        ))

        return True
