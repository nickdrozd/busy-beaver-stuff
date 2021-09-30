from argparse import ArgumentParser

from generate.tree import run_tree_gen


if __name__ == '__main__':
    parser = ArgumentParser()
    parser.add_argument('states', type=int)

    args = parser.parse_args()

    run_tree_gen(
        states = args.states,
    )
