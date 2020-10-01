import re
from itertools import permutations, product, filterfalse

STATES = A, B, C, D = 'A', 'B', 'C', 'D'

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
    for a1 in (A, C, D):
        for b in permutations(STATES, r=2):
            b0, _ = b
            if b0 == A:
                continue
            for c in permutations(STATES, r=2):
                for d in permutations(STATES, r=2):
                    yield {A: (B, a1), B: b, C: c, D: d}


def is_normal(graph):
    graph_string = graph_to_string(graph)

    if any(state not in graph_string for state in STATES):
        return False

    c = graph_string.find(C)
    d = graph_string.find(D)

    return c < d


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


def decorate(arrows):
    n1, n2 = arrows[A]
    n3, n4 = arrows[B]
    n5, n6 = arrows[C]
    n7, n8 = arrows[D]

    for shifts in product(SHIFTS, repeat=7):
        L_count = shifts.count(L)

        if L_count > 6 or L_count < 2:
            continue

        s2, s3, s4, s5, s6, s7, s8 = shifts

        for colors in product(COLORS, repeat=7):
            if colors.count(1) < 2:
                continue

            c2, c3, c4, c5, c6, c7, c8 = colors

            # pylint: disable=line-too-long
            yield f'1R{n1} {c2}{s2}{n2} {c3}{s3}{n3} {c4}{s4}{n4} {c5}{s5}{n5} {c6}{s6}{n6} {c7}{s7}{n7} {c8}{s8}{n8}'


regexps = (
    '^1RB ... .RB',
    '^1RB ... 0LB .RC .RA',
    '^1RB ... 0LB 0LC 0RA',
    '^1RB ..A 0LB .RC .LA',
    '^1RB 0RA .LB .LC 0LA',
    '^1RB 0.A .LB 1LC .RA',
    '^1RB 1RA .LB .LC 0LA',
    '^1RB .LA .LB .LC 0LA',
)

compiled_regexps = [re.compile(regexp) for regexp in regexps]

def is_obviously_stupid(prog_string):
    for regexp in compiled_regexps:
        if regexp.match(prog_string):
            return True

    return False


def compress(prog_string):
    return prog_string[4:].replace(' ', '')


def main():
    graphs = filter(
        is_strongly_connected,
        filter(
            is_normal,
            yield_graphs()))

    for graph in graphs:
        for prog in filterfalse(is_obviously_stupid, decorate(graph)):
            print((prog))


if __name__ == '__main__':
    try:
        main()
    except BrokenPipeError:
        pass
