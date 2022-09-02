import sys

from tm import Machine

PRINT = 1
STEPS = 10 ** 10
BLANK = 1
RCRNC = None
SKIP  = 1
TAPE  = 50

if __name__ == '__main__':
    for i, program in enumerate(sys.stdin):
        machine = Machine(program).run(
            tape = TAPE,
            skip = SKIP,
            sim_lim = STEPS,
            watch_tape = PRINT,
            check_rec = RCRNC,
            check_blanks = BLANK,
        )

        print(f'{i} | {machine}')
