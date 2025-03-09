import argparse
import sys

from tm.machine import Machine


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()

    parser.add_argument(
        "-p", "--no-print",
        action = "store_false",
        dest = "print",
        default = True,
    )

    parser.add_argument(
        "-s", "--steps",
        type = int,
        default = 10 ** 6,
    )

    parser.add_argument(
        "-b", "--backsym",
        type = int,
        default = None,
    )

    parser.add_argument(
        "-m", "--macro",
        type = int,
        default = 8_000,
    )

    return parser.parse_args()


if __name__ == '__main__':
    sys.set_int_max_str_digits(100_000)

    args = parse_args()

    for i, prog in enumerate(p.strip() for p in sys.stdin):
        machine = Machine(
            prog,
            opt_macro = args.macro,
            backsym = args.backsym,
        ).run(
            sim_lim = args.steps,
            watch_tape = args.print,
        )

        print(f"{i} | {machine}")
