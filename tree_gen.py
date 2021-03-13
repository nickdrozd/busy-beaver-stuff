from pprint import pprint
from itertools import product
from collections import defaultdict

from turing import parse, run_bb

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

        try:
            return used.union(diff[0])
        except IndexError:
            return used

    def branch(self, instr):
        actions = (
            ''.join(prod) for prod in
            product(
                self.colors,
                SHIFTS,
                self.available_states.union('H'))
        )

        output = []

        for action in actions:
            prog = self.prog.copy()
            prog[instr] = action

            output.append(
                ' '.join(
                    x[1] for x in
                    sorted(prog.items())
                )
            )

        return output


def tree_gen(steps):
    progs = ['1RB ... ... ... ... ...']

    categories = 'BLANKS', 'HALTED', 'QSIHLT', 'RECURR', 'XLIMIT'

    output = {cat: defaultdict(set) for cat in categories}

    while progs:
        prog, *progs = progs

        if '.' not in prog:
            if 'H' in prog and '1RH' not in prog:
                continue

            if prog.count('H') > 1:
                continue

        program = Program(prog)

        machine = run_bb(
            program,
            x_limit = steps,
            check_rec = 0,
            check_blanks = True,
        )

        status, step, instr = machine.final

        if status != 'UNDFND':
            if step < 15:
                continue

            output[status][step].add(str(program))
            continue

        progs.extend(
            program.branch(
                instr
            )
        )

    return {
        key: dict(val)
        for key, val in
        output.items()
    }


if __name__ == '__main__':
    output = tree_gen(200)
    pprint(output)
