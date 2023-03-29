from collections.abc import Iterator

from tm.tape import Tape
from tm.instrs import Color, Shift
from tm.machine import Machine, LinRecMachine
from tm.macro import BlockMacro, BacksymbolMacro, MacroProg


def macro_variations(
        prog: str,
        block_steps: int,
) -> Iterator[str | MacroProg]:
    yield (
        prog
        if (opt := opt_block(prog, block_steps)) == 1 else
        BlockMacro(prog, [opt])
    )

    yield BacksymbolMacro(prog, [1])


def run_variations(
        prog: str,
        sim_lim: int,
        *,
        lin_rec: int = 50,
        block_steps: int = 1_000,
) -> Iterator[Machine]:
    yield LinRecMachine(prog).run(
        step_lim = lin_rec,
        check_rec = 0,
        skip = True,
    )

    for macro in macro_variations(prog, block_steps):
        yield Machine(macro).run(
            sim_lim = sim_lim,
            prover = True,
        )


class BlockMeasure(Tape):
    steps: int = 0
    max_blocks: int = 0
    max_blocks_step: int = 0

    def step(self, shift: Shift, color: Color, skip: bool) -> int:
        self.steps += 1

        blocks = len(self.lspan) + len(self.rspan)

        if blocks > self.max_blocks:
            self.max_blocks = blocks
            self.max_blocks_step = self.steps

        return super().step(shift, color, skip)


def opt_block(prog: str, steps: int) -> int:
    machine = Machine(prog).run(
        sim_lim = steps,
        tape = BlockMeasure([], 0, []))

    if machine.xlimit is None:
        return 1

    tape = Machine(prog).run(
        # pylint: disable = line-too-long
        sim_lim = machine.tape.max_blocks_step,  # type: ignore[attr-defined]
    ).tape.to_ptr().tape

    opt_size = 1
    min_comp = 1 + len(tape)

    for block_size in range(1, len(tape) // 2):
        if (compr_size := compr_eff(tape, block_size)) < min_comp:
            min_comp = compr_size
            opt_size = block_size

    return opt_size


def compr_eff(tape: list[Color], k: int) -> int:
    compr_size = len(tape)

    for i in range(0, len(tape) - 2 * k, k):
        if tape[i : i + k] == tape[i + k : i + 2 * k]:
            compr_size -= k

    return compr_size
