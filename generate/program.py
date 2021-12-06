from itertools import product
from typing import Dict, Iterator, Optional, Set, Tuple

from tm import parse


SHIFTS = 'L', 'R'

class Program:
    def __init__(self, program: str):
        self.prog: Dict[str, str] = {
            chr(state + 65) + str(color): action
            for state, instructions in enumerate(parse(program))
            for color, action in enumerate(instructions)
        }

        self.states: Set[str] = {key[0] for key in self.prog}
        self.colors: Set[str] = {key[1] for key in self.prog}

    def __repr__(self):
        return '  '.join(
            entry[1]
            for entry in
            sorted(
                {
                    state: ' '.join(
                        quint[1]
                        for quint in
                        sorted(
                            instr
                            for instr in self.prog.items()
                            if instr[0].startswith(state)
                        )
                    )
                    for state in self.states
                }.items()
            )
        )

    def __getitem__(self, slot):
        return self.prog[slot]

    def __setitem__(self, slot, instr):
        self.prog[slot] = instr

    @property
    def instructions(self):
        return self.prog.items()

    @property
    def actions(self):
        return self.prog.values()

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
