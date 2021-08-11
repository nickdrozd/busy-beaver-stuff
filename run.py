import sys

from tm.run_bb import run_bb

CANDIDATES = [
    "1RB 1LB  1LB 1LA"
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

        res = machine.final

        reasons = [
            f'{reason}: {data}'
            for reason, data in
            {
                'BLANKS': res.blanks,
                'HALTED': res.halted,
                'LINREC': res.linrec,
                'QSIHLT': res.qsihlt,
            }.items()
            if data is not None
        ]

        if not res.xlimit:
            print(f'{i} | {res.prog} | {reasons}')
