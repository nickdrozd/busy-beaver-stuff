import sys
import argparse

from tools.normalize import normalize, expand
from tools.tree_norm import tree_norm

if __name__ == '__main__':
    parser = argparse.ArgumentParser()

    parser.add_argument('--tree', action = 'store_true')

    args = parser.parse_args()

    for prog in sys.stdin:
        normalizer = (
            tree_norm if args.tree else
            expand if ' ' not in prog else
            normalize
        )

        print(normalizer(prog))
