from __future__ import annotations

from typing import TYPE_CHECKING

from tm.tape import Tape
from tm.blocks import opt_block
from tm.prover import Prover, ConfigLimit
from tm.show import show_slot, show_number
from tm.rules import RuleLimit, InfiniteRule, SuspectedRule
from tm.macro import BlockMacro, BacksymbolMacro, tcompile

if TYPE_CHECKING:
    from typing import Self

    from tm.tape import Count
    from tm.parse import State, Slot, GetInstr

    LinRec = tuple[int | None, int]
    Undfnd = tuple[int, Slot]

    Result = str | int | LinRec | Undfnd


TERM_CATS = (
    'cfglim',
    'halted',
    'infrul',
    'limrul',
    'linrec',
    'spnout',
    'undfnd',
    'xlimit',
)


class BasicMachine:
    program: str | GetInstr
    comp: GetInstr

    tape: Tape
    state: State
    steps: int
    cycles: int

    blanks: dict[State, int]

    halted: int | None = None
    spnout: int | None = None
    xlimit: int | None = None

    linrec: LinRec | None = None
    undfnd: Undfnd | None = None

    qsihlt: bool | None = None

    infrul: bool | None = None

    rulapp: Count = 0

    def __init__(self, program: str | GetInstr):
        self.program = program

        self.comp = (
            tcompile(self.program)
            if isinstance(self.program, str) else
            self.program
        )

    @property
    def term_results(self) -> list[tuple[str, Result]]:
        results = []

        for cat in TERM_CATS:
            try:
                # pylint: disable = bad-builtin
                data = getattr(self, cat)
            except AttributeError:
                continue

            if data is not None:
                results.append((cat, data))

        return results

    def __str__(self) -> str:
        info = [ f'CYCLES: {self.cycles}' ]

        info.append(
            f'MARKS: {show_number(self.marks)}')

        info += [
            '{}: {}'.format(
                cat.upper(),
                data if isinstance(data, int | str) else "...")
            for cat, data in self.term_results
        ]

        if self.rulapp:
            rulapp_disp = (
                show_number(self.rulapp)
                if isinstance(self.rulapp, int) else
                "..."
            )

            info.append(
                f'RULAPP: {rulapp_disp}')

        return f"{self.program} || {' | '.join(info)}"

    @property
    def simple_termination(self) -> Count | None:
        return self.spnout if self.halted is None else self.halted

    @property
    def marks(self) -> Count:
        return self.tape.marks

    def show_tape(
            self,
            step: int | None,
            cycle: int,
            state: int,
    ) -> None:
        info = [
            f'{cycle: 5d}',
            show_slot((state, self.tape.scan)),
            str(self.tape),
        ]

        if step is not None:
            info.insert(1, f'{step : 3d}')

        print(' | '.join(info))


    def finalize(
            self,
            step: int | None,
            cycle: int,
            state: State,
    ) -> None:
        if step is None:
            step = -1

        if state == -1:
            self.halted = step
            self.qsihlt = True

        if self.spnout is not None:
            self.qsihlt = True

        if self.tape.blank:
            if state == -1:
                self.blanks[-1] = step

            if 0 in self.blanks:
                self.linrec = 0, step
            elif self.blanks:
                if (period := step - self.blanks[state]):
                    self.linrec = None, period
                    self.xlimit = None

        self.steps = step
        self.state = state
        self.cycles = cycle


class QuickMachine(BasicMachine):
    def run(self,
            sim_lim: int = 100_000_000,
            *,
            state: State = 0,
            tape: Tape | None = None,
    ) -> Self:
        comp = self.comp

        if tape is None:
            tape = Tape.init()

        self.tape = tape

        self.blanks = {}

        step: int = 0

        for cycle in range(sim_lim):

            if (instr := comp[state, tape.scan]) is None:
                self.undfnd = step, (state, tape.scan)
                break

            color, shift, next_state = instr

            if (same := state == next_state) and tape.at_edge(shift):
                self.spnout = step
                break

            stepped = tape.step(shift, color, same)

            assert isinstance(stepped, int)

            step += stepped

            if (state := next_state) == -1:
                break

            if not color and tape.blank:
                if state in self.blanks:
                    break

                self.blanks[state] = step

                if state == 0:
                    break

        else:
            self.xlimit = step

        self.finalize(step, cycle, state)

        return self


class Machine(BasicMachine):
    prover: Prover

    cfglim: bool | None = None
    limrul: str | None = None
    susrul: tuple[int, int] | None = None

    def __init__(
            self,
            program: str | GetInstr,
            *,
            blocks: int | list[int] | None = None,
            backsym: int | list[int] | None = None,
            opt_macro: int | None = None,
    ):
        if opt_macro is not None:
            assert isinstance(program, str)
            program = find_opt_macro(program, opt_macro)

        if blocks is not None:
            if isinstance(blocks, int):
                blocks = [blocks]

            program = BlockMacro(program, blocks)

        if backsym is not None:
            if isinstance(backsym, int):
                backsym = [backsym]

            program = BacksymbolMacro(program, backsym)

        super().__init__(program)

    def __str__(self) -> str:
        return '{} | TPCFGS: {}'.format(
            super().__str__(),
            self.prover.config_count,
        )

    def run(
        self,
        sim_lim: int = 100_000_000,
        watch_tape: bool = False,
    ) -> Self:
        comp = self.comp

        self.tape = tape = Tape.init()

        self.prover = Prover(comp)

        self.blanks = {}

        step: int | None = 0

        state: State = 0

        for cycle in range(sim_lim):

            if watch_tape:
                self.show_tape(step, cycle, state)

            try:
                rule = self.prover.try_rule(cycle, state, tape)
            except InfiniteRule:
                self.infrul = True
                break
            except RuleLimit as lim:
                self.limrul = str(lim)
                break
            except ConfigLimit:  # no-cover
                self.cfglim = True
                break
            except SuspectedRule as sus:
                self.susrul = sus.args
                rule = None

            if rule is not None:
                try:
                    times = tape.apply_rule(rule)
                except RuleLimit as lim:
                    self.limrul = str(lim)
                    break

                if times is not None:
                    # print(f'--> applied rule: {rule}')
                    step = None
                    self.rulapp += times
                    continue

            if (instr := comp[state, tape.scan]) is None:
                self.undfnd = step or -1, (state, tape.scan)
                break

            color, shift, next_state = instr

            if (same := state == next_state) and tape.at_edge(shift):
                self.spnout = step or -1
                break

            stepped = tape.step(shift, color, same)

            if not isinstance(stepped, int):
                step = None
            elif step is not None:
                step += stepped

            if (state := next_state) == -1:
                break

            if not color and tape.blank:
                if state in self.blanks:
                    break

                self.blanks[state] = step or -1

                if state == 0:
                    break

        else:
            self.xlimit = step or -1

        self.finalize(step, cycle, state)

        if watch_tape and (bool(self.halted) or bool(self.blanks)):
            self.show_tape(step, 1 + cycle, state)

        return self

########################################

def find_opt_macro(prog: str, steps: int) -> str | GetInstr:
    if (blocks := opt_block(prog, steps)) > 1:
        return BlockMacro(prog, [blocks])

    return prog
