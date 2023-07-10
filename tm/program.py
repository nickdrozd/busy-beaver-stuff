from __future__ import annotations

import re
from typing import TYPE_CHECKING
from itertools import product
from functools import cached_property
from collections import defaultdict

from tm.graph import Graph
from tm.parse import parse, show_instr

from tm.tape import Tape
from tm.parse import tcompile
from tm.machine import Machine
from tm.lin_rec import History

if TYPE_CHECKING:
    from typing import Self

    from tm.parse import Color, Shift, State, Slot, Instr

    ProgStr = str

    Switch = dict[Color, Instr | None]


class BasicProgram:
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

    def __len__(self) -> int:
        return len(self.states) * len(self.colors)

    def __getitem__(self, slot: Slot) -> Instr | None:
        state, color = slot

        return self.prog[state][color]

    def get_switch(self, state: State) -> Switch:
        return self.prog[state]

    def __setitem__(self, slot: Slot, instr: Instr | None) -> None:
        state, color = slot

        self.prog[state][color] = instr

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
        return cls.init(states, colors).branch((1, 0))

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

        orig = self[slot]

        for instr in self.available_instrs:
            if orig is not None and instr >= orig:
                continue

            self[slot] = instr

            branches.append(str(self))

        self[slot] = orig

        return branches

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
        for _ in self.states:  # pragma: no branch
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
        for _ in self.colors:  # pragma: no branch
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
        # pylint: disable = unsubscriptable-object
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


class Program(BasicProgram):
    @property
    def instr_seq(self) -> list[tuple[ProgStr, int, Slot]]:
        seqs: list[tuple[ProgStr, int, Slot]] = []

        partial = Program.init(len(self.states), len(self.colors))

        for _ in range(len(self.states) * len(self.colors) - 1):
            if (result := Machine(partial).run().undfnd) is None:
                return seqs

            step, slot = result

            seqs.append((str(partial), step, slot))

            partial[slot] = self[slot]

        return seqs

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
            self.erase_slots,
        )

    @property
    def cant_spin_out(self) -> bool:
        return self._cant_reach(
            'spnout',
            self.spinout_slots,
        )

    def _cant_reach(
            self,
            final_prop: str,
            slots: tuple[Slot, ...],
            max_steps: int = 24,
            max_cycles: int = 1_000,
    ) -> bool:
        configs: list[
            tuple[int, State, Tape, int, History]
        ] = [
            (
                1,
                state,
                Tape.init(color),
                0,
                History(tapes = {}),
            )
            for state, color in sorted(slots)
        ]

        comp = tcompile(str(self))

        max_repeats = max_steps // 2

        seen: dict[State, set[Tape]] = defaultdict(set)

        def final_value(machine: Machine) -> int | None:
            match final_prop:
                case 'spnout':
                    return machine.spnout
                case 'blanks':
                    return (
                        min(blanks.values())
                        if (blanks := machine.blanks) else
                        None
                    )
                case 'halted':
                    return (
                        und[0]
                        if (und := machine.undfnd) else
                        machine.halted
                    )

            return None  # no-coverage

        for _ in range(max_cycles):
            try:
                step, state, tape, repeat, history = configs.pop()
            except IndexError:
                return True

            if step > max_steps:
                return False

            if state == 0 and tape.blank:
                return False

            if tape in seen[state]:
                continue

            seen[state].add(tape)

            history.add_state_at_step(step, state)
            history.add_tape_at_step(step, tape)

            if history.check_rec(
                    step,
                    slot := (state, tape.scan)) is None:
                repeat = 0
            else:
                repeat += 1

                if repeat > max_repeats:
                    continue

            history.add_slot_at_step(step, slot)

            # print(step, state, tape)

            for entry in sorted(self.graph.entry_points[state]):
                for _, instr in self.get_switch(entry).items():
                    if instr is None:
                        continue

                    _, shift, trans = instr

                    if trans != state:
                        continue

                    for color in self.colors:
                        next_tape = tape.copy()

                        _ = next_tape.step(
                            not shift,
                            next_tape.scan,
                            False,
                        )

                        next_tape.scan = color

                        run = Machine(comp).run(
                            sim_lim = step + 1,
                            tape = next_tape.copy(),
                            state = entry,
                        )

                        if not (result := final_value(run)):
                            continue

                        if abs(result - step) > 1:
                            continue

                        configs.append((
                            step + 1,
                            entry,
                            next_tape,
                            repeat,
                            history.copy(),
                        ))

        return False
