from __future__ import annotations

from collections import defaultdict
from typing import Dict, Optional, Tuple

from tm.tape import BlockTape
from tm.parse import tcompile, st_str, ProgLike
from tm.types import Action, State
from tm.recurrence import History, RecRes, Tapes

LinRec = Tuple[Optional[int], int]

TERM_CATS = (
    'halted',
    'linrec',
    'spnout',
    'undfnd',
    'xlimit',
)

class Machine:
    def __init__(self, prog: ProgLike):
        self.program = prog

        if type(prog).__name__ == 'Program':
            prog = str(prog)

        self._comp = tcompile(prog) if isinstance(prog, str) else prog

        self.tape: BlockTape
        self.state: State
        self.steps: int
        self.cycles: int

        self.history: Optional[History] = None

        self.reached: Dict[Action, int] = defaultdict(lambda: 0)

        self.blanks: Dict[State, int] = {}

        self.halted: Optional[int] = None
        self.spnout: Optional[int] = None
        self.xlimit: Optional[int] = None

        self.linrec: Optional[LinRec] = None

        self.qsihlt: Optional[bool] = None

        self.undfnd: Optional[Tuple[int, str]] = None

    def __str__(self) -> str:
        info = [ f'CYCLES: {self.cycles}' ]

        info += [
            f'{cat.upper()}: {data}'
            for cat in TERM_CATS
            # pylint: disable = bad-builtin
            if (data := getattr(self, cat)) is not None
        ]

        if self.blanks:
            info.append(
                f'BLANKS: {self.blanks}')

        return f"{self.program} || {' | '.join(info)}"

    @property
    def simple_termination(self) -> Optional[int]:
        return self.spnout if self.halted is None else self.halted

    @property
    def marks(self) -> int:
        return self.tape.marks

    def show_tape(self, step: int, cycle: int, state: int) -> None:
        print(' | '.join([
            f'{cycle: 5d}',
            f'{step : 5d}',
            f'{st_str(state)}{self.tape.scan}',
            str(self.tape),
        ]))

    def run(self,
            skip: bool = True,
            step_lim: Optional[int] = None,
            state: int = 0,
            sim_lim: int = 100_000_000,
            watch_tape: bool = False,
            check_rec: Optional[int] = None,
            samples: Optional[Tapes] = None,
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
                self.show_tape(step, cycle, state)

            # Halt conditions ######################

            if step_lim is not None and step >= step_lim:
                self.xlimit = step
                break

            if check_rec is not None and step >= check_rec:
                assert self.history is not None

                if self.check_rec(step, action) is not None:
                    break

                self.history.add_action_at_step(step, action)

            # Machine operation ####################

            try:
                color, shift, next_state = self._comp[state][scan]
            except TypeError:
                self.undfnd = step, f'{st_str(state)}{scan}'
                break

            self.reached[action] += 1

            if (state == next_state
                    and (shift == tape.edge or marks == 0)):
                self.spnout = step
                break

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
                    if state in self.blanks:
                        break

                self.blanks[state] = step

                if state == 0:
                    break

            if state == -1:
                break

            # End of main loop #####################
        else:
            self.xlimit = step

        self.finalize(step, cycle, state)

        if watch_tape and bool(self.halted):
            self.show_tape(step, 1 + cycle, state)

        return self

    def check_rec(self, step: int, action: Action) -> RecRes:
        assert self.history is not None

        if (result := self.history.check_rec(step, action)) is None:
            return None

        self.linrec = start, _rec = result

        hc_beeps = self.history.calculate_beeps()
        hp_beeps = self.history.calculate_beeps(start)

        self.qsihlt = any(
            hc_beeps[st] <= hp_beeps[st]
            for st in hp_beeps
        )

        return result

    def finalize(self, step: int, cycle: int, state: int) -> None:
        assert cycle <= step

        if state == -1:
            self.halted = step
            self.qsihlt = True

        if self.spnout is not None:
            self.qsihlt = True

        if self.tape.blank:
            if 0 in self.blanks:
                self.linrec = 0, step
            elif (blanks := self.blanks):
                if (period := step - blanks[state]):
                    self.linrec = None, period
                    self.xlimit = None

        self.blanks = blanks = {
            st_str(stt): stp   # type: ignore
            for stt, stp in self.blanks.items()
        }

        self.steps = step
        self.state = state
        self.cycles = cycle

        self.validate_results()

    def validate_results(self) -> None:
        assert len(results := [
            (cat, data)
            for cat in TERM_CATS
            # pylint: disable = bad-builtin
            if (data := getattr(self, cat)) is not None
        ]) == 1, results
