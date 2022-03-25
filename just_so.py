import sys
from multiprocessing import Pool

from generate import Program

def worker(prog):
    if (program := Program(prog)).cant_spin_out:
        return

    print(program)

if __name__ == '__main__':
    with Pool() as pool:
        pool.map(worker, sys.stdin)
