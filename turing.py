from tm.tape import Tape
from tm.parse import tcompile
from tm.history import History


class Machine:
    def __init__(self, prog):
        prog = prog.strip() if isinstance(prog, str) else str(prog)
        self._prog = prog
        self._comp = tcompile(prog)
        self._tape = None
        self._pos = None
        self._state = None
        self._steps = None
        self._marks = None
        self._final = None
        self._history = None

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

    def print_results(self):
        print(
            '\n'.join([
                f'marks: {self.marks}',
                f'steps: {self.steps}',
                f'beeps: {self.beeps}',
                f'final: {self.final}',
                '',
            ]))

    def run(
            self,
            tape,
            x_limit=100_000_000,
            watch_tape=False,
            check_rec=None,
            check_blanks=False,
            samples=None,
    ):
        state = 0

        step = 0
        prog = self._comp

        history = (
            None
            if samples is None and check_rec is None else
            History(tapes=samples)
        )

        marks = 0

        while True:

            # Output ###############################

            if watch_tape:
                print(f'{step : 5d} {chr(state + 65)} ', tape)

            # Bookkeeping ##########################

            if history is not None:
                history.positions.append(tape.head)
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

            # pylint: disable = protected-access

            scan = tape._list[tape._pos]

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

            tape._list[tape._pos] = color

            if shift:
                tape.head += 1
                tape._pos  += 1

                try:
                    tape._list[tape._pos]
                except IndexError:
                    tape._list.append(0)
                    tape.rspan += 1
            else:
                if tape.head + tape._init == 0:
                    tape._list.insert(0, 0)
                    tape._init += 1
                    tape._pos  += 1
                    tape.lspan -= 1

                tape.head -= 1
                tape._pos  -= 1

            # pylint: enable = protected-access

            # End of main loop #####################

        self._tape = tape
        self._steps = step

        self._marks = marks

        self._history = history

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

    # pylint: disable = pointless-statement
    tape1[ tape2.lspan : tape2.rspan ]

    pos1 = positions[steps]
    pos2 = positions[recurrence]

    if st1 != st2:
        return False

    if pos1 < pos2:
        diff = pos2 - pos1
        leftmost = min(positions[steps:])

        slice1 = tape1[        leftmost : ]
        slice2 = tape2[ diff + leftmost : ]

        slice_diff = len(slice1) - len(slice2)

        if slice_diff > 0:
            slice2 = slice2 + [0] * slice_diff

    elif pos1 > pos2:
        diff = pos1 - pos2
        rightmost = max(positions[steps:]) + 1

        slice1 = tape1[ : rightmost        ]
        slice2 = tape2[ : rightmost - diff ]

        slice_diff = len(slice1) - len(slice2)

        if slice_diff > 0:
            slice2 = [0] * slice_diff + slice2

    else:
        assert pos1 == pos2

        leftmost  = min(positions[steps:])
        rightmost = max(positions[steps:]) + 1

        slice1 = tape1[ leftmost : rightmost ]
        slice2 = tape2[ leftmost : rightmost ]

    return slice1 == slice2

########################################

def run_bb(
        prog,
        tape=None,
        x_limit=100_000_000,
        watch_tape=False,
        check_rec=None,
        check_blanks=False,
        samples=None,
):
    if tape is None:
        tape = [0]
    elif isinstance(tape, int):
        tape = [0] * tape

    machine = Machine(prog)
    machine.run(
        Tape(tape, init = len(tape) // 2),
        x_limit,
        watch_tape,
        check_rec,
        check_blanks,
        samples,
    )

    return machine
