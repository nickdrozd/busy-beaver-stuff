import argparse
import sys

from tm.rust_stuff import (
    cant_blank,
    cant_halt,
    cant_spinout,
    cant_twostep,
    ctl_cant_halt,
    segment_cant_halt,
)

CYCLES = 2_000
CANT_REACH = cant_spinout


if __name__ == '__main__':
    parser = argparse.ArgumentParser()

    parser.add_argument('--cycles', type = int)

    group = parser.add_mutually_exclusive_group()
    group.add_argument('--halt', action = 'store_true')
    group.add_argument('--blank', action = 'store_true')
    group.add_argument('--spinout', action = 'store_true')
    group.add_argument('--twostep', action = 'store_true')
    group.add_argument('--segment', action = 'store_true')
    group.add_argument('--ctl', action = 'store_true')

    args = parser.parse_args()

    if cycles := args.cycles:
        CYCLES = cycles

    if args.halt:
        CANT_REACH = cant_halt
    elif args.blank:
        CANT_REACH = cant_blank
    elif args.twostep:
        CANT_REACH = cant_twostep
    elif args.segment:
        CYCLES = 20
        CANT_REACH = segment_cant_halt  # type: ignore[assignment]
    elif args.ctl:
        CYCLES = 100

        for prog in sys.stdin:
            print(ctl_cant_halt(prog, 100))

        sys.exit()

    for prog in sys.stdin:
        print()
        print(prog)
        result = CANT_REACH(prog, CYCLES)
        print()
        print(result)
        print()
        print('||--------------------------------------')
