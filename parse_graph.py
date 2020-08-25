import sys

STATES = ['A', 'B', 'C', 'D']

def get_arrows(prog_string):
    prog_states = [
        action[2]
        for action in
        prog_string.split()
    ]

    arrows = {
        state: set()
        for state in
        STATES
    }

    for i, state in enumerate(prog_states):
        arrows[STATES[i // 2]].add(state)

    return arrows


def dump_dot(arrows):
    return 'digraph NAME {{\n  init -> A;\n{}\n}}'.format('\n'.join([
        f'  {node} -> {target};'
        for node, targets in arrows.items()
        for target in targets
    ]))


def flatten(arrows):
    # this will only return one permutation out of 2 ** 8 = 256
    return ' '.join([
        connection
        for state in sorted(STATES)
        for connection in sorted(arrows[state])
    ])


if __name__ == '__main__':
    print(
        flatten(
            get_arrows(
                sys.argv[1])))
