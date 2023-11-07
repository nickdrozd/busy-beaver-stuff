from __future__ import annotations

from typing import TYPE_CHECKING

from tm.tape import Tape
from tm.parse import tcompile

if TYPE_CHECKING:
    from tm.parse import Color, Shift, State


class BlockMeasure(Tape):
    steps: int = 0
    max_blocks: int = 0
    max_blocks_step: int = 0

    def step(self, shift: Shift, color: Color, skip: bool) -> int:
        self.steps += 1

        if (blocks := self.blocks) > self.max_blocks:
            self.max_blocks = blocks
            self.max_blocks_step = self.steps

        return int(super().step(shift, color, skip))


def measure_blocks(prog: str, steps: int) -> int | None:
    comp = tcompile(prog)
    state: State = 0
    tape = BlockMeasure.init()

    for _ in range(steps):
        if (instr := comp[state, tape.scan]) is None:
            return None

        color, shift, next_state = instr

        if (same := state == next_state) and tape.at_edge(shift):
            return None

        _ = tape.step(shift, color, same)

        if (state := next_state) == -1:
            return None

    return tape.max_blocks_step


def compr_eff(tape: list[Color], k: int) -> int:
    compr_size = len(tape)

    for i in range(0, len(tape) - 2 * k, k):
        if tape[i : i + k] == tape[i + k : i + 2 * k]:
            compr_size -= k

    return compr_size
