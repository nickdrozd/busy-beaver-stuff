import itertools

A, B, C, D, E = 'A', 'B', 'C', 'D', 'E'

STATES = ( A, B, C, D, E )


def to_string(graph, sep=''):
    return sep.join(
        dst
        for state in STATES
        for conn in graph[state]
        for dst in conn
    )


def yield_graphs():
    for b in itertools.permutations(STATES, r=2):
        for c in itertools.permutations(STATES, r=2):
            for d in itertools.permutations(STATES, r=2):
                for e in itertools.permutations(STATES, r=2):
                    yield {A: (B, C), B: b, C: c, D: d, E: e,}


def is_normal(graph):
    graph_string = to_string(graph)

    if any(state not in graph_string for state in STATES):
        return False

    c = graph_string.find(C)
    d = graph_string.find(D)
    e = graph_string.find(E)

    return c < d < e


def is_connected(arrows):
    for state in STATES:
        if all(state not in arrows[dst]
               for dst in set(STATES).difference(state)):
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
    graphs = filter(
        is_connected,
        filter(
            is_normal,
            yield_graphs()))

    for graph in graphs:
        print(to_string(graph, sep=' '))
