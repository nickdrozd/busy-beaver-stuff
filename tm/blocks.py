from __future__ import annotations

from typing import TYPE_CHECKING

from tm.rust_stuff import measure_blocks, unroll_tape

if TYPE_CHECKING:
    from tm.parse import Color


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
