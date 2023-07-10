import sys
from multiprocessing import Pool

from tm.reason import BackwardReasoner

def worker(prog: str) -> None:
    if (program := BackwardReasoner(prog)).cant_spin_out:
        return

    print(program)

if __name__ == '__main__':
    with Pool() as pool:
        pool.map(worker, sys.stdin)
