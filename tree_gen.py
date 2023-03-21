from argparse import ArgumentParser

from generate.tree import run_tree_gen


if __name__ == '__main__':
    parser = ArgumentParser()

    parser.add_argument('states', type = int)
    parser.add_argument('colors', type = int)

    parser.add_argument('--steps', type = int, default = 200)

    parser.add_argument('--halt', action = 'store_true')

    args = parser.parse_args()

    run_tree_gen(
        states = args.states,
        colors = args.colors,
        halt   = args.halt,
        steps  = args.steps,
        output = print,
    )
