from __future__ import annotations

from collections import defaultdict
from typing import Any, Dict, Optional, Tuple

from tm.tape import BlockTape
from tm.parse import tcompile, st_str
from tm.recurrence import History, Action, RecRes


TERM_CATS = (
    'halted',
    'linrec',
    'spnout',
    'undfnd',
    'xlimit',
)

class MachineResult:
    def __init__(self, prog: str):
        self.prog = prog

        self.blanks: Dict[str, int] = {}

        self.fixdtp: Optional[bool] = None

        self.halted: Optional[int] = None
        self.spnout: Optional[int] = None
        self.xlimit: Optional[int] = None

        self.linrec: Optional[Tuple[int, int]] = None
        self.qsihlt: Optional[Tuple[int, int]] = None

        self.undfnd: Optional[Tuple[int, str]] = None

    def __str__(self):
        info = [
            f'{cat.upper()}: {data}'
            for cat in TERM_CATS
            if (data := getattr(self, cat)) is not None
        ]

        if self.blanks:
            info.append(
                f'BLANKS: {self.blanks}')

        return ' | '.join(info)

    def validate_results(self):
        assert len(results := [
            (cat, data)
            for cat in TERM_CATS
            if (data := getattr(self, cat)) is not None
        ]) == 1, results

        if self.simple_termination is not None:
            assert self.fixdtp is not None

    @property
    def simple_termination(self) -> Optional[int]:
        if self.halted is None:
            return self.spnout

        return self.halted


class Machine:
    def __init__(self, prog):
        self.program = prog

        if type(prog).__name__ == 'Program':
            prog = str(prog)

        if type(prog).__name__ == 'BlockMacro':
            # pylint: disable = pointless-statement
            prog[0][0]

        self._comp = tcompile(prog) if isinstance(prog, str) else prog
        self.tape    = None
        self.state   = None
        self.steps   = None
        self.cycles  = None
        self.final   = MachineResult(prog)
        self.history = None

        self.reached = defaultdict(lambda: 0)

    def __str__(self):
        return f'{self.program} || {self.final}'

    @property
    def marks(self):
        return self.tape.marks

    def show_tape(self, step, state):
        print(' '.join([
            f'{step : 5d}',
            f'{st_str(state)}{self.tape.scan}',
            str(self.tape),
        ]))

    def run(self,
            skip = True,
            step_lim = None,
            state: int = 0,
            sim_lim: int = 100_000_000,
            watch_tape: bool = False,
            check_rec: Optional[int] = None,
            samples: Optional[Dict[int, Any]] = None,
            tape: Optional[BlockTape] = None,
    ) -> Machine:
        self.tape = tape = (
            tape
            if tape is not None else
            BlockTape([], 0, [])
        )

        if samples is not None or check_rec is not None:
            self.history = History(tapes = samples)

        step: int = 0

        if step_lim:
            sim_lim = step_lim + 1

        marks: int = tape.marks

        for cycle in range(sim_lim):

            # Bookkeeping ##########################

            if self.history is not None:
                self.history.add_state_at_step(step, state)

                if ((check_rec is not None and step >= check_rec)
                    or (samples is not None
                       and step in self.history.tapes)):
                    self.history.add_tape_at_step(step, tape)

            action: Action = state, (scan := tape.scan)

            # Output ###############################

            if watch_tape:
                self.show_tape(step, state)

            # Halt conditions ######################

            if step_lim is not None and step >= step_lim:
                self.final.xlimit = step
                break

            if check_rec is not None and step >= check_rec:
                if self.check_rec(step, action) is not None:
                    break

                self.history.add_action_at_step(step, action)

            # Machine operation ####################

            try:
                color, shift, next_state = self._comp[state][scan]
            except TypeError:
                instr = f'{st_str(state)}{scan}'
                self.final.undfnd = step, instr
                break

            self.reached[action] += 1

            if self.history is None:
                if (state == next_state
                        and (shift == tape.edge or marks == 0)):
                    self.final.spnout = step
                    self.final.fixdtp = color == 0
                    break
            else:
                self.history.add_change_at_step(
                    step,
                    color != scan)

            change = (
                True
                if scan == 0 and color != 0 else
                False
                if scan != 0 and color == 0 else
                None
            )

            stepped = (
                tape.skip
                if skip and (state == next_state) else
                tape.step
            )(shift, color)

            state = next_state

            # Bookkeeping ##########################

            step += stepped

            if change is not None:
                if change:
                    marks += stepped
                else:
                    marks -= stepped

            # Halt conditions ######################

            if marks == 0:
                if check_rec is None and samples is None:
                    if state in self.final.blanks:
                        break

                self.final.blanks[state] = step

                if state == 0:
                    break

            if state == -1:
                break

            # End of main loop #####################
        else:
            self.final.xlimit = step

        if self.finalize(step, cycle, state) and watch_tape:
            self.show_tape(step, state)

        return self

    def check_rec(self, step: int, action: Action) -> RecRes:
        if (result := self.history.check_rec(step, action)) is None:
            return None

        self.final.linrec = start, _rec = result

        hc_beeps = self.history.calculate_beeps()
        hp_beeps = self.history.calculate_beeps(start)

        if any(hc_beeps[st] <= hp_beeps[st] for st in hp_beeps):
            self.final.qsihlt = result

        self.final.fixdtp = self.history.tape_is_fixed(start)

        return result

    def finalize(self, step, cycle, state) -> bool:
        assert cycle <= step

        show = (
            bool(self.final.halted)
            or bool(self.final.blanks)
        )

        if state == -1:
            self.final.halted = step
            self.final.fixdtp = True

        if self.tape.blank:
            if 0 in self.final.blanks:
                self.final.linrec = 0, step
                self.final.fixdtp = False
            elif (blanks := self.final.blanks):
                if (period := step - blanks[state]):
                    self.final.linrec = None, period
                    self.final.fixdtp = False
                    self.final.xlimit = None

        self.final.blanks = blanks = {
            st_str(stt): stp
            for stt, stp in self.final.blanks.items()
        }

        self.steps = step
        self.state = state
        self.cycles = cycle

        self.final.validate_results()

        return show
