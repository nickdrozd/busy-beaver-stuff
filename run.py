# pylint: disable = redefined-loop-name, redefined-variable-type
import sys

from perf import profile
from tm.utils import opt_block
from tm.machine import Machine
from tm.macro import BlockMacro, BacksymbolMacro

PRINT = 1
STEPS = 10 ** 10
PROVE = 1
BACKS = 0

PROFILE = 0

def main() -> None:
    for i, program in enumerate(sys.stdin):
        program = program.strip()

        if (block := opt_block(program, steps = 8_000)) > 1:
            print(f'block size: {block}')
            program = BlockMacro(
                program, [block])  # type: ignore[assignment]

        if BACKS > 0:
            program = BacksymbolMacro(
                program, [BACKS])  # type: ignore[assignment]

        machine = Machine(program).run(
            sim_lim = STEPS,
            watch_tape = bool(PRINT),
            prover = bool(PROVE),
        )

        print(f'{i} | {machine}')

if __name__ == '__main__':
    if PROFILE:
        PRINT = 0
        main = profile(main)

    sys.set_int_max_str_digits(100_000)

    main()
