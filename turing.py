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

class Tape:
    def __init__(self, underlying_list=None, init=None, head=None):
        self._list = underlying_list or [0]
        self._init = init or len(self) // 2
        self._head = head or 0
        self._pos  = self._head + self._init

    def copy(self):
        return Tape(
            self._list.copy(),
            self._init,
            self._head,
        )

    def __repr__(self):
        squares = [
            '#' if square == 1 else
            '!' if square == 2 else
            '@' if square == 3 else
            '_' # if square == 0
            for square in self._list
        ]

        return ''.join([
            (f'[{square}]' if i != self._init else f'[<{square}>]')
            if i == self._head + self._init else
            (square if i != self._init else f'<{square}>')
            for i, square in enumerate(squares)
        ])

    def __len__(self):
        return len(self._list)

    def __getitem__(self, tape_index):
        if isinstance(tape_index, int):
            try:
                return self._list[tape_index + self._init]
            except Tape_IndexError:
                return 0

        if tape_index.stop is None:
            right = None
        else:
            right = tape_index.stop + self._init

            for _ in range(right - len(self._list)):
                self._list.append(0)

            right = tape_index.stop + self._init

        if tape_index.start is None:
            left = None
        else:
            left = tape_index.start + self._init

            for _ in range(0 - left):
                self._list.insert(0, 0)
                self._init += 1
                self._pos  += 1

            left = tape_index.start + self._init

        return self._list[ left : right ]

    def __setitem__(self, tape_index, value):
        self._list[tape_index + self._init] = value

    def read(self):
        return self._list[self._pos]

    def print(self, color):
        self._list[self._pos] = color

    def right(self):
        self._head += 1
        self._pos  += 1

        try:
            self._list[self._pos]
        except IndexError:
            self._list.append(0)

    def left(self):
        if self._head + self._init == 0:
            self._list.insert(0, 0)
            self._init += 1
            self._pos  += 1

        self._head -= 1
        self._pos  -= 1


class OperatingHistory:
    def __init__(self, tapes=None):
        self.tapes = [] if tapes is None else tapes
        self.beeps = []
        self.states = []
        self.actions = defaultdict(lambda: [])
        self.positions = []

    def calculate_beeps(self, through=None):
        states = (
            self.states
            if through is None else
            self.states[:through]
        )

        steps = len(states)
        rev   = list(reversed(states))

        return {
            state: steps - 1 - rev.index(state)
            for state in set(states)
        }


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
        return self._history.calculate_beeps()

    @property
    def history(self):
        return self._history

    @property
    def final(self):
        return self._final

    def run(
            self,
            tape,
            x_limit=None,
            watch_tape=False,
            check_rec=None,
            check_blanks=False,
            samples=None,
    ):
        state = 0

        step = 0
        prog = self._comp

        if x_limit is None:
            x_limit = sys.maxsize

        history = (
            None
            if samples is None and check_rec is None else
            OperatingHistory(tapes=samples)
        )

        marks = 0

        while True:

            # Output ###############################

            if watch_tape:
                print(f'{step : 5d} {chr(state + 65)} ', end='')
                print(tape)

            # Bookkeeping ##########################

            if history is not None:
                history.positions.append(tape._head)
                history.states.append(state)

                if samples is not None:
                    if step in history.tapes:
                        history.tapes[step] = tape.copy()
                else:
                    history.tapes.append(
                        None
                        if check_rec is None or step < check_rec else
                        tape.copy()
                    )

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
                action = state, tape.read()

                result = check_for_recurrence(
                    step,
                    action,
                    history,
                )

                if result is not None:
                    step, rec = result

                    hp_beeps = history.calculate_beeps(step)
                    hc_beeps = history.calculate_beeps()

                    self._final = (
                        (
                            'RECURR'
                            if all(hc_beeps[state] > hp_beeps[state]
                                   for state in hp_beeps) else
                            'QSIHLT'
                        ),
                        step,
                        rec,
                    )

                    break

                history.actions[action].append(step)

            # Bookkeeping ##########################

            step += 1

            # Machine operation ####################

            scan = tape.read()

            try:
                color, shift, state = prog[state][scan]
            except TypeError:
                self._final = (
                    'UNDFND',
                    step,
                    chr(state + 65) + str(scan),
                )
                break

            if color:
                if not scan:
                    marks += 1
            else:
                if scan:
                    marks -= 1

            tape.print(color)

            if shift:
                tape.right()
            else:
                tape.left()

            # End of main loop #####################

        self._tape = tape
        self._steps = step

        self._marks = marks

        self._history = history


def print_results(machine):
    print(
        '\n'.join([
            f'marks: {machine.marks}',
            f'steps: {machine.steps}',
            f'beeps: {machine.beeps}',
            f'final: {machine.final}',
            '',
        ]))

########################################

def check_for_recurrence(step, action, history):
    for pstep in history.actions[action]:
        if verify_lin_recurrence(pstep, step - pstep, history):
            return pstep, step - pstep

    return None


def verify_lin_recurrence(steps, period, history):
    tapes     = history.tapes
    states    = history.states
    positions = history.positions

    recurrence = steps + period

    st1 = states[steps]
    st2 = states[recurrence]

    tape1 = tapes[steps]
    tape2 = tapes[recurrence]

    pos1 = positions[steps]
    pos2 = positions[recurrence]

    if st1 != st2:
        return False

    if pos1 < pos2:
        diff = pos2 - pos1
        offset = min(positions[steps:])

        slice1 = tape1[        offset : ]
        slice2 = tape2[ diff + offset : ]

        for _ in range(len(slice1) - len(slice2)):
            slice2.append(0)

    elif pos1 > pos2:
        diff = pos1 - pos2
        offset = max(positions[steps:]) + 1

        slice1 = tape1[ : offset        ]
        slice2 = tape2[ : offset - diff ]

        for _ in range(len(slice1) - len(slice2)):
            slice2.insert(0, 0)

    else:
        assert pos1 == pos2

        left  = min(positions[steps:])
        right = max(positions[steps:]) + 1

        slice1 = tape1[ left : right ]
        slice2 = tape2[ left : right ]

    return slice1 == slice2

########################################

def run_bb(
        prog,
        tape=None,
        x_limit=None,
        watch_tape=False,
        check_rec=None,
        check_blanks=False,
        samples=None,
):
    if tape is None:
        tape = [0] * 30
    elif isinstance(tape, int):
        tape = [0] * tape

    machine = Machine(prog)
    machine.run(
        Tape(tape),
        x_limit,
        watch_tape,
        check_rec,
        check_blanks,
        samples,
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
