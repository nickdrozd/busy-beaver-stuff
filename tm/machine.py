from __future__ import annotations

from typing import TYPE_CHECKING

from tm.macro import MacroInfLoop, make_macro
from tm.prover import ConfigLimit, Prover
from tm.rules import InfiniteRule, RuleLimit, SuspectedRule, apply_rule
from tm.rust_stuff import (
    py_quick_term_or_rec as quick_term_or_rec,  # noqa: F401
)
from tm.show import show_comp, show_slot, show_state
from tm.tape import Tape, show_number

if TYPE_CHECKING:
    from typing import Final, Self

    from tm.macro import GetInstr, Params, Slot, State
    from tm.tape import Count

    Undfnd = tuple[int, Slot]

    Result = str | int | Undfnd


TERM_CATS: Final[tuple[str, ...]] = (
    'cfglim',
    'infrul',
    'limrul',
    'spnout',
    'undfnd',
    'xlimit',
)

########################################

class Machine:
    program: GetInstr

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
            prog: str,
            *,
            blocks: int | None = None,
            backsym: int | None = None,
            opt_macro: int | None = None,
            params: Params | None = None,
    ):
        self.program = make_macro(
            prog,
            blocks = blocks,
            backsym = backsym,
            opt_macro = opt_macro,
            params = params,
        )

    @property
    def prog_str(self) -> str:
        return (
            show_comp(comp)
            if isinstance(comp := self.program, dict) else
            str(comp)
        )

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

        if rulapp := self.rulapp:
            rulapp_disp = (
                show_number(rulapp)
                if isinstance(rulapp, int) else
                rulapp.estimate()
            )

            info.append(
                f'RULAPP: {rulapp_disp}')

        if self.blanks:
            blanks = {
                show_state(state): step
                for state, step in self.blanks.items()
            }

            info.append(
                f'BLANKS: {blanks}')

        return f"{self.prog_str} || {' | '.join(info)}"

    def config_str(
            self,
            step: int,
            cycle: int,
            state: int,
    ) -> str:
        info = [
            f'{cycle: 5d}',
            show_slot((state, self.tape.scan)),
            str(self.tape),
        ]

        info.insert(1, f'{step : 3d}' if step != -1 else '...')

        return ' | '.join(info)

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

    @property
    def is_algebraic(self) -> bool:
        return (
            not isinstance(self.rulapp, int)
            or not isinstance(self.marks, int)
        )

    def run(
        self,
        sim_lim: int = 100_000_000,
        watch_tape: bool = False,  # noqa: FBT001, FBT002
    ) -> Self:
        comp = self.program

        self.tape = tape = Tape()

        self.prover = Prover(comp)

        self.blanks = {}

        step: int = 0

        state: State = 0

        for cycle in range(sim_lim):

            if watch_tape:
                print(self.config_str(step, cycle, state))

            try:
                rule = self.prover.try_rule(cycle, state, tape)
            except InfiniteRule:
                self.infrul = step
                break
            except RuleLimit as lim:
                self.limrul = str(lim)
                break
            except NotImplementedError as nie:
                self.limrul = str(nie)
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

            if step != -1:
                assert isinstance(stepped, int)
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
            print(self.config_str(step, 1 + cycle, state))

        return self
