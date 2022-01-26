from argparse import ArgumentParser

from generate.tree import run_tree_gen


if __name__ == '__main__':
    parser = ArgumentParser()

    parser.add_argument('states', type = int)
    parser.add_argument('colors', type = int)

    parser.add_argument('--halt', action = 'store_true')

    args = parser.parse_args()

    if halt := args.halt:
        def print_complete(prog):
            if prog.count('...') <= 1:
                print(prog.replace('...', '1RH'))
    else:
        def print_complete(prog):
            if '.' not in prog:
                print(prog)

    run_tree_gen(
        states = args.states,
        colors = args.colors,
        halt   = halt,
        output = print_complete,
    )
