from __future__ import annotations

from dataclasses import dataclass
from collections import defaultdict
from typing import Dict, Optional, Tuple

from tm.tape import BlockTape
from tm.parse import tcompile, st_str, ProgLike
from tm.types import Action, State
from tm.recurrence import History, RecRes, Tapes, Prover, InfiniteRule

LinRec = Tuple[Optional[int], int]

TERM_CATS = (
    'halted',
    'infrul',
    'linrec',
    'spnout',
    'undfnd',
    'xlimit',
)

class Machine:
    def __init__(self, prog: ProgLike):
        self.program = prog

        self.tape: BlockTape
        self.state: State
        self.steps: int
        self.cycles: int

        self.prover: Optional[Prover] = None

        self.blanks: Dict[State, int]

        self.reached: Dict[Action, int]

        self.halted: Optional[int] = None
        self.spnout: Optional[int] = None
        self.xlimit: Optional[int] = None

        self.linrec: Optional[LinRec] = None

        self.qsihlt: Optional[bool] = None
        self.infrul: Optional[bool] = None

        self.rulapp: int = 0

        self.undfnd: Optional[Tuple[int, str]] = None

    def __str__(self) -> str:
        info = [ f'CYCLES: {self.cycles}' ]

        info += [
            f'{cat.upper()}: {data}'
            for cat in TERM_CATS
            # pylint: disable = bad-builtin
            if (data := getattr(self, cat)) is not None
        ]

        info.append(
            f'MARKS: {self.marks}')

        if self.rulapp > 0:
            info.append(
                f'RULAPP: {self.rulapp}')

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
            tape: Optional[BlockTape] = None,
            prover: bool = False,
    ) -> Machine:
        comp = (
            tcompile(self.program)
            if isinstance(self.program, str) else
            tcompile(str(self.program))
            if type(self.program).__name__ == 'Program' else
            self.program
        )

        self.tape = tape = (
            tape
            if tape is not None else
            BlockTape([], 0, [])
        )

        blanks: Dict[State, int] = {}
        reached: Dict[Action, int] = defaultdict(lambda: 0)

        if prover:
            self.prover = Prover(comp)

        step: int = 0

        if step_lim:
            sim_lim = step_lim + 1

        for cycle in range(sim_lim):

            # Bookkeeping ##########################

            action: Action = state, (scan := tape.scan)

            # Output ###############################

            if watch_tape:
                self.show_tape(step, cycle, state)

            # Halt conditions ######################

            if step_lim is not None and step >= step_lim:
                self.xlimit = step
                break

            # Machine operation ####################

            if self.prover:
                try:
                    times = self.prover.try_rule(cycle, state, tape)
                except InfiniteRule:
                    self.infrul = True
                    break

                if times is not None:
                    step += times
                    self.rulapp += times
                    continue

            try:
                color, shift, next_state = comp[state][scan]
            except TypeError:
                self.undfnd = step, f'{st_str(state)}{scan}'
                break

            reached[action] += 1

            if ((same_state := state == next_state)
                    and (shift == tape.edge or tape.blank)):
                self.spnout = step
                break

            stepped = tape.step(shift, color, skip and same_state)

            state = next_state

            # Bookkeeping ##########################

            step += stepped

            # Halt conditions ######################

            if tape.blank:
                if state in blanks:
                    break

                blanks[state] = step

                if state == 0:
                    break

            if state == -1:
                break

            # End of main loop #####################
        else:
            self.xlimit = step

        self.finalize(step, cycle, state, blanks, reached)

        if watch_tape and bool(self.halted):
            self.show_tape(step, 1 + cycle, state)

        return self

    def finalize(
            self,
            step: int,
            cycle: int,
            state: int,
            blanks: Dict[State, int],
            reached: Dict[Action, int],
    ) -> None:
        assert cycle <= step

        self.blanks = blanks
        self.reached = reached

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

########################################

@dataclass
class LinRecMachine:
    prog: str

    tape: Optional[BlockTape] = None
    history: Optional[History] = None

    halted: Optional[int] = None
    xlimit: Optional[int] = None
    qsihlt: Optional[bool] = None
    linrec: Optional[LinRec] = None

    def run(
        self,
        step_lim: Optional[int] = None,
        check_rec: Optional[int] = None,
        samples: Optional[Tapes] = None,
    ) -> LinRecMachine:
        assert (
            check_rec is not None
            or samples is not None)

        comp = tcompile(self.prog)

        self.tape = tape = BlockTape([], 0, [])

        self.history = History(tapes = samples)

        step: int = 0
        state: State = 0

        for _ in range(step_lim or 1_000_000):
            self.history.add_state_at_step(step, state)

            action: Action = state, (scan := tape.scan)

            if ((check_rec is not None and step >= check_rec)
                or (samples is not None
                   and step in self.history.tapes)):
                self.history.add_tape_at_step(step, tape)

            if check_rec is not None and step >= check_rec:
                if self.check_rec(step, action) is not None:
                    break

                self.history.add_action_at_step(step, action)

            color, shift, next_state = comp[state][scan]  # type: ignore

            _ = tape.step(shift, color, False)

            state = next_state

            step += 1

            if state == -1:
                self.halted = step
                break

        else:
            self.xlimit = step

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
