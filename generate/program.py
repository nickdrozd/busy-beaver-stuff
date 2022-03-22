from __future__ import annotations

import re
from itertools import product
from typing import Dict, Iterator, List, Optional, Set, Tuple

from tm import parse, BlockTape, Machine
from generate.graph import Graph

INIT = 'A'
BLANK = '0'
SHIFTS = 'L', 'R'

class Program:
    def __init__(self, program: str):
        self.prog: Dict[str, Dict[int, str]] = {
            chr(state + 65): dict(enumerate(instructions))
            for state, instructions in enumerate(parse(program))
        }

        self.graph = Graph(program)

    def __repr__(self):
        return '  '.join([
            ' '.join(instrs.values())
            for instrs in self.prog.values()
        ])

    def __getitem__(self, slot: str):
        if len(slot) == 1:
            return self.prog[slot]

        state: str = slot[0]
        color: str = slot[1]
        return self.prog[state][int(color)]

    def __setitem__(self, slot: str, instr: str):
        if len(slot) == 1:
            self.prog[slot] = instr  # type: ignore
            return

        state: str = slot[0]
        color: str = slot[1]
        self.prog[state][int(color)] = instr

    def __eq__(self, other) -> bool:
        return str(self) == str(other)

    @property
    def states(self) -> Set[str]:
        return set(self.prog.keys())

    @property
    def non_start_states(self) -> List[str]:
        return sorted(self.states)[1:]

    @property
    def colors(self) -> Set[str]:
        return set(map(str, range(len(self.prog['A']))))

    @property
    def non_blank_colors(self) -> List[str]:
        return sorted(self.colors)[1:]

    @property
    def instructions(self):
        for state, instrs in self.prog.items():
            for color, instr in instrs.items():
                yield state + str(color), instr

    @property
    def actions(self):
        for instrs in self.prog.values():
            yield from instrs.values()

    @property
    def open_slots(self) -> Tuple[str, ...]:
        return tuple(
            slot
            for slot, instr in self.instructions
            if '.' in instr)

    @property
    def last_slot(self) -> Optional[str]:
        if len((slots := self.open_slots)) != 1:
            return None

        return slots[0]

    @property
    def halt_slots(self) -> Iterator[str]:
        yield from (
            slot
            for slot, instr in self.instructions
            if instr[2] in ('.', '_')
        )

    @property
    def blank_slots(self) -> Iterator[str]:
        yield from (
            slot
            for slot, instr in self.instructions
            if instr[0] == '0'
        )

    @property
    def used_states(self) -> Iterator[str]:
        yield from (
            action[2]
            for action in self.actions if
            '.' not in action
        )

    @property
    def available_states(self) -> Set[str]:
        used = set(self.used_states) | { INIT }
        diff = sorted(self.states.difference(used))

        return used | { diff[0] } if diff else used

    @property
    def used_colors(self) -> Iterator[str]:
        yield from (
            action[0]
            for action in self.actions if
            '.' not in action
        )

    @property
    def available_colors(self) -> Set[str]:
        used = set(self.used_colors) | { BLANK }
        diff = sorted(self.colors.difference(used))

        return used | { diff[0] } if diff else used

    @property
    def available_actions(self) -> Iterator[str]:
        return (
            ''.join(prod) for prod in
            product(
                self.available_colors,
                SHIFTS,
                self.available_states)
        )

    @property
    def instruction_sequence(self) -> Iterator[Tuple[str, int, str]]:
        partial = Program(
            re.sub(
                r'^\.\.\.',
                '1RB',
                '  '.join([
                    ' '.join(
                        ['...'] * len(self.colors))
                ] * len(self.states))))

        for _ in range(len(self.states) * len(self.colors) - 1):
            if (result := Machine(partial).run().final.undfnd) is None:
                return

            step, instr = result

            yield str(partial), step, instr

            partial[instr] = self[instr]

    def branch(self, instr: str, halt: bool = False) -> Iterator[str]:
        if halt and self.last_slot:
            return

        orig = self[instr]

        for action in sorted(self.available_actions):
            self[instr] = action
            yield str(self)

        self[instr] = orig

    def swap_states(self, st1: str, st2: str,) -> Program:
        self[st1], self[st2] = self[st2], self[st1]

        for slot, action in self.instructions:
            if st1 in action:
                self[slot] = re.sub(st1, st2, action)
            elif st2 in action:
                self[slot] = re.sub(st2, st1, action)

        return self

    def swap_colors(self, co1: str, co2: str) -> Program:
        ic1, ic2 = int(co1), int(co2)

        for state_str in self.states:
            state = self[state_str]
            state[ic1], state[ic2] = state[ic2], state[ic1]

        for slot, action in self.instructions:
            if co1 in action:
                self[slot] = re.sub(co1, co2, action)
            elif co2 in action:
                self[slot] = re.sub(co2, co1, action)

        return self

    def normalize_states(self) -> Program:
        for _ in self.states:
            todo = self.non_start_states

            for action in self.actions:
                if (state := action[2]) not in todo:
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
            todo = self.non_blank_colors

            for action in self.actions:
                if (color := action[0]) not in todo:
                    continue

                norm, *rest = todo

                if color != norm:
                    self.swap_colors(color, norm)

                todo = rest
            else:
                break

        return self

    def normalize(self) -> Program:
        for _ in self.colors:
            self.normalize_states()
            self.normalize_colors()

        return self

    @property
    def cant_halt(self) -> bool:
        return self._cant_reach(
            'halted',
            self.halt_slots,
            step_exp = 3,
        )

    @property
    def cant_blank(self) -> bool:
        return self._cant_reach(
            'blanks',
            self.blank_slots,
            step_exp = 2,
            **{'check_blanks': True},
        )

    @property
    def cant_spin_out(self) -> bool:
        return self._cant_reach(
            'spnout',
            (state + str(0) for state in
             self.graph.zero_reflexive_states),
        )

    def _cant_reach(
            self,
            final_prop: str,
            slots: Iterator[str],
            step_exp: int = 1,
            max_attempts: int = 10,
            **run_args,
    ):
        configs: List[Tuple[int, str, BlockTape]] = [# type: ignore
            (1, state, BlockTape([], color, []))     # type: ignore
            for state, color in slots
        ]

        while configs:  # pylint: disable = while-used
            step, state, tape = configs.pop()

            if step > max_attempts:
                return False

            run = Machine(self).run(
                step_lim = step ** step_exp,
                tape = tape.copy(),
                state = ord(state) - 65,
                **run_args,
            )

            if getattr(run.final, final_prop) is None:
                continue

            for entry in self.graph.entry_points[state]:
                for branch, (_, shift, trans) in self[entry].items():
                    if entry == state and branch == 0:
                        continue

                    if trans != state:
                        continue

                    for color in map(int, self.colors):
                        next_tape = tape.copy()

                        _ = next_tape.step(
                            not (0 if shift == 'L' else 1),
                            next_tape.scan,
                        )

                        next_tape.scan = color

                        configs.append((
                            step + 1,
                            entry,
                            next_tape,
                        ))

        return True
