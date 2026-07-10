import argparse
import sys

from tm.rust_stuff import (
    bkw_cant_blank,
    bkw_cant_halt,
    bkw_cant_spinout,
    bkw_cant_twostep,
    bkw_cant_zloop,
)

CYCLES = 2_000
CANT_REACH = bkw_cant_spinout


if __name__ == '__main__':
    parser = argparse.ArgumentParser()

    parser.add_argument('--cycles', type = int)

    group = parser.add_mutually_exclusive_group()
    group.add_argument('--halt', action = 'store_true')
    group.add_argument('--blank', action = 'store_true')
    group.add_argument('--zloop', action = 'store_true')
    group.add_argument('--spinout', action = 'store_true')
    group.add_argument('--twostep', action = 'store_true')

    args = parser.parse_args()

    if cycles := args.cycles:
        CYCLES = cycles

    if args.halt:
        CANT_REACH = bkw_cant_halt
    elif args.blank:
        CANT_REACH = bkw_cant_blank
    elif args.zloop:
        CANT_REACH = bkw_cant_zloop
    elif args.twostep:
        CANT_REACH = bkw_cant_twostep

    for prog in sys.stdin:
        print()
        print(prog)
        result = CANT_REACH(prog, CYCLES)
        print()
        print(result)
        print()
        print('||--------------------------------------')
