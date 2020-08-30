import itertools

COLOR_COUNT = 2

A, B, C, D, E, F, G = 'A', 'B', 'C', 'D', 'E', 'F', 'G'

STATES = A, B, C, D, E, F, G


def how_many_graphs(state_count, color_count):
    return state_count ** (state_count * color_count)


def generate_states(n):
    assert n <= len(STATES)

    # grab the first n states
    states = STATES[:n]

    # get all the pairs of those states
    pairs = itertools.product(states, repeat=COLOR_COUNT)

    # get all the n-tuples of pairs
    yield from itertools.product(pairs, repeat=n)


def is_normal(graph):
    if graph[0][0] != 'B':
        return False

    for i, connections in enumerate(graph[ : - COLOR_COUNT]):
        possible = set(STATES[ : i + COLOR_COUNT + 1])
        if not set(connections).issubset(possible):
            return False

    return True


def is_connected(graph):
    states = set(STATES[:len(graph)])

    arrows = dict(zip(states, graph))

    for state in states:
        if all(state not in arrows[dst]
               for dst in states.difference(state)):
            return False

    for state in states:
        reachable_from_x = set(arrows[state]).difference(state)

        for _ in range(3):
            reachable = {
                node
                for connection in reachable_from_x
                for node in arrows[connection]
            }

            reachable_from_x.update(reachable)

        if not reachable_from_x.issuperset(states):
            return False

    return True


def main():
    print(len(list(
        filter(
            is_connected,
            filter(
                is_normal,
                generate_states(4))))))
