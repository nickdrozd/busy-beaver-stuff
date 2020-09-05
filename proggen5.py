import itertools

A, B, C, D, E = 'A', 'B', 'C', 'D', 'E'

STATES = ( A, B, C, D, E )


def graph_to_string(graph, sep=''):
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
    graph_string = graph_to_string(graph)

    if any(state not in graph_string for state in STATES):
        return False

    c = graph_string.find(C)
    d = graph_string.find(D)
    e = graph_string.find(E)

    return c < d < e


def is_strongly_connected(arrows):
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


def only_D_self_connected(arrows):
    if D not in arrows[D]:
        return False

    return all(state not in arrows[state]
               for state in set(STATES).difference(D))


def decorate(arrows):
    return graph_to_string(arrows, sep=' ')


if __name__ == '__main__':
    graphs = map(
        decorate,
        filter(
            only_D_self_connected,
            filter(
                is_strongly_connected,
                filter(
                    is_normal,
                    yield_graphs()))))

    for graph in graphs:
        print(graph)
