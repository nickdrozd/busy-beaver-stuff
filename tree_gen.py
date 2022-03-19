from argparse import ArgumentParser

from generate import Program
from generate.tree import run_tree_gen

if (USE_RUST := 0):
    # pylint: disable = import-error
    import py_lin_rado_turing.tools as rust
else:
    from tm import Machine
    rust = None  # pylint: disable = invalid-name


def prune_print(prog: str):
    if '.' in prog:
        return

    if COLORS == 2 and prog.count('0') < 2:
        return

    if not (program := Program(prog)).graph.is_strongly_connected:
        return

    if not program.graph.is_simple:
        return

    if not HALT and program.cant_spin_out:
        return

    run_and_print(program)


def print_complete(prog: str):
    if len(set((program := Program(prog)).used_states)) != STATES:
        return

    run_and_print(program)


def run_and_print(program: Program):
    check_rec = (
        Machine(program).run(
            sim_lim = 100,
            check_rec = 0,
        )
        if rust is None else
        rust.run_bb(
            str(program),
            x_limit = 100,
            check_rec = 0,
        )
    )

    if check_rec.final.xlimit is None:
        return

    print(program.normalize())


if __name__ == '__main__':
    parser = ArgumentParser()

    parser.add_argument('states', type = int)
    parser.add_argument('colors', type = int)

    parser.add_argument('--halt', action = 'store_true')
    parser.add_argument('--aggressive', action = 'store_true')

    args = parser.parse_args()

    HALT = args.halt
    COLORS = args.colors
    STATES = args.states

    run_tree_gen(
        states = STATES,
        colors = COLORS,
        halt   = HALT,
        output = (
            prune_print
            if args.aggressive else
            print_complete
        ),
    )
