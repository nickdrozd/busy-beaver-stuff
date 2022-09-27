import sys
from multiprocessing import Pool

from analyze import Program

def worker(prog: str) -> None:
    for slot in (program := Program(prog)).slots[1:]:
        for ext in program.branch(slot):
            print(ext)

if __name__ == '__main__':
    with Pool() as pool:
        pool.map(worker, sys.stdin)
