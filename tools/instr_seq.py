from __future__ import annotations

from typing import TYPE_CHECKING

from tm.tape import Tape
from tm.program import Program, init_prog

if TYPE_CHECKING:
    from tm.machine import Slot, Undfnd

    InstrSeq = list[tuple[str, int, Slot]]


def instr_seq(prog: str) -> InstrSeq:
    program = Program(prog)

    seqs: InstrSeq = []

    partial = init_prog(
        states := len(program.states),
        colors := len(program.colors))

    for _ in range(states * colors - 1):  # no-branch
        if (result := run_for_undefined(partial)) is None:
            break

        step, slot = result

        seqs.append((str(partial), step, slot))

        try:
            partial[slot] = program[slot]
        except KeyError:
            break

    return seqs


def run_for_undefined(prog: Program) -> Undfnd | None:
    tape = Tape()

    step = 0

    state = 0

    for _ in range(100_000_000):  # no-branch
        try:
            instr = prog[state, tape.scan]
        except KeyError:
            return step, (state, tape.scan)

        color, shift, next_state = instr

        if (same := state == next_state) and tape.at_edge(shift):
            return None

        stepped = tape.step(shift, color, same)

        assert isinstance(stepped, int)

        step += stepped

        state = next_state

    return None  # no-cover
