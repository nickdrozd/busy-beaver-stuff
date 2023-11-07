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

    @property
    def blocks(self) -> int:
        return len(self.lspan) + len(self.rspan)

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

        if (state := next_state) == -1:  # no-cover
            return None

    return tape.max_blocks_step


def unroll_tape(prog: str, steps: int) -> list[Color]:
    comp = tcompile(prog)
    state: State = 0
    tape = Tape.init()

    for _ in range(steps):
        assert (instr := comp[state, tape.scan]) is not None

        color, shift, next_state = instr

        _ = tape.step(shift, color, state == next_state)

        state = next_state

    return tape.unroll()


def compr_eff(tape: list[Color], k: int) -> int:
    compr_size = len(tape)

    for i in range(0, len(tape) - 2 * k, k):
        if tape[i : i + k] == tape[i + k : i + 2 * k]:
            compr_size -= k

    return compr_size


def opt_block(prog: str, steps: int) -> int:
    if (max_blocks_step := measure_blocks(prog, steps)) is None:
        return 1

    tape = unroll_tape(prog, max_blocks_step)

    opt_size = 1
    min_comp = 1 + len(tape)

    for block_size in range(1, len(tape) // 2):
        if (compr_size := compr_eff(tape, block_size)) < min_comp:
            min_comp = compr_size
            opt_size = block_size

    return opt_size
