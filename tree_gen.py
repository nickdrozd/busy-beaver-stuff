from __future__ import annotations

from typing import TYPE_CHECKING
from argparse import ArgumentParser

from tm.reason import cant_halt, cant_spin_out
from tm.machine import run_variations

from generate.tree import run_tree_gen

if TYPE_CHECKING:
    from generate.tree import Output


def filter_run_print(halt: bool) -> Output:
    cant_reach = cant_halt if halt else cant_spin_out

    def drop(prog: str) -> None:
        if cant_reach(prog):
            return

        for machine in run_variations(prog, 10_000):
            if machine.simple_termination and machine.rulapp > 1_000:
                print(machine)
                return

            if machine.xlimit is None:
                return

        print(prog)

    return drop


if __name__ == '__main__':
    parser = ArgumentParser()

    parser.add_argument('states', type = int)
    parser.add_argument('colors', type = int)

    parser.add_argument('--steps', type = int, default = 200)

    parser.add_argument('--halt', action = 'store_true')

    parser.add_argument('--progfile', type = str)

    args = parser.parse_args()

    if (progfile := args.progfile) is None:
        BRANCHES = None
    else:
        with open(progfile) as progs:
            BRANCHES = [prog.strip() for prog in progs.readlines()]

    run_tree_gen(
        states = args.states,
        colors = args.colors,
        halt   = args.halt,
        steps  = args.steps,
        output = filter_run_print(args.halt),
        branches = BRANCHES,
    )
