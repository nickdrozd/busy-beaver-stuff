import sys

from tm import Machine

CANDIDATES = [
    "1RB 1LB  1LB 1LA"
]

RCRNC = 0
STEPS = 100
BLANK = 1
PRINT = 1
STDIN = 1
TAPE  = 50

if __name__ == '__main__':
    source = sys.stdin if STDIN else CANDIDATES

    for i, program in enumerate(source):
        machine = Machine(program).run(
            tape = TAPE,
            sim_lim = STEPS,
            watch_tape = PRINT,
            check_rec = RCRNC,
            check_blanks = BLANK,
        )

        print(f'{i} | {machine}')
