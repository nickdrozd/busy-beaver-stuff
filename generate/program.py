from itertools import product
from typing import Dict, Iterator, Optional, Set, Tuple

from tm import parse


SHIFTS = 'L', 'R'

class Program:
    def __init__(self, program: str):
        prog: Dict[str, Dict[int, str]] = {
            chr(state + 65): dict(enumerate(instructions))
            for state, instructions in enumerate(parse(program))
        }

        self.states: Set[str] = set(prog.keys())
        self.colors: Set[str] = set(map(str, range(len(prog['A']))))

        self.prog = prog

    def __repr__(self):
        return '  '.join([
            ' '.join(instrs.values())
            for instrs in self.prog.values()
        ])

    def __getitem__(self, slot: str):
        state, color = slot
        return self.prog[state][int(color)]

    def __setitem__(self, slot: str, instr: str):
        state, color = slot
        self.prog[state][int(color)] = instr

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
    def used_states(self) -> Set[str]:
        return {
            action[2]
            for action in self.actions if
            '.' not in action
        }

    @property
    def available_states(self) -> Set[str]:
        used = self.used_states.union('A')
        diff = sorted(self.states.difference(used))

        return used.union(diff[0]) if diff else used

    @property
    def used_colors(self) -> Set[str]:
        return {
            action[0]
            for action in self.actions if
            '.' not in action
        }

    @property
    def available_colors(self) -> Set[str]:
        used = self.used_colors.union('0')
        diff = sorted(self.colors.difference(used))

        return used.union(diff[0]) if diff else used

    @property
    def available_actions(self) -> Iterator[str]:
        return (
            ''.join(prod) for prod in
            product(
                self.available_colors,
                SHIFTS,
                self.available_states)
        )

    def branch(self, instr: str):
        # if self.last_slot:
        #     return

        orig = self[instr]

        for action in self.available_actions:
            self[instr] = action
            yield str(self)

        self[instr] = orig
