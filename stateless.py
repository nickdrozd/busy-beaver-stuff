import sys
from itertools import product

########################################

class ParseError(Exception):
    pass


def parse(program_string):
    actions = program_string.split()

    action_count = len(actions)

    for m1, _, m2 in actions:
        for m in (m1, m2):
            try:
                if action_count < int(m):
                    raise ParseError(program_string)
            except ValueError:
                pass

    return tuple(
        tuple(
            int(instruction)
            if instruction.isdigit()
            else instruction
            for instruction in action
        ) for action in actions
    )

########################################

class Machine:
    def __init__(self, prog):
        self._prog = parse(prog)
        self._tape = None
        self._pos = None
        self._exec_count = None

    @property
    def exec_count(self):
        return self._exec_count

    @property
    def ones_count(self):
        total = 0
        for square in self._tape:
            if square != 0 and square != 'H':
                total += 1
        return total

    def run_to_halt(self, tape, x_limit=None, watch_tape=False):
        pos = 0

        exec_count = 0
        prog = self._prog

        if x_limit is None:
            x_limit = sys.maxsize

        while True:
            if exec_count >= x_limit:
                break

            if watch_tape:
                print_tape(tape, pos)

            read = tape[pos]

            if read == 'H':
                break

            mark_1, shift, mark_2 = prog[read]

            tape[pos] = mark_1

            if shift == 'R':
                pos += 1

                try:
                    tape[pos]
                except IndexError:
                    tape.append(0)

            else:
                if pos == 0:
                    tape.insert(0, 0)
                else:
                    pos -= 1

            tape[pos] = mark_2

            exec_count += 1

        self._pos = pos
        self._tape = tape
        self._exec_count = exec_count


def print_results(machine):
    print(
        '\n'.join([
            f'ones: {machine.ones_count}',
            f'exec: {machine.exec_count}',
            '',
        ]))


def print_tape(tape, pos):
    squares = [
        '_' if square == 0
        else str(square)
        for square in tape
    ]

    with_pos = ''.join([
        f'[{square}]' if i == pos else square
        for i, square in enumerate(squares)
    ])

    print(with_pos)

########################################

def run_bb(prog, tape=None, x_limit=None, watch_tape=False):
    if tape is None:
        tape = [0]

    machine = Machine(prog)
    machine.run_to_halt(tape, x_limit, watch_tape)
    return machine

########################################

CANDIDATES = [
    '1RH',
    '1R1 1RH',
]

COLOR = '0', '1', '2', 'H',
SHIFT = 'L', 'R'

ACTIONS = (
    ''.join(instr)
    for instr in product(COLOR, SHIFT, COLOR)
)

PROGRAMS = [
    ' '.join(actions)
    for actions in product(ACTIONS, repeat=3)
]

STEPS = 20
PRINT = False

if __name__ == '__main__':
    for i, program in enumerate(CANDIDATES):
        machine = run_bb(
            program,
            x_limit = STEPS,
            watch_tape = PRINT)

        print(program)
        print_results(machine)
