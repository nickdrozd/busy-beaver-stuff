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
        self._status = None

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

    @property
    def status(self):
        return self._status

    def run_to_halt(self, tape, x_limit=None, watch_tape=False, check_rec=False):
        pos = len(tape) // 2
        init = pos

        state = 0

        step = 0
        beeps = defaultdict(lambda: 0)
        prog = parse(self._prog)

        if x_limit is None:
            x_limit = sys.maxsize

        if check_rec:
            deviations = []
            snapshots = defaultdict(lambda: [])

        while True:

            # Output ###############################

            if watch_tape:
                print(f'{step} {state} ', end='')
                print_tape(tape, pos, init)

            # Halt conditions ######################

            if state == HALT:
                self._status = 'HALTED'
                break

            if step >= x_limit:
                self._status = 'XLIMIT'
                break

            if check_rec:
                dev = pos - init
                deviations.append(dev)

                action = state, tape[pos]

                class BreakLoop(Exception):
                    '''This is control flow, not exception handling.'''

                try:
                    for pstep, pinit, pdev, ptape, pbeeps in snapshots[action]:

                        if dev < pdev:
                            dmax = max(deviations[pstep:]) + 1

                            prev = ptape[ : pinit + dmax]
                            curr = tape[ : init + dmax + dev - pdev]

                            for i in range(len(prev)):
                                try:
                                    curr[i]
                                except IndexError:
                                    curr.insert(0, 0)

                            if prev == curr:
                                # print('L', pstep, step)
                                raise BreakLoop

                        elif pdev < dev:
                            dmin = min(deviations[pstep:])

                            prev = ptape[ pinit + dmin : ]
                            curr = tape[ init + dmin + dev - pdev : ]

                            for i in range(len(prev)):
                                try:
                                    curr[i]
                                except IndexError:
                                    curr.append(0)

                            if prev == curr:
                                # print('R', pstep, step)
                                raise BreakLoop

                        elif pdev == dev:
                            dmax = max(deviations[pstep:]) + 1
                            dmin = min(deviations[pstep:])

                            prev = ptape[pinit + dmin : pinit + dmax]
                            curr =  tape[init  + dmin : init  + dmax]

                            if prev == curr:
                                # print('C', pstep, step)
                                raise BreakLoop

                except BreakLoop:
                    self._status = 'RECURR'
                    break

                snapshots[action].append((
                    step,
                    init,
                    dev,
                    tape.copy(),
                    beeps.copy(),
                ))

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

def run_bb(prog, tape=None, x_limit=None, watch_tape=False, check_rec=False):
    if tape is None:
        tape = [0]

    machine = Machine(prog)
    machine.run_to_halt(tape, x_limit, watch_tape, check_rec)
    return machine

########################################

CANDIDATES = [

]

STEPS = 70
PRINT = 1
RCRNC = 1
STDIN = 1
FTAPE = 1

if __name__ == '__main__':
    source = sys.stdin if STDIN else CANDIDATES

    for i, program in enumerate(source):
        if run_bb(
                program,
                x_limit = STEPS,
                watch_tape = PRINT,
                check_rec = RCRNC,
                tape = [0] * 36 if FTAPE else None):
            print(program.strip())
