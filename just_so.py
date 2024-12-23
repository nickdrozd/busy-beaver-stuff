import sys
import argparse

from tm.reason import (
    cant_halt,
    cant_blank,
    cant_spin_out,

    segment_cant_halt,
)


CYCLES = 2_000
CANT_REACH = cant_spin_out


if __name__ == '__main__':
    parser = argparse.ArgumentParser()

    parser.add_argument('--cycles', type = int)

    group = parser.add_mutually_exclusive_group()
    group.add_argument('--halt', action = 'store_true')
    group.add_argument('--blank', action = 'store_true')
    group.add_argument('--spinout', action = 'store_true')
    group.add_argument('--segment', action = 'store_true')

    args = parser.parse_args()

    if cycles := args.cycles:
        CYCLES = cycles

    if args.halt:
        CANT_REACH = cant_halt
    elif args.blank:
        CANT_REACH = cant_blank
    elif args.segment:
        CYCLES = 20
        CANT_REACH = segment_cant_halt  # type: ignore[assignment]

    for prog in sys.stdin:
        if CANT_REACH(prog, CYCLES) is not None:
            continue

        print(prog.strip())
