import sys
import argparse
from multiprocessing import Pool

from tm.reason import cant_halt, cant_blank, cant_spin_out


CANT_REACH = cant_spin_out


def worker(prog: str) -> None:
    if CANT_REACH(prog):
        return

    print(prog)


if __name__ == '__main__':
    parser = argparse.ArgumentParser()

    group = parser.add_mutually_exclusive_group()
    group.add_argument('--halt', action = 'store_true')
    group.add_argument('--blank', action = 'store_true')
    group.add_argument('--spinout', action = 'store_true')

    args = parser.parse_args()

    if args.halt:
        CANT_REACH = cant_halt
    elif args.blank:
        CANT_REACH = cant_blank

    with Pool() as pool:
        pool.map(worker, sys.stdin)
