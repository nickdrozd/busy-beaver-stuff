import json
from itertools import product
from collections import defaultdict, deque

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

        if self.last_slot:
            used.add('H')

        return used.union(diff[0]) if diff else used

    @property
    def actions(self):
        return (
            ''.join(prod) for prod in
            product(
                self.colors,
                SHIFTS,
                self.available_states)
        )

    def branch(self, instr):
        orig = self.prog[instr]

        output = []

        for action in self.actions:
            self.prog[instr] = action
            output.append(str(self))

        self.prog[instr] = orig

        return output


def tree_gen(steps):
    progs = deque(['1RB ... ... ... ... ...'])

    complete, xlimit = [], []

    output = {
        'CMPLTE': complete,
        'XLIMIT': xlimit,
        'FINSHD': {
            cat: defaultdict(list)
            for cat in (
                'BLANKS',
                'HALTED',
                'QSIHLT',
                'RECURR',
            )
        },
    }

    while progs:
        prog = progs.popleft()

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

        if status == 'XLIMIT':
            xlimit.append(prog)
            continue

        if status != 'UNDFND':
            if step < 15:
                continue

            output['FINSHD'][status][step].append(str(program))
            continue

        target = complete if program.last_slot else progs

        target.extend(
            program.branch(
                instr
            )
        )

    return {
        key: dict(val) if not isinstance(val, list) else val
        for key, val in output.items()
    }


if __name__ == '__main__':
    output = tree_gen(126)

    print(len(output['CMPLTE']))
    print(len(output['XLIMIT']))

    print(
        json.dumps(
            output,
            sort_keys = True,
            indent = 4,
        )
    )
