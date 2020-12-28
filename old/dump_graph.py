import sys

from arrows import parse_arrows

COLORS = (
    'blue',
    'red',
    'green',
    'black',
)


def dump_dot(arrows):
    return 'digraph NAME {{\n{}\n}}'.format('\n'.join([
        f'  {node} -> {target} [ color=" {COLORS[i]}" ];'
        for node, targets in arrows.items()
        for i, target in enumerate(targets)
    ]))


if __name__ == '__main__':
    for graph in sys.stdin:
        print(
            dump_dot(
                parse_arrows(
                    graph)))
