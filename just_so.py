import sys
from multiprocessing import Pool

from tm.reason import cant_spin_out

def worker(prog: str) -> None:
    if cant_spin_out(prog):
        return

    print(prog)

if __name__ == '__main__':
    with Pool() as pool:
        pool.map(worker, sys.stdin)
