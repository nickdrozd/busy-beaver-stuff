from tm.tape import Tape
from tm.parse import tcompile
from tm.recurrence import History

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

        lspan, scan, rspan = tape
        head, init = 0, 0

        marks = 0

        while True:

            # Output ###############################

            if watch_tape:
                print(f'{step : 5d} {chr(state + 65)} ', tape)

            # Bookkeeping ##########################

            if history is not None:
                history.positions.append(head)
                history.states.append(state)

                if samples is not None:
                    if step in history.tapes:
                        history.tapes[step] = \
                            Tape(lspan + [scan] + list(reversed(rspan)), init, head)
                else:
                    history.tapes.append(
                        None
                        if check_rec is None or step < check_rec else
                        Tape(lspan + [scan] + list(reversed(rspan)), init, head)
                    )

            # Halt conditions ######################

            if step >= x_limit:
                self._final = ('XLIMIT', step, None)
                break

            if check_rec is not None and step >= check_rec:
                action = state, scan

                result = history.check_for_recurrence(step, action)

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

            # Machine operation ####################

            try:
                color, shift, next_state = prog[state][scan]
            except TypeError:
                self._final = (
                    'UNDFND',
                    step,
                    chr(state + 65) + str(scan),
                )
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

            state = next_state

            # Bookkeeping ##########################

            step += 1

            marks += (1 * marked)

            # Halt conditions ######################

            if state == 7:  # ord('H') - 65
                self._final = ('HALTED', step, None)
                break

            if check_blanks and step != 0:
                if marks == 0:
                    self._final = ('BLANKS', step, None)
                    break

            # End of main loop #####################

        self._tape = lspan, scan, rspan
        self._steps = step

        self._marks = marks

        self._history = history
