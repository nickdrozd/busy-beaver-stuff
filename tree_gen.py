from __future__ import annotations

from argparse import ArgumentParser

from tm.reason import cant_halt, cant_spin_out
from tm.machine import Machine
from tm.lin_rec import LooseLinRecMachine

from tm.tree import run_tree_gen


def run_variations(
        prog: str,
        sim_lim: int,
        *,
        lin_rec: int = 50,
        block_steps: int = 1_000,
) -> Machine | None:
    if LooseLinRecMachine(prog).run(lin_rec).xlimit is None:
        return None

    machine = Machine(
        prog,
        opt_macro = block_steps,
    ).run(sim_lim = sim_lim)

    if machine.infrul:
        return None

    return machine


def run_print(prog: str) -> None:
    if (machine := run_variations(prog, 10_000)) is None:
        return

    if machine.simple_termination and machine.rulapp > 1_000:
        print(machine)
        return

    print(prog)


def filter_halt(prog: str) -> None:
    if cant_halt(prog):
        return

    run_print(prog)


def filter_spin_out(prog: str) -> None:
    if cant_spin_out(prog):
        return

    run_print(prog)


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
        output = filter_halt if args.halt else filter_spin_out,
        branches = BRANCHES,
    )
