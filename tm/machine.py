from __future__ import annotations

from typing import TYPE_CHECKING

from tm.tape import Tape, show_number
from tm.blocks import opt_block
from tm.prover import Prover, ConfigLimit
from tm.show import show_slot
from tm.rules import apply_rule, RuleLimit, InfiniteRule, SuspectedRule
from tm.macro import BlockMacro, BacksymbolMacro, MacroInfLoop,comp_thin
# pylint: disable-next = unused-import
from tm.rust_stuff import quick_term_or_rec  # noqa: F401

if TYPE_CHECKING:
    from typing import Self

    from tm.tape import Count
    from tm.parse import State, Slot, GetInstr

    Undfnd = tuple[int, Slot]

    Result = str | int | Undfnd


TERM_CATS = (
    'cfglim',
    'infrul',
    'limrul',
    'spnout',
    'undfnd',
    'xlimit',
)


def find_opt_macro(prog: str, steps: int) -> str | BlockMacro:
    if (blocks := opt_block(prog, steps)) > 1:
        return BlockMacro(prog, blocks)

    return prog

########################################

class Machine:
    program: str | BlockMacro | BacksymbolMacro
    comp: GetInstr

    tape: Tape
    prover: Prover

    steps: int
    cycles: int

    blanks: dict[State, int]

    spnout: int | None = None
    xlimit: int | None = None

    undfnd: Undfnd | None = None

    infrul: int | None = None

    cfglim: int | None = None
    limrul: str | None = None
    susrul: tuple[int, int] | None = None

    rulapp: Count = 0

    def __init__(
            self,
            program: str,
            *,
            blocks: int | None = None,
            backsym: int | None = None,
            opt_macro: int | None = None,
    ):
        prog: str | BlockMacro | BacksymbolMacro = program

        if opt_macro is not None:
            assert isinstance(prog, str)
            prog = find_opt_macro(prog, opt_macro)

        if blocks is not None:
            assert isinstance(prog, str)
            prog = BlockMacro(prog, blocks)

        if backsym is not None:
            assert isinstance(prog, str | BlockMacro)
            prog = BacksymbolMacro(prog, backsym)

        self.comp = (
            comp_thin(prog)
            if isinstance(prog, str) else
            prog
        )

        self.program = prog

    def __str__(self) -> str:
        info = [
            f"CYCLES: {self.cycles}",
            f"MARKS: {show_number(self.marks)}",
            *[
                "{}: {}".format(
                    cat.upper(),
                    data if isinstance(data, int|str|tuple) else "...")
                for cat in TERM_CATS
                # pylint: disable = bad-builtin
                if (data := getattr(self, cat, None)) is not None
            ],
            f"TPCFGS: {self.prover.config_count}",
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

    def show_tape(
            self,
            step: int,
            cycle: int,
            state: int,
    ) -> None:
        info = [
            f'{cycle: 5d}',
            show_slot((state, self.tape.scan)),
            str(self.tape),
        ]

        info.insert(1, f'{step : 3d}' if step != -1 else '...')

        print(' | '.join(info))

    @property
    def simple_termination(self) -> Count | None:
        return (
            self.spnout
            if (undfnd := self.undfnd) is None else
            undfnd[0]
        )

    @property
    def marks(self) -> Count:
        return self.tape.marks

    def run(
        self,
        sim_lim: int = 100_000_000,
        watch_tape: bool = False,
    ) -> Self:
        comp = self.comp

        self.tape = tape = Tape()

        self.prover = Prover(comp)

        self.blanks = {}

        step: int = 0

        state: State = 0

        for cycle in range(sim_lim):

            if watch_tape:
                self.show_tape(step, cycle, state)

            try:
                rule = self.prover.try_rule(cycle, state, tape)
            except InfiniteRule:
                self.infrul = step
                break
            except RuleLimit as lim:
                self.limrul = str(lim)
                break
            except ConfigLimit:  # no-cover
                self.cfglim = step
                break
            except SuspectedRule as sus:
                self.susrul = sus.args
                rule = None
            except MacroInfLoop:
                rule = None

            if rule is not None:
                try:
                    times = apply_rule(rule, tape)
                except RuleLimit as lim:
                    self.limrul = str(lim)
                    break

                if times is not None:
                    # print(f'--> applied rule: {rule}')
                    step = -1
                    self.rulapp += times
                    continue

            try:
                instr = comp[state, tape.scan]
            except KeyError as slot:
                self.undfnd = step, slot.args[0]
                break
            except MacroInfLoop:
                self.infrul = step
                break

            color, shift, next_state = instr

            if (same := state == next_state) and tape.at_edge(shift):
                self.spnout = step
                break

            stepped = tape.step(shift, color, same)

            if not isinstance(stepped, int):
                step = -1
            elif step != -1:
                step += stepped

            state = next_state

            if not color and tape.blank:
                if state in self.blanks:
                    self.infrul = step
                    break

                self.blanks[state] = step

                if state == 0:
                    self.infrul = step
                    break

        else:
            self.xlimit = step

        self.steps = step
        self.cycles = cycle

        if watch_tape and (bool(self.undfnd) or bool(self.blanks)):
            self.show_tape(step, 1 + cycle, state)

        return self
