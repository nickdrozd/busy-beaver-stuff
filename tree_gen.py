from argparse import ArgumentParser

from tm.program import Program
from tm.utils import run_variations
from generate.tree import run_tree_gen


def prune_print(prog: str) -> None:
    if not HALT and '.' in prog:
        return

    if COLORS == 2 and prog.count('0') < 2:
        return

    if not (program := Program(prog)).graph.is_strongly_connected:
        return

    if not program.graph.is_simple:
        return

    if not HALT and program.cant_spin_out:
        return

    run_and_print(prog)


def print_complete(prog: str) -> None:
    if len(set(Program(prog).used_states)) != STATES:
        return

    run_and_print(prog)


def run_and_print(prog: str) -> None:
    if any(machine.xlimit is None for machine in
           run_variations(prog, 200, 8, 1)):
        return

    print(Program(prog).normalize())


if __name__ == '__main__':
    parser = ArgumentParser()

    parser.add_argument('states', type = int)
    parser.add_argument('colors', type = int)

    parser.add_argument('--steps', type = int, default = 200)

    parser.add_argument('--halt', action = 'store_true')
    parser.add_argument('--aggressive', action = 'store_true')

    args = parser.parse_args()

    HALT   = args.halt
    STEPS  = args.steps
    COLORS = args.colors
    STATES = args.states

    run_tree_gen(
        states = STATES,
        colors = COLORS,
        halt   = HALT,
        steps = STEPS,
        blank = True,
        output = (
            prune_print
            if args.aggressive else
            print_complete
        ),
    )
