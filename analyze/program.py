from __future__ import annotations

import re
from itertools import product
from collections import defaultdict
from typing import Dict, Iterator, List, Optional, Set, Tuple

from tm import parse, BlockTape, Machine
from tm.parse import tcompile, st_str
from tm.recurrence import History
from analyze.graph import Graph

INIT = 'A'
BLANK = '0'
SHIFTS = 'L', 'R'

class Program:
    def __init__(self, program: str):
        self.prog: Dict[str, Dict[int, str]] = {
            st_str(state): dict(enumerate(instructions))
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
    def slots(self) -> Tuple[str, ...]:
        return tuple(slot for slot, _ in self.instructions)

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
    def halt_slots(self) -> Tuple[str, ...]:
        return tuple(
            slot
            for slot, instr in self.instructions
            if instr[2] in {'.', '_'}
        )

    @property
    def erase_slots(self) -> Tuple[str, ...]:
        return tuple(
            slot
            for slot, instr in self.instructions
            if instr[0] == '0' and slot[1] != '0'
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
        partial = Program.empty(len(self.states), len(self.colors))

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

        for action in sorted(self.available_actions, reverse = True):
            if action >= orig and '.' not in orig:
                continue
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

    def normalize_directions(self) -> Program:
        if self['A0'][1] == 'R':
            return self

        for slot, action in self.instructions:
            if (shift := action[1]) == 'R':
                self[slot] = re.sub('R', 'L', action)
            else:
                assert shift == 'L'
                self[slot] = re.sub('L', 'R', action)

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
            self.erase_slots,
            blank = True,
        )

    @property
    def cant_spin_out(self) -> bool:
        return self._cant_reach(
            'spnout',
            tuple(
                state + str(0) for state in
                self.graph.zero_reflexive_states),
        )

    def _cant_reach(
            self,
            final_prop: str,
            slots: Tuple[str, ...],
            max_attempts: int = 24,
            blank: bool = False,
    ):
        configs: List[
            Tuple[int, str, BlockTape, int, History]
        ] = [                              # type: ignore
            (
                1,
                state,                     # type: ignore
                BlockTape([], color, []),  # type: ignore
                0,
                History(),
            )
            for state, color in sorted(slots)
        ]

        comp = tcompile(str(self))

        max_repeats = max_attempts // 2

        seen: Dict[str, Set[str]] = defaultdict(set)

        while configs:  # pylint: disable = while-used
            step, state, tape, repeat, history = configs.pop()

            if step > max_attempts:
                return False

            if state == 'A' and tape.blank:
                return False

            if (tape_hash := str(tape)) in seen[state]:
                continue

            seen[state].add(tape_hash)

            history.add_state_at_step(step, state)  # type: ignore
            history.add_tape_at_step(step, tape)

            if history.check_for_recurrence(
                    step,
                    (state, tape.scan)) is None:  # type: ignore
                repeat = 0
            else:
                repeat += 1

                if repeat > max_repeats:
                    continue

            history.add_action_at_step(
                step,
                (state, tape.scan)) # type: ignore

            # print(step, state, tape)

            for entry in sorted(self.graph.entry_points[state]):
                for _, (_, shift, trans) in self[entry].items():
                    if trans != state:
                        continue

                    for color in sorted(map(int, self.colors)):
                        next_tape = tape.copy()

                        _ = next_tape.step(
                            not (0 if shift == 'L' else 1),
                            next_tape.scan,
                        )

                        next_tape.scan = color

                        run = Machine(comp).run(
                            step_lim = step + 1,
                            tape = next_tape.copy(),
                            state = ord(entry) - 65,
                        )

                        result = (
                            getattr(run.final, final_prop)
                            if not blank else
                            (
                                min(run.final.blanks.values())
                                if run.final.blanks else
                                None
                            )
                        )

                        if result is None:
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

        return True
