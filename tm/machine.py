from __future__ import annotations

from dataclasses import dataclass
from collections import defaultdict

from tm.tape import BlockTape
from tm.parse import tcompile, st_str, ProgLike
from tm.types import Action, State
from tm.recurrence import History, RecRes, Tapes, Prover, InfiniteRule

LinRec = tuple[int | None, int]

TERM_CATS = (
    'halted',
    'infrul',
    'linrec',
    'spnout',
    'undfnd',
    'xlimit',
)

@dataclass
class Machine:
    program: ProgLike

    tape: BlockTape | None = None
    state: State | None = None
    steps: int | None = None
    cycles: int | None = None

    prover: Prover | None = None

    blanks: dict[State, int] | None = None
    reached: dict[Action, int] | None = None

    halted: int | None = None
    spnout: int | None = None
    xlimit: int | None = None

    linrec: LinRec | None = None

    qsihlt: bool | None = None
    infrul: bool | None = None

    rulapp: int = 0

    undfnd: tuple[int, str] | None = None

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
    def simple_termination(self) -> int | None:
        return self.spnout if self.halted is None else self.halted

    @property
    def marks(self) -> int:
        assert self.tape is not None
        return self.tape.marks

    def show_tape(self, step: int, cycle: int, state: int) -> None:
        assert self.tape is not None
        print(' | '.join([
            f'{cycle: 5d}',
            f'{step : 5d}',
            f'{st_str(state)}{self.tape.scan}',
            str(self.tape),
        ]))

    def run(self,
            step_lim: int | None = None,
            state: int = 0,
            sim_lim: int = 100_000_000,
            watch_tape: bool = False,
            tape: BlockTape | None = None,
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

        blanks: dict[State, int] = {}
        reached: dict[Action, int] = defaultdict(lambda: 0)

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

            stepped = tape.step(shift, color, same_state)

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
            blanks: dict[State, int],
            reached: dict[Action, int],
    ) -> None:
        assert cycle <= step

        self.reached = reached

        if state == -1:
            self.halted = step
            self.qsihlt = True

        if self.spnout is not None:
            self.qsihlt = True

        assert self.tape is not None

        if self.tape.blank:
            if 0 in blanks:
                self.linrec = 0, step
            elif blanks:
                if (period := step - blanks[state]):
                    self.linrec = None, period
                    self.xlimit = None

        self.blanks = {
            st_str(int(stt)): stp
            for stt, stp in blanks.items()
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

    tape: BlockTape | None = None
    history: History | None = None

    halted: int | None = None
    xlimit: int | None = None
    qsihlt: bool | None = None
    linrec: LinRec | None = None

    def run(
        self,
        step_lim: int | None = None,
        check_rec: int | None = None,
        samples: Tapes | None = None,
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
