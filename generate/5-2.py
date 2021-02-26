import re
from itertools import permutations, product, filterfalse

from graph import Graph


STATES = A, B, C, D, E = 'A', 'B', 'C', 'D', 'E'

SHIFTS = R, L = 'R', 'L'

COLORS = 0, 1


def yield_graphs():
    a = B, C
    for b0, b1 in permutations(STATES, r=2):
        if b0 == A:
            continue
        for c in permutations(STATES, r=2):
            for d in permutations(STATES, r=2):
                for e in permutations(STATES, r=2):
                    graph = Graph((*a, b0, b1, *c, *d, *e))
                    if graph.is_normal and graph.is_strongly_connected:
                        if only_d_self_connected(graph):
                            yield graph


def only_d_self_connected(graph):
    if D not in graph.arrows[D]:
        return False

    return all(state not in graph.arrows[state]
               for state in set(STATES).difference(D))


def decorate(graph):
    _, _, n3, n4, n5, n6, n7, n8, n9, n10 = graph.flatten().split()

    for shifts in product(SHIFTS, repeat=9):
        L_count = shifts.count(L)

        if L_count > 7 or L_count < 3:
            continue

        s2, s3, s4, s5, s6, s7, s8, s9, s10 = shifts

        for colors in product(COLORS, repeat=8):
            if colors.count(1) != 5:
                continue

            c3, c4, c5, c6, c7, c8, c9, c10 = colors

            # pylint: disable=line-too-long
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
    for graph in yield_graphs():
        for prog in filterfalse(is_obviously_stupid, decorate(graph)):
            print(compress(prog))


if __name__ == '__main__':
    try:
        main()
    except BrokenPipeError:
        pass
