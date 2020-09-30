import sys

A, B, C, D = 'A', 'B', 'C', 'D'

STATES = {A, B, C, D}


def parse_arrows(prog_string):
    states = iter(
        prog_string.split())

    connections = zip(states, states)

    # pylint: disable=unnecessary-comprehension
    return {
        state: connection
        for (state, connection) in
        zip(sorted(STATES), connections)
    }


def is_connected(arrows):
    for state in STATES:
        if all(state not in arrows[dst]
               for dst in STATES.difference(state)):
            return False

    for state in STATES:
        reachable_from_x = set(arrows[state]).difference(state)

        for _ in range(3):
            reachable = {
                node
                for connection in reachable_from_x
                for node in arrows[connection]
            }

            reachable_from_x.update(reachable)

        if not reachable_from_x.issuperset(STATES):
            return False

    return True


if __name__ == '__main__':
    for prog in sys.stdin:
        if is_connected(parse_arrows(prog)):
            print(prog.strip())
