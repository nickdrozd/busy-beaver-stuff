from __future__ import annotations

from typing import TYPE_CHECKING

from tm.reason import Program, HeadTape

if TYPE_CHECKING:
    from tm.machine import Slot, Undfnd

    InstrSeq = list[tuple[str, int, Slot]]


def instr_seq(prog: str) -> InstrSeq:
    program = Program(prog)

    seqs: InstrSeq = []

    partial = Program.init(len(program.states), len(program.colors))

    for _ in range(len(program.states) * len(program.colors) - 1):
        if (result := run_for_undefined(partial)) is None:
            return seqs

        step, slot = result

        seqs.append((str(partial), step, slot))

        partial[slot] = program[slot]

    return seqs


def run_for_undefined(prog: Program) -> Undfnd | None:
    tape = HeadTape()

    step = 0

    state = 0

    for _ in range(100_000_000):  # no-branch
        if (instr := prog[state, tape.scan]) is None:
            return step, (state, tape.scan)

        color, shift, next_state = instr

        if (same := state == next_state) and tape.at_edge(shift):
            return None

        stepped = tape.step(shift, color, same)

        assert isinstance(stepped, int)

        step += stepped

        if (state := next_state) == -1:  # no-cover
            return None

    return None  # no-cover
