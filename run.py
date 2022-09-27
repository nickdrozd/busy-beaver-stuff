import sys

from tm import Machine
from analyze import BlockMacro

PRINT = 1
STEPS = 10 ** 10
RCRNC = None
MACRO = None

PROFILE = 0

if __name__ == '__main__':
    if bool(PROFILE):
        # pylint: disable = import-error
        import yappi  # type: ignore

        yappi.set_clock_type('cpu')
        yappi.start()

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

    if bool(PROFILE):
        stats = yappi.get_func_stats()
        stats.save('yappi.callgrind', type = 'callgrind')
