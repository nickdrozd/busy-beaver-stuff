from itertools import product

from tm.parse import parse


SHIFTS = 'L', 'R'

class Program:
    def __init__(self, prog_string):
        self.prog = {
            chr(state + 65) + str(color): action
            for state, instructions in enumerate(parse(prog_string))
            for color, action in enumerate(instructions)
        }

        self.states = {key[0] for key in self.prog}
        self.colors = {key[1] for key in self.prog}

    def __str__(self):
        return ' '.join(
            instr[1]
            for instr in
            sorted(self.prog.items()))

    @property
    def slots(self):
        return tuple(
            instr
            for instr in self.prog.values()
            if '.' in instr)

    @property
    def last_slot(self):
        return len(self.slots) == 1

    @property
    def is_complete(self):
        return len(self.slots) == 0

    @property
    def used_states(self):
        return {
            action[2]
            for action in self.prog.values() if
            '.' not in action
        }

    @property
    def available_states(self):
        used = self.used_states.union('A')
        diff = sorted(self.states.difference(used))

        return used.union(diff[0]) if diff else used

    @property
    def used_colors(self):
        return {
            action[0]
            for action in self.prog.values() if
            '.' not in action
        }

    @property
    def available_colors(self):
        used = self.used_colors.union('0')
        diff = sorted(self.colors.difference(used))

        return used.union(diff[0]) if diff else used

    @property
    def actions(self):
        return (
            ''.join(prod) for prod in
            product(
                self.available_colors,
                SHIFTS,
                self.available_states)
        )

    def branch(self, instr):
        orig = self.prog[instr]

        for action in self.actions:
            self.prog[instr] = action
            yield str(self)

        self.prog[instr] = orig
