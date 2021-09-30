from argparse import ArgumentParser

from generate.tree import run_tree_gen


if __name__ == '__main__':
    parser = ArgumentParser()
    parser.add_argument('states', type=int)

    args = parser.parse_args()

    def print_complete(prog):
        if '.' not in prog:
            print(prog)

    run_tree_gen(
        states = args.states,
        output = print_complete,
    )
