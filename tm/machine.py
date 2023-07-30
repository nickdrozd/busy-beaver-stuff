from __future__ import annotations

from typing import TYPE_CHECKING

from tm.prover import Prover
from tm.lin_rec import History, HeadTape
from tm.tape import Tape, BlockMeasure, compr_eff
from tm.show import show_slot, show_number
from tm.rules import RuleLimit, InfiniteRule
from tm.macro import BlockMacro, BacksymbolMacro, tcompile

if TYPE_CHECKING:
    from typing import Self
    from collections.abc import Iterator

    from tm.tape import Count
    from tm.parse import State, Slot, GetInstr
    from tm.lin_rec import RecRes, Tapes

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
    cfglim: bool | None = None
    limrul: bool | None = None

    rulapp: int = 0

    def __init__(self, program: str | GetInstr):
        self.program = program

        self.comp = (
            tcompile(self.program)
            if isinstance(self.program, str) else
            self.program
        )

    @property
    def term_results(self) -> tuple[tuple[str, Result], ...]:
        return tuple(
            (cat, data)
            for cat in TERM_CATS
            # pylint: disable = bad-builtin
            if (data := getattr(self, cat)) is not None
        )

    def __str__(self) -> str:
        info = [ f'CYCLES: {self.cycles}' ]

        info.append(
            f'MARKS: {show_number(self.marks)}')

        info += [
            f'{cat.upper()}: {data if self.rulapp == 0 else "..."}'
            for cat, data in self.term_results
        ]

        if self.rulapp > 0:
            info.append(
                f'RULAPP: {show_number(self.rulapp)}')

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

        step: int | None = 0

        for cycle in range(sim_lim):

            if (instr := comp[state, tape.scan]) is None:
                self.undfnd = step or -1, (state, tape.scan)
                break

            color, shift, next_state = instr

            if (same := state == next_state) and tape.at_edge(shift):
                self.spnout = step or -1
                break

            stepped = tape.step(shift, color, same)

            if step is not None:
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

        return self

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

        # assert len(results := self.term_results) == 1, results


class Machine(BasicMachine):
    prover: Prover

    def __init__(
            self,
            program: str | GetInstr,
            *,
            blocks: int | list[int] | None = None,
            backsym: int | list[int] | None = None,
            opt_blocks: int | None = None,
    ):
        if opt_blocks is not None:
            blocks = (
                opt
                if (opt := opt_block(program, opt_blocks)) > 1 else
                None
            )

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
        # pylint: disable = line-too-long
        return f'{super().__str__()} | TPCFGS: {self.prover.config_count}'

    def run(self,
            sim_lim: int = 100_000_000,
            *,
            watch_tape: bool = False,
            state: State = 0,
            tape: Tape | None = None,
    ) -> Self:
        comp = self.comp

        if tape is None:
            tape = Tape.init()

        self.tape = tape

        self.prover = Prover(comp)

        self.blanks = {}

        step: int | None = 0

        for cycle in range(sim_lim):

            if watch_tape:
                self.show_tape(step, cycle, state)

            try:
                rule = self.prover.try_rule(cycle, state, tape)
            except InfiniteRule:
                self.infrul = True
                break

            if rule is not None:
                try:
                    times = tape.apply_rule(rule)
                except RuleLimit:
                    self.limrul = True
                    break

                if times is not None:
                    # print(f'--> applied rule: {rule}')
                    step = None
                    self.rulapp += times
                    continue

            if self.prover.config_count > 100_000:
                self.cfglim = True
                break

            if (instr := comp[state, tape.scan]) is None:
                self.undfnd = step or -1, (state, tape.scan)
                break

            color, shift, next_state = instr

            if (same := state == next_state) and tape.at_edge(shift):
                self.spnout = step or -1
                break

            stepped = tape.step(shift, color, same)

            if step is not None:
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

class LinRecMachine(BasicMachine):
    history: History

    def run(  # type: ignore[override]  # pylint: disable = arguments-differ
        self,
        sim_lim: int | None = None,
        *,
        skip: bool = False,
        check_rec: int | None = None,
        samples: Tapes | None = None,
    ) -> Self:
        assert (
            check_rec is not None
            or samples is not None)

        self.blanks = {}

        comp = self.comp

        self.tape = tape = HeadTape.init()

        self.history = History(tapes = samples or {})

        step: int = 0
        state: State = 0

        for cycle in range(sim_lim or 1_000_000):
            self.history.add_state_at_step(step, state)

            slot: Slot = state, tape.scan

            if ((check_rec is not None and step >= check_rec)
                or (samples is not None
                   and step in self.history.tapes)):
                self.history.add_tape_at_step(step, tape)

            if check_rec is not None and step >= check_rec:
                if self.check_rec(step, slot) is not None:
                    break

                self.history.add_slot_at_step(step, slot)

            if (instr := comp[slot]) is None:
                self.undfnd = step, slot
                break

            color, shift, next_state = instr

            step += tape.step(
                shift, color, skip and state == next_state)

            if (state := next_state) == -1:  # no-coverage
                self.halted = step
                break

            if not color and tape.blank and state not in self.blanks:
                self.blanks[state] = step

        else:
            self.xlimit = step

        self.cycles = cycle

        return self

    def check_rec(self, step: int, slot: Slot) -> RecRes | None:
        if (result := self.history.check_rec(step, slot)) is None:
            return None

        self.linrec = start, rec = result

        if rec == 1:
            self.spnout = step - 1

        hc_beeps = self.history.calculate_beeps()
        hp_beeps = self.history.calculate_beeps(start)

        self.qsihlt = any(
            hc_beeps[st] <= hp_beeps[st]
            for st in hp_beeps
        )

        return result

########################################

def run_variations(
        prog: str,
        sim_lim: int,
        *,
        lin_rec: int = 50,
        block_steps: int = 1_000,
) -> Iterator[BasicMachine]:
    yield LinRecMachine(prog).run(
        sim_lim = lin_rec,
        check_rec = 0,
        skip = True,
    )

    yield Machine(
        prog,
        opt_blocks = block_steps,
    ).run(
        sim_lim = sim_lim,
    )

    yield Machine(
        prog,
        backsym = 1,
    ).run(
        sim_lim = sim_lim,
    )


def opt_block(prog: str | GetInstr, steps: int) -> int:
    machine = BasicMachine(prog).run(
        sim_lim = steps,
        tape = BlockMeasure.init())

    if machine.xlimit is None:
        return 1

    tape = machine.run(
        # pylint: disable = line-too-long
        sim_lim = machine.tape.max_blocks_step,  # type: ignore[attr-defined]
    ).tape.unroll()

    opt_size = 1
    min_comp = 1 + len(tape)

    for block_size in range(1, len(tape) // 2):
        if (compr_size := compr_eff(tape, block_size)) < min_comp:
            min_comp = compr_size
            opt_size = block_size

    return opt_size
