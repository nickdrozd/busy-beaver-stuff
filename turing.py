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
        self._prog = parse(prog)
        self._tape = None
        self._pos = None
        self._state = None
        self._exec_count = None
        self._beep_count = None

    @property
    def exec_count(self):
        return self._exec_count

    @property
    def ones_count(self):
        total = 0
        for square in self._tape:
            if square != 0:
                total += 1
        return total

    @property
    def beep_count(self):
        return sorted(
            tuple(
                (key, val)
                for key, val in
                self._beep_count.items()),
            key=lambda x: x[1],
            reverse=True)

    def run_to_halt(self, tape, x_limit=None, watch_tape=False):
        init = 0
        pos = 0

        state = 0

        exec_count = 0
        beep_count = defaultdict(lambda: 0)
        prog = self._prog

        if x_limit is None:
            x_limit = sys.maxsize

        while True:

            # Halt conditions ######################

            if state == HALT:
                break

            if exec_count >= x_limit:
                break

            # Output ###############################

            if watch_tape:
                print_tape(tape, pos, init)

            # Bookkeeping ##########################

            exec_count += 1
            beep_count[state] = exec_count

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
        self._exec_count = exec_count

        self._beep_count = dict(beep_count)


def print_results(machine):
    print(
        '\n'.join([
            f'ones: {machine.ones_count}',
            f'exec: {machine.exec_count}',
            f'beep: {machine.beep_count}',
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
    '1RB 1RH 0LC 1RB 1LA 1LC',
    '1RB 1RH 1LC 1RB 1LA 1LC',
    '1RB 1RH 1LC 1RB 0RA 0LC',
    '1RB 1RH 1LC 1RA 0RA 0LC',
    '1RB 1RH 1LC 0RB 1LC 1LA',
    '1RB 1RH 1LC 0RB 1LB 1LA',
    '1RB 1RH 1LC 0RA 1RB 0LB',
    '1RB 1RH 1LC 0RA 0RB 0LB',
    '1RB 1RH 0LC 0RA 0RA 1LB',
    '1RB 1RH 0LC 0RA 1LA 1LB',
    '1RB 1RH 0LC 0RB 0RA 1LB',
    '1RB 1RH 0LC 0RB 1LC 1LA',
    '1RB 1RH 0RC 0RB 1LC 1LA',
    '1RB 1RH 0RC 1LB 1LA 0LA',
    '1RB 1RC 0RC 1RH 1LC 0LA',
    '1RB 1RC 1LC 1RH 0RA 0LB',
    '1RB 0RB 1LC 1RH 0LA 1RC',
    '1RB 1RA 1LC 1RH 1RA 1LC',
    '1RB 0RC 0RC 1RH 1LC 0LA',
    '1RB 1RA 1LC 1RH 0RA 1LC',
    '1RB 1RA 0LC 1RH 1RA 1LC',
    '1RB 1RA 0LC 1RH 0RA 1LC',
    '1RB 0LB 1LC 1RB 1RH 1LA',
    '1RB 0LC 0LC 1RA 1RH 1LA',
    '1RB 0LC 1LB 1RA 1RH 1LA',
    '1RB 1LC 1LC 1RB 1RH 1LA',
    '1RB 0RC 1LA 1RB 1RH 1LB',
    '1RB 1LA 0LA 0LC 1RH 1RA',
    '1RB 1LA 0LC 1RB 1RH 1LA',
    '1RB 1LA 1LA 0LC 1RH 1RA',
    '1RB 1LA 1LA 1RC 1RH 1RB',
    '1RB 1LA 1LC 0LC 1RH 1RA',
    '1RB 1LA 1LC 1RB 1RH 1LA',
    '1RB 1LA 1LC 1RB 1RH 1RA',
    '1RB 0LB 1LA 0RC 1RB 1RH',
    '1RB 0LC 0LA 0RA 1LA 1RH',
    '1RB 0LC 1LA 0RA 1LA 1RH',
    '1RB 1LA 1RC 1RB 0LA 1RH',
    '1RB 1LA 1RC 1RB 1LA 1RH',
    '1RB 1LC 0LA 0RB 1LA 1RH',
]

STEPS = 300
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
