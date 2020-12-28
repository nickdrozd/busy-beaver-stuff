import sys

STATES = ['A', 'B', 'C', 'D', 'E', 'H']

def get_arrows(prog_string):
    prog_states = prog_string.split()

    arrows = {
        state: set()
        for state in
        STATES
    }

    for i, state in enumerate(prog_states):
        arrows[STATES[i // 2]].add(state)

    return arrows


def dump_dot(arrows):
    return 'digraph NAME {{\n  init -> A;\n{}\n}}'.format(''.join([
        f'  {node} -> {target} [ label=" {i}" ];'
        for node, targets in arrows.items()
        for i, target in enumerate(targets)
    ]))


def flatten(arrows):
    # this will only return one permutation out of 2 ** 8 = 256
    return ' '.join([
        connection
        for state in sorted(STATES)
        for connection in sorted(arrows[state])
    ])


if __name__ == '__main__':
    for graph in sys.stdin:
        print(
            dump_dot(
                get_arrows(
                    graph)))
