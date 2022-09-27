import sys

from tm import Machine
from analyze import BlockMacro
from perf import profile

PRINT = 1
STEPS = 10 ** 10
RCRNC = None
MACRO = None

PROFILE = 0

def main() -> None:
    for i, program in enumerate(sys.stdin):
        # pylint: disable = redefined-loop-name
        program = program.strip()

        if MACRO is not None:
            program = BlockMacro(program, MACRO)

        machine = Machine(program).run(
            sim_lim = STEPS,
            watch_tape = bool(PRINT),
            check_rec = RCRNC,
        )

        print(f'{i} | {machine}')

if __name__ == '__main__':
    if PROFILE:
        PRINT = 0
        main = profile(main)

    main()
