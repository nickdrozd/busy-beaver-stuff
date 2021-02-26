import re
from itertools import permutations, product, filterfalse

from graph import Graph


STATES = A, B, C, D = 'A', 'B', 'C', 'D'

SHIFTS = R, L = 'R', 'L'

COLORS = 0, 1


def yield_graphs():
    for a1 in A, C, D:
        for b0, b1 in permutations(STATES, r=2):
            if b0 == A:
                continue
            for c in permutations(STATES, r=2):
                for d in permutations(STATES, r=2):
                    graph = Graph((B, a1, b0, b1, *c, *d))
                    if graph.is_normal and graph.is_strongly_connected:
                        yield graph


def decorate(graph):
    n1, n2, n3, n4, n5, n6, n7, n8 = graph.flatten().split()

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


def main():
    for graph in yield_graphs():
        for prog in filterfalse(is_obviously_stupid, decorate(graph)):
            print(prog)


if __name__ == '__main__':
    try:
        main()
    except BrokenPipeError:
        pass
