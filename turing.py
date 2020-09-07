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
        pos = 0
        state = 0

        exec_count = 0
        beep_count = defaultdict(lambda: 0)
        prog = self._prog

        if x_limit is None:
            x_limit = sys.maxsize

        while True:
            if state == HALT:
                break

            if exec_count >= x_limit:
                break

            if watch_tape:
                print_tape(tape, pos)

            old_state = state

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
                else:
                    pos -= 1

            exec_count += 1
            beep_count[old_state] = exec_count

        self._pos = pos
        self._tape = tape
        self._exec_count = exec_count

        self._beep_count = dict(beep_count)

    def print_results(self):
        print('\n'.join([
            f'ones: {self.ones_count}',
            f'exec: {self.exec_count}',
            f'beep: {self.beep_count}',
            '',
        ]))


def print_tape(tape, pos):
    squares = [
        '!' if square == 1 else
        '@' if square == 2 else
        '#' if square == 3 else
        '_' # if square == 0
        for square in tape
    ]

    with_pos = ''.join([
        f'[{square}]' if i == pos else square
        for i, square in enumerate(squares)
    ])

    print(with_pos)

########################################

MACHINES = {
    'BB_2_2': "1RB 1LB 1LA 1RH",
    'BB_3_2': "1RB 1RH 1LB 0RC 1LC 1LA",
    'BB_4_2': "1RB 1LB 1LA 0LC 1RH 1LD 1RD 0RA",
    'BB_5_2': "1RB 1LC 1RC 1RB 1RD 0LE 1LA 1LD 1RH 0LA",

    'BB_2_3': "1RB 2LB 1RH 2LA 2RB 1LB",
    'BB_2_4': "1RB 2LA 1RA 1RA 1LB 1LA 3RB 1RH",
    'RU_2_4': "1RB 3LA 1LA 1RA 2LA 1RH 3RA 3RB",
    'COPY_3_3_2_4': "1RB 1LC 1RH 1LA 1LC 2RB 1RB 2LC 1RC",

    'BBB_3_2': "1RB 0LB 1LA 0RC 1LC 1LA",
    'BBB_4_2': "1RB 1RC 1LC 1RD 1RA 1LD 0RD 0LB",

    'BBB_2_3_shift': "1RB 2LB 1LA 2LB 2RA 0RA",
    'BBB_2_3_sigma': "1RB 2LB 1RA 2LB 2LA 0RA",

    'SIAB_3_3': "1RB 2LB 1LC 1LA 2RB 1RB 1RH 2LA 0LC",
}

for name, prog in MACHINES.items():
    exec(f'{name} = "{prog}"')


def run_bb(prog, tape=None, x_limit=None, watch_tape=False):
    if tape is None:
        tape = [0]

    machine = Machine(prog)
    machine.run_to_halt(tape, x_limit, watch_tape)
    return machine

########################################

CANDIDATES = [
    SIAB_3_3,
]

STEPS = None
PRINT = False

if __name__ == '__main__':
    for i, program in enumerate(CANDIDATES):
        run_bb(program, x_limit=STEPS, watch_tape=PRINT).print_results()
