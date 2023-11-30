from __future__ import annotations

from typing import TYPE_CHECKING

from tm.machine import tcompile, Tape

if TYPE_CHECKING:
    from typing import Self

    from tm.machine import Count, State, GetInstr, Undfnd


class QuickMachine:
    program: str | GetInstr
    comp: GetInstr

    tape: Tape
    steps: int
    cycles: int

    blanks: dict[State, int]

    halted: int | None = None
    spnout: int | None = None
    xlimit: int | None = None

    undfnd: Undfnd | None = None

    infrul: int | None = None

    def __init__(self, program: str | GetInstr):
        self.program = program

        self.comp = (
            tcompile(self.program)
            if isinstance(self.program, str) else
            self.program
        )

    @property
    def simple_termination(self) -> Count | None:
        return self.spnout if self.halted is None else self.halted

    @property
    def marks(self) -> Count:
        return self.tape.marks

    def run(self, sim_lim: int = 100_000_000) -> Self:
        comp = self.comp

        self.tape = tape = Tape.init()

        self.blanks = {}

        step: int = 0

        state: State = 0

        for cycle in range(sim_lim):  # no-branch

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

            if (state := next_state) == -1:  # no-cover
                self.halted = step
                break

            if not color and tape.blank:
                if state in self.blanks:
                    self.infrul = step
                    break

                self.blanks[state] = step

                if state == 0:  # no-cover
                    self.infrul = step
                    break

        else:
            self.xlimit = step  # no-cover

        self.steps = step
        self.cycles = cycle

        return self
