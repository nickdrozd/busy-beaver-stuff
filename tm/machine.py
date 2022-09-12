# pylint: disable = attribute-defined-outside-init
from __future__ import annotations

from collections import defaultdict
from typing import Any, Dict, Optional, Tuple

from tm.tape import BlockTape
from tm.parse import tcompile
from tm.recurrence import History


NONHALT = (
    'linrec',
    'qsihlt',
    'spnout',
    'undfnd',
    'xlimit',
)

REASONS = NONHALT + (
    'blanks',
    'fixdtp',
    'halted',
)

class MachineResult:
    def __init__(self, prog: str):
        self.prog = prog

        for reason in REASONS:
            setattr(self, reason, None)

    def __str__(self):
        return ' | '.join([
            f'{reason.upper()}: {data}'
            for reason in REASONS
            if (data := getattr(self, reason)) is not None
        ])

    def validate_results(self):
        if self.halted is not None:
            for cat in NONHALT:
                assert getattr(self, cat) is None

            assert self.fixdtp is not None

        if (spnout := self.spnout) is not None:
            lstep, _ = self.linrec
            assert lstep == spnout

            qstep, _ = self.qsihlt
            assert qstep == spnout

    @property
    def simple_termination(self):
        if self.halted is None:
            assert self.spnout is not None
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

    def run(self,
            tape = None,
            skip = True,
            step_lim = None,
            state: int = 0,
            sim_lim: int = 100_000_000,
            watch_tape: bool = False,
            check_rec: Optional[int] = None,
            check_blanks: bool = False,
            samples: Optional[Dict[int, Any]] = None,
    ) -> Machine:
        prog = self._comp

        self.tape = tape = (
            tape
            if isinstance(tape, BlockTape) else
            BlockTape([], 0, [], extend_to = tape)
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

            action: Tuple[int, int] = state, (scan := tape.scan)

            # Output ###############################

            if watch_tape:
                print(f'{step : 5d} {chr(state + 65)}{scan} ', tape)

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
                color, shift, next_state = prog[state][scan]
            except TypeError:
                instr = chr(state + 65) + str(scan)
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

            if state == -1:
                break

            if check_blanks and marks == 0:
                break

            # End of main loop #####################
        else:
            self.final.xlimit = step

        show = self.finalize(step, cycle, state)

        if watch_tape and show:
            print(f'{step : 5d} {chr(state + 65)}{scan} ', tape)

        return self

    def check_rec(self, step, action):
        result: Optional[Tuple[int, int]] = \
            self.history.check_for_recurrence(step, action)

        if result is None:
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

        if state == -1:
            self.final.halted = step
            self.final.fixdtp = True

        if self.tape.blank:
            self.final.blanks = step

        if self.final.spnout is not None:
            self.final.qsihlt = self.final.linrec = step, 1

        self.steps = step
        self.state = state
        self.cycles = cycle

        self.final.validate_results()

        return self.final.halted or self.final.blanks
