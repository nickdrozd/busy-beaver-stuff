import re
from itertools import permutations, product, filterfalse

STATES = A, B, C, D, E = 'A', 'B', 'C', 'D', 'E'

SHIFTS = R, L = 'R', 'L'

COLORS = 0, 1


def graph_to_string(graph, sep=''):
    return sep.join(
        dst
        for state in STATES
        for conn in graph[state]
        for dst in conn
    )


def yield_graphs():
    for b in permutations(STATES, r=2):
        b0, _b1 = b
        if b0 == A:
            continue
        for c in permutations(STATES, r=2):
            for d in permutations(STATES, r=2):
                for e in permutations(STATES, r=2):
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


def only_d_self_connected(arrows):
    if D not in arrows[D]:
        return False

    return all(state not in arrows[state]
               for state in set(STATES).difference(D))


def decorate(arrows):
    n3, n4 = arrows[B]
    n5, n6 = arrows[C]
    n7, n8 = arrows[D]
    n9, n10 = arrows[E]

    for shifts in product(SHIFTS, repeat=9):
        L_count = shifts.count(L)

        if L_count > 7 or L_count < 3:
            continue

        s2, s3, s4, s5, s6, s7, s8, s9, s10 = shifts

        for colors in product(COLORS, repeat=8):
            if colors.count(1) != 5:
                continue

            c3, c4, c5, c6, c7, c8, c9, c10 = colors

            yield f'1RB 1{s2}C {c3}{s3}{n3} {c4}{s4}{n4} {c5}{s5}{n5} {c6}{s6}{n6} {c7}{s7}{n7} {c8}{s8}{n8} {c9}{s9}{n9} {c10}{s10}{n10}'


regexps = (
    re.compile('^1RB ... .RC ... ..A'),
    re.compile('^1RB ... 0LC ... ..A'),
)

def is_obviously_stupid(prog_string):
    for regexp in regexps:
        if regexp.match(prog_string):
            return True

    return False


def compress(prog_string):
    return prog_string[4:].replace(' ', '')


def main():
    graphs = filter(
        only_d_self_connected,
        filter(
            is_strongly_connected,
            filter(
                is_normal,
                yield_graphs())))

    for graph in graphs:
        for prog in filterfalse(is_obviously_stupid, decorate(graph)):
            print(compress(prog))


if __name__ == '__main__':
    try:
        main()
    except BrokenPipeError:
        pass
