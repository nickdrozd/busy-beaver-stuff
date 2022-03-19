import sys
from multiprocessing import Pool

# pylint: disable = import-error
from py_lin_rado_turing.tools import run_bb

STEPS = 10_000
PERIOD = 1_000

def shear(prog):
    result = run_bb(
        prog.strip(),
        check_rec = STEPS,
        x_limit = STEPS + PERIOD,
    )

    if result.final.xlimit is not None:
        print(prog.strip())

if __name__ == '__main__':
    with Pool() as pool:
        pool.map(shear, sys.stdin)
