from argparse import ArgumentParser
from typing import Union
from collections.abc import Iterator

from tm import Machine
from generate.tree import run_tree_gen
from analyze.macro import MacroProg
from analyze import Program, BlockMacro, BacksymbolMacro


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


def macro_variations(base: str) -> Iterator[Union[str, MacroProg]]:
    yield base

    for block in range(2, 8):
        yield (block_mac := BlockMacro(base, [block]))
        yield BacksymbolMacro(block_mac, [1])

    yield (back_1 := BacksymbolMacro(base, [1]))
    yield BacksymbolMacro(back_1, [1])


def run_and_print(prog: str) -> None:
    for macro in macro_variations(prog):
        if Machine(macro).run(
                sim_lim = 200,
                prover = True,
        ).xlimit is None:
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
