import itertools

COLOR_COUNT = 2

A, B, C, D, E, F, G = 'A', 'B', 'C', 'D', 'E', 'F', 'G'

STATES = A, B, C, D, E, F, G


def how_many_graphs(state_count, color_count):
    return state_count ** (state_count * color_count)


def is_normal(graph):
    if graph[0][0] != 'B':
        return False

    for i, state in enumerate(graph[:-COLOR_COUNT]):
        pass

    return True


def generate_states(n):
    assert n <= len(STATES)

    # grab the first n states
    states = STATES[:n]

    # get all the pairs of those states
    pairs = itertools.product(states, repeat=COLOR_COUNT)

    # get all the n-tuples of pairs
    yield from itertools.product(pairs, repeat=n)


def main():
    print(len(list(filter(is_normal, generate_states(4)))))
