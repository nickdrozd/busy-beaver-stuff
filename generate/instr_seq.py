from __future__ import annotations

from typing import TYPE_CHECKING

from tm.program import Program
from tm.machine import QuickMachine

if TYPE_CHECKING:
    from tm.program import Slot

    InstrSeq = list[tuple[str, int, Slot]]


def instr_seq(prog: str) -> InstrSeq:
    program = Program(prog)

    seqs: InstrSeq = []

    partial = Program.init(len(program.states), len(program.colors))

    machine = QuickMachine(partial)

    for _ in range(len(program.states) * len(program.colors) - 1):
        if (result := machine.run().undfnd) is None:
            return seqs

        step, slot = result

        seqs.append((str(partial), step, slot))

        partial[slot] = program[slot]

        machine.undfnd = None

    return seqs
