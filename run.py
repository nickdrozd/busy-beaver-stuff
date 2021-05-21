import sys

from tm.run_bb import run_bb

CANDIDATES = [
    "1RB 1LA 1LA 1RB"
]

RCRNC = 0
STEPS = 100
BLANK = 0
PRINT = 1
STDIN = 0
TAPE  = 50

if __name__ == '__main__':
    source = sys.stdin if STDIN else CANDIDATES

    for i, program in enumerate(source):
        machine = run_bb(
            program,
            tape = TAPE,
            x_limit = STEPS,
            watch_tape = PRINT,
            check_rec = RCRNC,
            check_blanks = BLANK,
        )

        status, step, period = machine.final

        if status != 'XLIMIT':
            print(f'{i} | {machine.program} | {machine.final}')
