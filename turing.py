import sys
from collections import defaultdict

def parse(program_string):
    instructions = iter(program_string.split())

    return tuple(
        zip(instructions, instructions, instructions, instructions)
        if '3' in program_string else
        zip(instructions, instructions, instructions)
        if '2' in program_string else
        zip(instructions, instructions)
    )

def tcompile(parsed):
    return tuple(
        tuple(
            (
                int(action[0]),
                0 if action[1] == 'L' else 1,
                ord(action[2]) - 65,
            )
            if '.' not in action else None
            for action in instr
        )
        for instr in parsed
    )

########################################

class Machine:
    def __init__(self, prog):
        prog = prog.strip() if isinstance(prog, str) else str(prog)
        self._prog = prog
        self._comp = tcompile(parse(prog))
        self._tape = None
        self._pos = None
        self._state = None
        self._steps = None
        self._marks = None
        self._beeps = None
        self._final = None
        self._tapes = None

    @property
    def program(self):
        return self._prog

    @property
    def steps(self):
        return self._steps

    @property
    def marks(self):
        return self._marks

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
    def tapes(self):
        return self._tapes

    @property
    def final(self):
        return self._final

    def run_to_halt(
            self,
            tape,
            x_limit=None,
            watch_tape=False,
            check_rec=None,
            check_blanks=False,
            collect_tapes=False,
    ):
        pos = len(tape) // 2
        init = pos

        state = 0

        step = 0
        beeps = defaultdict(lambda: 0)
        prog = self._comp

        if x_limit is None:
            x_limit = sys.maxsize

        if check_rec is not None:
            snapshots = defaultdict(lambda: [])

        tapes = None
        if collect_tapes or check_rec is not None:
            tapes = ([], [])

        marks = 0

        while True:

            # Output ###############################

            if watch_tape:
                print(f'{step : 5d} {chr(state + 65)} ', end='')
                print_tape(tape, pos, init)

            if tapes is not None:
                tapes[0].append(pos)
                tapes[1].append((
                    state,
                    tape.copy(),
                ))

            # Halt conditions ######################

            if state == 7:  # ord('H') - 65
                self._final = ('HALTED', step, None)
                break

            if step >= x_limit:
                self._final = ('XLIMIT', step, None)
                break

            if check_blanks and step != 0:
                if marks == 0:
                    self._final = ('BLANKS', step, None)
                    break

            if check_rec is not None and step >= check_rec:
                action = state, tape[pos]

                class BreakLoop(Exception):
                    '''This is control flow, not exception handling.'''

                try:
                    for pstep, pbeeps in snapshots[action]:
                        if verify_lin_recurrence(pstep, step - pstep, tapes):
                            raise BreakLoop(pstep, step, pbeeps)

                except BreakLoop as breakloop:
                    # pylint: disable = unbalanced-tuple-unpacking
                    pstep, step, pbeeps = breakloop.args

                    final = (
                        'RECURR'
                        if all(beeps[state] > pbeeps[state]
                               for state in pbeeps) else
                        'QSIHLT'
                    )

                    self._final = final, pstep, step - pstep
                    break

                snapshots[action].append((
                    step,
                    beeps.copy(),
                ))

            # Bookkeeping ##########################

            step += 1
            beeps[state] = step

            # Machine operation ####################

            try:
                color, shift, state = prog[state][tape[pos]]
            except TypeError:
                self._final = (
                    'UNDFND',
                    step,
                    chr(state + 65) + str(tape[pos]),
                )
                break

            if color and not tape[pos]:
                marks += 1
            elif not color and tape[pos]:
                marks -= 1

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
        self._marks = marks

        self._tapes = tapes


def print_results(machine):
    print(
        '\n'.join([
            f'marks: {machine.marks}',
            f'steps: {machine.steps}',
            f'beeps: {machine.beeps}',
            f'final: {machine.final}',
            '',
        ]))


def print_tape(tape, pos, init):
    squares = [
        '#' if square == 1 else
        '!' if square == 2 else
        '@' if square == 3 else
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

def verify_lin_recurrence(steps, period, tapes):
    positions, tapes = tapes

    recurrence = steps + period

    st1, tape1 = tapes[steps]
    st2, tape2 = tapes[recurrence]

    pos1 = positions[steps]
    pos2 = positions[recurrence]

    if st1 != st2:
        return False

    if pos1 < pos2:
        diff = pos2 - pos1
        offset = min(positions[steps:])

        slice1 = tape1[        offset : ]
        slice2 = tape2[ diff + offset : ]

        for i in range(len(slice1)):
            try:
                slice2[i]
            except IndexError:
                slice2.append(0)

    elif pos1 > pos2:
        diff = pos1 - pos2
        offset = max(positions[steps:]) + 1

        slice1 = tape1[ : offset        ]
        slice2 = tape2[ : offset - diff ]

        for i in range(len(slice1)):
            try:
                slice2[i]
            except IndexError:
                slice2.insert(0, 0)

    else:
        assert pos1 == pos2
        slice1, slice2 = tape1, tape2

    return slice1 == slice2

########################################

def run_bb(
        prog,
        tape=None,
        x_limit=None,
        watch_tape=False,
        check_rec=None,
        check_blanks=False,
        collect_tapes=False,
):
    if tape is None:
        tape = [0] * 50
    elif isinstance(tape, int):
        tape = [0] * tape

    machine = Machine(prog)
    machine.run_to_halt(
        tape,
        x_limit,
        watch_tape,
        check_rec,
        check_blanks,
        collect_tapes,
    )

    return machine

########################################

CANDIDATES = [

]

RCRNC = 0
STEPS = 100
BLANK = 0
PRINT = 1
STDIN = 0
TAPE  = 50

if __name__ == '__main__':
    source = sys.stdin if STDIN else CANDIDATES

    for i, program in enumerate(source):
        machine = run_bb(
            program,
            tape = TAPE,
            x_limit = STEPS,
            watch_tape = PRINT,
            check_rec = RCRNC,
            check_blanks = BLANK,
        )

        status, step, period = machine.final

        if status != 'XLIMIT':
            print(f'{i} | {machine.program} | {machine.final}')
