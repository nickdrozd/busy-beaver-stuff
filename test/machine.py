from __future__ import annotations

from enum import Enum
from typing import TYPE_CHECKING
from dataclasses import dataclass

from tm.machine import tcompile, Tape

if TYPE_CHECKING:
    from tm.machine import State, Slot


class TermRes(Enum):
    # pylint: disable = invalid-name
    xlimit = 0
    infrul = 1
    spnout = 2
    halted = 3
    undfnd = 4


@dataclass
class QuickMachineResult:
    result: TermRes

    steps: int
    cycles: int
    marks: int

    last_slot: Slot | None

    blanks: dict[State, int]

    @property
    def simple_termination(self) -> int | None:
        return (
            self.steps
            if self.result in {TermRes.halted, TermRes.spnout} else
            None
        )

    @property
    def undfnd(self) -> tuple[int, Slot] | None:
        if self.result != TermRes.undfnd:
            return None

        assert (last_slot := self.last_slot) is not None

        return self.steps, last_slot

    @property
    def halted(self) -> int | None:
        return (
            self.steps
            if self.result == TermRes.halted else
            None
        )

    @property
    def infrul(self) -> int | None:
        return (
            self.steps
            if self.result == TermRes.infrul else
            None
        )

    @property
    def spnout(self) -> int | None:
        return (
            self.steps
            if self.result == TermRes.spnout else
            None
        )

    @property
    def xlimit(self) -> int | None:
        return (
            self.steps
            if self.result == TermRes.xlimit else
            None
        )


def run_quick_machine(
        program: str,
        sim_lim: int = 100_000_000,
) -> QuickMachineResult:
    comp = tcompile(program)

    tape = Tape()

    blanks = {}

    step: int = 0

    state: State = 0

    result: TermRes | None = None
    last_slot: Slot | None = None

    for cycle in range(sim_lim):
        try:
            instr = comp[state, tape.scan]
        except KeyError:
            last_slot = state, tape.scan
            result = TermRes.undfnd
            break

        color, shift, next_state = instr

        if (same := state == next_state) and tape.at_edge(shift):
            result = TermRes.spnout
            break

        stepped = tape.step(shift, color, same)

        assert isinstance(stepped, int)

        step += stepped

        if (state := next_state) == -1:
            result = TermRes.halted
            break

        if not color and tape.blank:
            if state in blanks:
                result = TermRes.infrul
                break

            blanks[state] = step

            if state == 0:
                result = TermRes.infrul
                break

    else:
        result = TermRes.xlimit

    assert isinstance(marks := tape.marks, int)

    return QuickMachineResult(
        result = result,
        steps = step,
        cycles = cycle,
        marks = marks,
        last_slot = last_slot,
        blanks = blanks,
    )
