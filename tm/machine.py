from tm.tape import Tape
from tm.parse import tcompile
from tm.recurrence import History


class ValidationError(Exception):
    pass


class MachineResult:
    def __init__(self, prog):
        self.prog = prog

        self.blanks = None
        self.fixdtp = None
        self.halted = None
        self.linrec = None
        self.qsihlt = None
        self.undfnd = None
        self.xlimit = None

    def __str__(self):
        return ' | '.join([
            f'{reason}: {data}'
            for reason, data in
            {
                'BLANKS': self.blanks,
                'FIXDTP': self.fixdtp,
                'HALTED': self.halted,
                'LINREC': self.linrec,
                'QSIHLT': self.qsihlt,
                'XLIMIT': self.xlimit,
            }.items()
            if data is not None
        ])

    nonhalt = 'fixdtp', 'linrec', 'qsihlt', 'undfnd', 'xlimit'

    def validate_results(self):
        if self.halted is not None:
            for cat in self.nonhalt:
                if getattr(self, cat) is not None:
                    raise ValidationError(
                        f'{self.prog} || {cat} | {self}')

        if self.fixdtp and self.linrec is None:
            raise ValidationError(
                f'{self.prog} || {self}')


class Machine:
    def __init__(self, prog):
        prog = prog.strip() if isinstance(prog, str) else str(prog)
        self._prog = prog
        self._comp = tcompile(prog)
        self._tape = None
        self._state = None
        self._steps = None
        self._marks = None
        self._final = MachineResult(prog)
        self._history = None

    def __str__(self):
        return f'{self._prog} || {self._final}'

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

        lspan, scan, rspan = tape
        head, init = 0, len(lspan)

        marks = 0

        while True:  # pylint: disable = while-used

            # Output ###############################

            if watch_tape:
                print(f'{step : 5d} {chr(state + 65)}{scan} ',
                      Tape(lspan, scan, rspan, init, head))

            # Bookkeeping ##########################

            if history is not None:
                history.add_position_at_step(head, step)
                history.add_state_at_step(state, step)

                if samples is not None:
                    if step in history.tapes:
                        history.tapes[step] = \
                            Tape(lspan, scan, rspan, init, head)
                else:
                    history.add_tape_at_step(
                        None
                        if check_rec is None or step < check_rec else
                        Tape(lspan, scan, rspan, init, head),
                        step,
                    )

            # Halt conditions ######################

            if step >= x_limit:
                self._final.xlimit = step
                break

            if check_rec is not None and step >= check_rec:
                action = state, scan

                result = history.check_for_recurrence(step, action)

                if result is not None:
                    self._final.linrec = start, _rec = result

                    hc_beeps = history.calculate_beeps()

                    if any(st not in hc_beeps
                           for st in range(len(prog))):
                        self._final.qsihlt = result
                    else:
                        hp_beeps = history.calculate_beeps(start)

                        if any(hc_beeps[st] <= hp_beeps[st]
                               for st in hp_beeps):
                            self._final.qsihlt = result

                    self._final.fixdtp = history.tape_is_fixed(start)

                    break

                history.actions[action].append(step)

            # Machine operation ####################

            try:
                color, shift, next_state = prog[state][scan]
            except TypeError:
                self._final.undfnd = step, chr(state + 65) + str(scan)
                break

            if color:
                if not scan:
                    marked = 1
                else:
                    marked = 0
            else:
                if scan:
                    marked = -1
                else:
                    marked = 0

            stepped = 0

            init_scan = scan

            side = rspan if shift else lspan

            while scan == init_scan:  # pylint: disable=while-used
                if shift:
                    # push new color to the left
                    lspan.append(color)

                    # pull next color from the right
                    try:
                        scan = rspan.pop()
                    except IndexError:
                        scan = 0

                    head += 1

                else:
                    # push new color to the right
                    rspan.append(color)

                    # pull next color from the left
                    try:
                        scan = lspan.pop()
                    except IndexError:
                        scan = 0

                    if head + init == 0:
                        init += 1

                    head -= 1

                stepped += 1

                if state != next_state:
                    break

                try:
                    next_square = side[-1]
                except IndexError:
                    break

                if next_square != init_scan:
                    break

            state = next_state

            # Bookkeeping ##########################

            step += stepped

            marks += (marked * stepped)

            # Halt conditions ######################

            if state == 7:  # ord('H') - 65
                break

            if check_blanks and marks == 0:
                break

            # End of main loop #####################

        if state == 7:  # ord('H') - 65
            self._final.halted = step

        if check_blanks and marks == 0:
            self._final.blanks = step

        self._tape = lspan, scan, rspan
        self._steps = step

        self._marks = marks

        self._history = history

        self._final.validate_results()
