from typing import Any, Dict, Optional, Tuple

from tm.parse import tcompile
from tm.recurrence import History

class ValidationError(Exception):
    pass

class MachineResult:
    def __init__(self, prog: str):
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
        prog = (
            prog.strip() if isinstance(prog, str) else
            prog if isinstance(prog, tuple) else str(prog)
        )
        self.program = prog
        self._comp = tcompile(prog) if isinstance(prog, str) else prog
        self.tape    = None
        self.state   = None
        self.steps   = None
        self.final   = MachineResult(prog)
        self.history = None
        self.reached = None

    def __str__(self):
        return f'{self.program} || {self.final}'

    @property
    def marks(self):
        return self.tape.marks()

    @property
    def beeps(self):
        return self.history.calculate_beeps()

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
            skip = True,
            xlimit: int = 100_000_000,
            watch_tape: bool = False,
            check_rec: Optional[int] = None,
            check_blanks: bool = False,
            samples: Optional[Dict[int, Any]] = None,
    ):
        prog = self._comp

        history: Optional[History] = (
            None
            if samples is None and check_rec is None else
            History(tapes=samples)
        )

        reached = set()

        state: int = 0
        step: int = 0

        while True:  # pylint: disable = while-used

            # Output ###############################

            if watch_tape:
                print(f'{step : 5d} {chr(state + 65)}{tape.scan} ',
                      tape.to_ptr())

            # Bookkeeping ##########################

            action: Tuple[int, int] = state, tape.scan

            if history is not None:
                history.add_position_at_step(tape.head, step)
                history.add_state_at_step(state, step)

                if samples is not None:
                    if step in history.tapes:
                        history.tapes[step] = tape.to_ptr()
                else:
                    history.add_tape_at_step(
                        None
                        if check_rec is None or step < check_rec else
                        tape.to_ptr(),
                        step,
                    )

            # Halt conditions ######################

            if step >= xlimit:
                self.final.xlimit = step
                break

            if check_rec is not None and step >= check_rec:
                assert history is not None

                result: Optional[Tuple[int, int]] = \
                    history.check_for_recurrence(step, action)

                if result is not None:
                    self.final.linrec = start, _rec = result

                    hc_beeps = history.calculate_beeps()

                    if any(st not in hc_beeps
                           for st in range(len(prog))):
                        self.final.qsihlt = result
                    else:
                        hp_beeps = history.calculate_beeps(start)

                        if any(hc_beeps[st] <= hp_beeps[st]
                               for st in hp_beeps):
                            self.final.qsihlt = result

                    self.final.fixdtp = history.tape_is_fixed(start)

                    break

                history.actions[action].append(step)

            # Machine operation ####################

            try:
                color, shift, next_state = prog[state][tape.scan]
            except TypeError:
                instr = chr(state + 65) + str(tape.scan)
                self.final.undfnd = step, instr
                break
            else:
                reached.add(action)

            if history is not None:
                history.add_change_at_step(color != tape.scan, step)

            stepped = 0

            init_scan = tape.scan

            side = tape.rspan if shift else tape.lspan

            if state != next_state:
                tape.step(shift, color)
                stepped += 1
            else:
                while tape.scan == init_scan:  # pylint: disable=while-used
                    tape.step(shift, color)

                    stepped += 1

                    if not skip:
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

            # Halt conditions ######################

            if state == 30:  # ord('_') - 65
                break

            if check_blanks and tape.blank():
                break

            # End of main loop #####################

        if state == 30:  # ord('_') - 65
            self.final.halted = step

        if check_blanks and tape.blank():
            self.final.blanks = step

        self.tape = tape
        self.steps = step

        self.history = history

        self.final.validate_results()

        self.reached = sorted(
            chr(s + 65) + str(c)
            for (s, c) in reached)
