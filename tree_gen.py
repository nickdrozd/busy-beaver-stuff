from argparse import ArgumentParser

from analyze import Program
from generate.tree import run_tree_gen
from tm import Machine


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

    run_and_print(program)


def print_complete(prog: str) -> None:
    if len(set((program := Program(prog)).used_states)) != STATES:
        return

    run_and_print(program)


def run_and_print(program: Program) -> None:
    run_proof = Machine(program).run(
        sim_lim = 200,
        prover = True,
    )

    if run_proof.xlimit is None:
        return

    check_rec = Machine(program).run(
        sim_lim = 100,
        check_rec = 0,
    )

    if check_rec.xlimit is None:
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

    try:
        STEPS = {
            (2, 2): 40,
            (3, 2): 126,
            (2, 3): 223,  # 220
        }[(STATES, COLORS)]
    except KeyError:
        STEPS = 500

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
