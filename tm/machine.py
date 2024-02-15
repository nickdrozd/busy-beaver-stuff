from __future__ import annotations

from typing import TYPE_CHECKING

from tm.tape import Tape, init_stepped, show_number
from tm.blocks import opt_block
from tm.prover import Prover, ConfigLimit
from tm.show import show_slot
from tm.rules import RuleLimit, InfiniteRule, SuspectedRule
from tm.macro import BlockMacro, BacksymbolMacro, MacroInfLoop, tcompile

if TYPE_CHECKING:
    from typing import Self

    from tm.tape import Count, HeadTape
    from tm.parse import State, Slot, GetInstr

    Undfnd = tuple[int, Slot]

    Result = str | int | Undfnd


TERM_CATS = (
    'cfglim',
    'halted',
    'infrul',
    'limrul',
    'spnout',
    'undfnd',
    'xlimit',
)


def find_opt_macro(prog: str, steps: int) -> str | GetInstr:
    if (blocks := opt_block(prog, steps)) > 1:
        return BlockMacro(prog, [blocks])

    return prog

########################################

class Machine:
    program: str | GetInstr
    comp: GetInstr

    tape: Tape
    prover: Prover

    steps: int
    cycles: int

    blanks: dict[State, int]

    halted: int | None = None
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
            except AttributeError:  # no-cover
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
                data if isinstance(data, int | str | tuple) else "...")
            for cat, data in self.term_results
        ]

        info.append(
            f'TPCFGS: {self.prover.config_count}')

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
        return self.spnout if self.halted is None else self.halted

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
                    times = tape.apply_rule(rule)
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

            if (state := next_state) == -1:
                self.halted = step
                break

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

        if watch_tape and (bool(self.halted) or bool(self.blanks)):
            self.show_tape(step, 1 + cycle, state)

        return self

########################################

def quick_term_or_rec(prog: str, sim_lim: int) -> bool:
    # pylint: disable = while-used, too-many-locals

    comp = tcompile(prog)

    state = 1

    tape: HeadTape = init_stepped()

    step, cycle = 1, 1

    while cycle < sim_lim:
        steps_reset = 2 * step

        leftmost = rightmost = tape.head

        init_state = state

        init_tape = tape.copy()

        while step < steps_reset and cycle < sim_lim:
            try:
                instr = comp[state, tape.scan]
            except KeyError:
                return False

            color, shift, next_state = instr

            if (same := state == next_state) and tape.at_edge(shift):
                return True

            stepped = tape.step(shift, color, same)

            step += stepped

            cycle += 1

            if (state := next_state) == -1:
                return True

            if (curr := tape.head) < leftmost:
                leftmost = curr
            elif rightmost < curr:
                rightmost = curr

            if state != init_state:
                continue

            if tape.scan != init_tape.scan:
                continue

            if tape.aligns_with(init_tape, leftmost, rightmost):
                return True

    return False
