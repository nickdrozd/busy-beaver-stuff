import sys
from collections import defaultdict

STATE_MAP = {
    'A': '0',
    'B': '1',
    'C': '2',
    'D': '3',
    'E': '4',
    'F': '5',
    'G': '6',
    'H': '7',
}

HALT = 7

SHIFT_MAP = {
    'L': 0,
    'R': 1,
}

def parse(program_string):
    instructions = iter([
        instr.replace(
            instr[2],
            STATE_MAP[instr[2]])
        for instr in
        program_string.split()
    ])

    zipped = (
        zip(instructions, instructions, instructions, instructions)
        if '3' in program_string else
        zip(instructions, instructions, instructions)
        if '2' in program_string else
        zip(instructions, instructions)
    )

    return tuple(
        tuple(
            (int(action[0]), int(SHIFT_MAP[action[1]]), int(action[2]))
            for action in instr)
        for instr in
        zipped
    )

########################################

class Machine:
    def __init__(self, prog):
        self._prog = prog.strip()
        self._tape = None
        self._pos = None
        self._state = None
        self._steps = None
        self._beeps = None

    @property
    def steps(self):
        return self._steps

    @property
    def marks(self):
        total = 0
        for square in self._tape:
            if square != 0:
                total += 1
        return total

    @property
    def beeps(self):
        return sorted(
            tuple(
                (key, val)
                for key, val in
                self._beeps.items()),
            key=lambda x: x[1],
            reverse=True)

    def run_to_halt(self, tape, x_limit=None, watch_tape=False):
        init = 0
        pos = 0

        state = 0

        step = 0
        beeps = defaultdict(lambda: 0)
        prog = parse(self._prog)

        if x_limit is None:
            x_limit = sys.maxsize

        while True:

            # Output ###############################

            if watch_tape:
                print_tape(tape, pos, init)

            # Halt conditions ######################

            if state == HALT:
                break

            if step >= x_limit:
                break

            # Bookkeeping ##########################

            step += 1
            beeps[state] = step

            # Machine operation ####################

            color, shift, state = prog[state][tape[pos]]

            tape[pos] = color

            if shift:
                pos += 1

                try:
                    tape[pos]
                except IndexError:
                    tape.append(0)

            else:
                if pos == 0:
                    tape.insert(0, 0)
                    init += 1
                else:
                    pos -= 1

            # End of main loop #####################

        self._pos = pos
        self._tape = tape
        self._steps = step

        self._beeps = dict(beeps)


def print_results(machine):
    print(
        '\n'.join([
            f'marks: {machine.marks}',
            f'steps: {machine.steps}',
            f'beeps: {machine.beeps}',
            '',
        ]))


def print_tape(tape, pos, init):
    squares = [
        '!' if square == 1 else
        '@' if square == 2 else
        '#' if square == 3 else
        '_' # if square == 0
        for square in tape
    ]

    with_pos = ''.join([
        (f'[{square}]' if i != init else f'[<{square}>]')
        if i == pos else
        (square if i != init else f'<{square}>')
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
    '1RB 1RH 0RC 1LB 1LA 0RB',  # total recurrence
    '1RB 1RH 1LB 0LC 1LA 1RA',  # partial, left barrier
    '1RB 1RH 1LC 1RA 1LA 0LC',  # partial, right barrier
]

STEPS = 60
PRINT = True
STDIN = False

if __name__ == '__main__':
    source = sys.stdin if STDIN else CANDIDATES

    for i, program in enumerate(source):
        print_results(
            run_bb(
                program,
                x_limit = STEPS,
                watch_tape = PRINT))
