# pylint: disable = redefined-loop-name, redefined-variable-type
import sys

from tm import Machine
from tm import BlockMacro, BacksymbolMacro
from perf import profile

PRINT = 1
STEPS = 10 ** 10
PROVE = 1
BLOCK = None
BACKS = None

PROFILE = 0

def main() -> None:
    for i, program in enumerate(sys.stdin):
        program = program.strip()

        if BLOCK is not None:
            program = BlockMacro(program, BLOCK)

        if BACKS is not None:
            program = BacksymbolMacro(program, BACKS)

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
