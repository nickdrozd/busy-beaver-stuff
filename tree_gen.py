from argparse import ArgumentParser

from generate.tree import run_tree_gen
from generate.program import Program


if __name__ == '__main__':
    parser = ArgumentParser()

    parser.add_argument('states', type = int)
    parser.add_argument('colors', type = int)

    parser.add_argument('--halt', action = 'store_true')

    args = parser.parse_args()

    states = args.states

    def print_complete(prog):
        if len(set((program := Program(prog)).used_states)) == states:
            print(program.normalize())

    run_tree_gen(
        states = states,
        colors = args.colors,
        halt   = args.halt,
        output = print_complete,
    )
