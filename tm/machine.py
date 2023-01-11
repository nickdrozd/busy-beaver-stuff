from __future__ import annotations

from tm.tape import Tape
from tm.parse import tcompile, st_str
from tm.instrs import State, Slot, GetInstr
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

class Machine:
    program: str | GetInstr

    tape: Tape
    state: State
    steps: int
    cycles: int

    blanks: dict[State, int]

    prover: Prover | None = None

    halted: int | None = None
    spnout: int | None = None
    xlimit: int | None = None

    linrec: LinRec | None = None

    qsihlt: bool | None = None
    infrul: bool | None = None

    rulapp: int = 0

    undfnd: tuple[int, tuple[str, int]] | None = None

    def __init__(self, program: str | GetInstr):
        self.program = program

    def __str__(self) -> str:
        info = [ f'CYCLES: {self.cycles}' ]

        info.append(
            f'MARKS: {self.marks}')

        info += [
            f'{cat.upper()}: {data if self.rulapp == 0 else "..."}'
            for cat in TERM_CATS
            # pylint: disable = bad-builtin
            if (data := getattr(self, cat)) is not None
        ]

        if self.rulapp > 0:
            info.append(
                f'RULAPP: {self.rulapp}')

        return f"{self.program} || {' | '.join(info)}"

    @property
    def simple_termination(self) -> int | None:
        return self.spnout if self.halted is None else self.halted

    @property
    def marks(self) -> int:
        return self.tape.marks

    def show_tape(self, step: int, cycle: int, state: int) -> None:
        info = [
            f'{cycle: 5d}',
            f'{st_str(state)}{self.tape.scan}',
            str(self.tape),
        ]

        if not self.rulapp:
            info.insert(1, f'{step : 3d}')

        print(' | '.join(info))

    def run(self,
            step_lim: int | None = None,
            state: int = 0,
            sim_lim: int = 100_000_000,
            watch_tape: bool = False,
            tape: Tape | None = None,
            prover: int | None = None,
    ) -> Machine:
        comp: GetInstr = (
            tcompile(self.program)
            if isinstance(self.program, str) else
            self.program
        )

        self.tape = tape = (
            tape
            if tape is not None else
            Tape([], 0, [])
        )

        if prover is not None:
            self.prover = Prover(comp, diff_lim = prover)

        self.blanks = {}

        step: int = 0

        if step_lim:
            sim_lim = step_lim + 1

        for cycle in range(sim_lim):

            if watch_tape:
                self.show_tape(step, cycle, state)

            if step_lim is not None and step >= step_lim:
                self.xlimit = step
                break

            if self.prover:
                try:
                    rule = self.prover.try_rule(cycle, state, tape)
                except InfiniteRule:
                    self.infrul = True
                    break

                if rule is not None:
                    if (times := tape.apply_rule(rule)) is not None:
                        step += times
                        self.rulapp += times
                        continue

            if (instr := comp[state, tape.scan]) is None:
                self.undfnd = step, (st_str(state), tape.scan)
                break

            color, shift, next_state = instr

            if ((same_state := state == next_state)
                    and tape.at_edge(shift)):
                self.spnout = step
                break

            step += tape.step(shift, color, same_state)

            state = next_state

            if tape.blank:
                if state in self.blanks:
                    break

                self.blanks[state] = step

                if state == 0:
                    break

            if state == -1:
                break

        else:
            self.xlimit = step

        self.finalize(step, cycle, state)

        if watch_tape and bool(self.halted):
            self.show_tape(step, 1 + cycle, state)

        return self

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
            elif self.blanks:
                if (period := step - self.blanks[state]):
                    self.linrec = None, period
                    self.xlimit = None

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

class LinRecMachine:
    program: str

    tape: Tape
    history: History

    halted: int | None = None
    xlimit: int | None = None
    qsihlt: bool | None = None
    linrec: LinRec | None = None

    def __init__(self, program: str):
        self.program = program

    def run(
        self,
        step_lim: int | None = None,
        skip: bool = False,
        check_rec: int | None = None,
        samples: Tapes | None = None,
    ) -> LinRecMachine:
        assert (
            check_rec is not None
            or samples is not None)

        comp = tcompile(self.program)

        self.tape = tape = Tape([], 0, [])

        self.history = History(tapes = samples or {})

        step: int = 0
        state: State = 0

        for _ in range(step_lim or 1_000_000):
            self.history.add_state_at_step(step, state)

            slot: Slot = state, (scan := tape.scan)

            if ((check_rec is not None and step >= check_rec)
                or (samples is not None
                   and step in self.history.tapes)):
                self.history.add_tape_at_step(step, tape)

            if check_rec is not None and step >= check_rec:
                if self.check_rec(step, slot) is not None:
                    break

                self.history.add_slot_at_step(step, slot)

            if (instr := comp[state, scan]) is None:
                break

            color, shift, next_state = instr

            _ = tape.step(shift, color, skip and state == next_state)

            state = next_state

            step += 1

            if state == -1:
                self.halted = step
                break

        else:
            self.xlimit = step

        return self

    def check_rec(self, step: int, slot: Slot) -> RecRes | None:
        if (result := self.history.check_rec(step, slot)) is None:
            return None

        self.linrec = start, _rec = result

        hc_beeps = self.history.calculate_beeps()
        hp_beeps = self.history.calculate_beeps(start)

        self.qsihlt = any(
            hc_beeps[st] <= hp_beeps[st]
            for st in hp_beeps
        )

        return result
