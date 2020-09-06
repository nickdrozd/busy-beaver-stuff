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

    def run_to_halt(self, tape):
        pos = 0
        state = 0

        exec_count = 0
        beep_count = defaultdict(lambda: 0)
        prog = self._prog

        for _ in range(STEPS):
            if state == HALT:
                break

            # self.print_tape(tape, pos, exec_count)

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

    def print_results(self, print_tape=False):
        print('\n'.join([
            f'ones: {self.ones_count}',
            f'exec: {self.exec_count}',
            f'beep: {self.beep_count}',
            '',
        ]))

    def print_tape(self, tape, pos, step):
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
}

BB_2_2 = MACHINES['BB_2_2']
BB_3_2 = MACHINES['BB_3_2']
BB_4_2 = MACHINES['BB_4_2']
BB_5_2 = MACHINES['BB_5_2']
BB_2_3 = MACHINES['BB_2_3']
BB_2_4 = MACHINES['BB_2_4']

def run_bb(prog, tape=None):
    if tape is None:
        tape = [0]

    machine = Machine(prog)
    machine.run_to_halt(tape)
    return machine

########################################

CANDIDATES = [
    # BB_2_2,
    # BB_3_2,
    # BB_4_2,
    # BB_5_2,
    # BB2_3,
    # BB2_4,

    # # BBB(3) = 55
    # "1RB 0LB 1LA 0RC 1LC 1LA",  # normal
    # "1LB 0RB 1RA 0LC 1RC 1RA",  # 55
    # "1LB 0RB 1LC 0LC 1RC 1RA",  # 54
    # "1LB 0RC 1RB 0LC 1RC 1RA",  # 52
    # "1LB 0RC 0RC 0LC 1RC 1RA",  # 51

    # # BBB(4) = 2819 ???
    # "1RB 1RC 1LC 1RD 1RA 1LD 0RD 0LB",  # normal
    # "1LB 1LC 1RC 1LD 1LA 1RD 0LD 0RB",
    # "1LB 1LD 1RD 1LC 0LC 0RB 1LA 1RC",
    # "1LC 1LB 1LA 1RD 1RB 1LD 0LD 0RC",
    # "1LC 1LD 0LB 0RC 1RD 1LB 1LA 1RB",
    # "1LD 1LB 1LA 1RC 0LC 0RD 1RB 1LC",
    # "1LD 1LC 0LB 0RD 1LA 1RB 1RC 1LB",

    # # 2568
    # "1RB 1RA 0RC 0RB 0RD 1RA 1LD 1LB",  # normal
    # "1RD 1RA 1LB 1LD 0RB 1RA 0RC 0RD",
    # "1LD 1LA 1RB 1RD 0LB 1LA 0LC 0LD",
    # "1LB 1LA 0LC 0LB 0LD 1LA 1RD 1RB",
    # "1LB 1LA 0LD 0LB 1RC 1RB 0LC 1LA",
    # "1LD 1LA 0LC 1LA 1RC 1RD 0LB 0LD",
    # "1RD 1RA 0RC 1RA 1LC 1LD 0RB 0RD",

    # # 2512 -- basically the same as 2568
    # "1LB 1LA 0LC 1RA 1RC 1RD 0LB 0LD",
    # "1LB 1LA 0LD 1RA 0LB 0LC 1RD 1RC",

    # # Smaller 4-state progs > 107

    # # 1153
    # "1RB 1LB 1RC 0LD 0RD 0RA 1LD 0LA",

    # # 705
    # "1RB 1LC 1RC 0RD 0RD 0RC 1LD 1LA",

    # # 703
    # "1RB 1LC 0RC 0RD 0RD 0RC 1LD 1LA",

    # # 478
    # "1RB 0LC 1LC 0RD 0LC 1LA 1RA 0RD",

    # # 456
    # "1RB 1LC 0RC 1RB 0RD 0RC 1LD 1LA",

    # # 326
    # "1RB 0RC 0RC 1RB 1LC 1LD 1LA 0LC",

    # # 259
    # "1RB 0RC 1LC 1RC 0LD 1RA 1LD 0LA",

    # # 219
    # "1RB 0RC 0RC 1RC 0LD 1RA 1LD 0LA",

    # # 209
    # "1RB 0LC 1LC 0RC 1LC 1LD 0RD 1LA",

    # # 161
    # "1RB 1LC 1RC 0RB 1LC 0LD 1LA 1LD",

    # # 159
    # "1RB 1LA 1LC 1RD 1LC 0LD 1LA 0RB",

    # # 147
    # "1RB 0LC 1LC 0RC 1LC 1LD 1RB 1LA",

    # # 126
    # "1RB 0RC 1RC 1RB 1LC 1LD 0RA 1LA",

    # # 119
    # "1RB 0LC 1RC 0RD 1LC 1LD 1LA 0RC",

    # # 118
    # "1RB 1LC 1RC 0RD 0LC 0LD 1RA 1LA",

    # # 116
    # "1RB 0LC 1LC 0RD 1LC 1LD 1LA 0RC",

    # # 109
    # "1RB 0LC 0LC 0RD 1LC 1LD 1LA 0RC",

    # BBB(2, 3)

    # "1RB 2LB 1LA 2LB 2RA 0RA",  # 59
    # "1RB 0LB 1RA 1LB 2LA 2RA",  # 45
    # "1RB 2LB 1RA 2LB 2LA 0RA",  # 43
    # "1RB 2RA 2LB 2LB 2LA 0LA",  # 40

    # "1RB 2LA 1LA 2LA 2RB 0RA",  # Wolram's "universal machine"

    # BBB(2, 4)
]

STEPS = 50_000_000

if __name__ == '__main__':
    for i, prog in enumerate(CANDIDATES):
        machine = run_bb(prog)
        machine.print_results()

    # for i, prog in enumerate(sys.stdin):
    #     machine = run_bb(prog)

    #     if len(machine.beep_count) < 4:
    #         continue

    #     second = machine.beep_count[1][1]

    #     if second > 9_000 or second < 107:
    #         continue

    #     print(f'{i} | {prog.strip()} | {machine.beep_count[1:]}')
