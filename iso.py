import sys

A, B, C, D = 'A', 'B', 'C', 'D'

STATES = {A, B, C, D}


def parse_arrows(prog_string):
    states = iter(
        prog_string.split())

    connections = zip(states, states)

    return {
        state: connection
        for (state, connection) in
        zip(sorted(STATES), connections)
    }


def is_isomorphic(g1, g2):
    for s1 in STATES:
        for s2 in STATES.difference(s1):
            for s3 in STATES.difference({s1, s2}):
                s4 = list(STATES.difference({s1, s2, s3}))[0]

                m = {
                    A: s1,
                    B: s2,
                    C: s3,
                    D: s4,
                }

                if all(g1[s] == g2[m[s]] for s in STATES):
                    return True

    return False


if __name__ == '__main__':
    ISOS = []

    for prog in sys.stdin:
        arrows = parse_arrows(prog)

        if any(is_isomorphic(arrows, graph) for graph in ISOS):
            continue

        ISOS.append(arrows)

        print(prog.strip())
