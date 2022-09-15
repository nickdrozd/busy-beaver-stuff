import sys

from tm import Machine

PRINT = 1
STEPS = 10 ** 10
BLANK = 1
RCRNC = None
SKIP  = 1

PROFILE = 0

if __name__ == '__main__':
    if PROFILE:
        # pylint: disable = import-error
        import yappi

        yappi.set_clock_type('cpu')
        yappi.start()

    for i, program in enumerate(sys.stdin):
        machine = Machine(program).run(
            skip = SKIP,
            sim_lim = STEPS,
            watch_tape = PRINT,
            check_rec = RCRNC,
            check_blanks = BLANK,
        )

        print(f'{i} | {machine}')

    if PROFILE:
        stats = yappi.get_func_stats()
        stats.save('yappi.callgrind', type = 'callgrind')
