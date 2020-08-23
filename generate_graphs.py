A = 'A'
B = 'B'
C = 'C'
D = 'D'

STATES = {A, B, C, D}


def dump_dot(arrows):
    return 'digraph NAME {{init -> A;{}}}'.format(''.join([
        f' {node} -> {target};'
        for node, targets in arrows.items()
        for target in targets
    ]))


def is_connected(arrows):
    if all(len(arrows[state]) == 1 for state in STATES):
        return False

    for state in STATES:
        if all(state not in arrows[dst]
               for dst in STATES.difference(state)):
            return False

    for state in STATES:
        reachable_from_x = arrows[state].difference(state)

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


def flatten(arrows):
    # this will only return one permutation out of 2 ** 8 = 256
    return ' '.join([
        connection
        for state in sorted(STATES)
        for connection in sorted(arrows[state])
    ])


if __name__ == '__main__':
    SEEN = []
    ISOS = []

    for a2 in (A, C):
        for b1 in tuple(STATES.difference(D)):
            for b2 in tuple(STATES.difference(b1)):
                if {b1, b2}.intersection({A, B}) and D in {b1, b2}:
                    continue
                for c1 in tuple(STATES):
                    for c2 in tuple(STATES.difference(c1)):
                        for d1 in tuple(STATES):
                            for d2 in tuple(STATES.difference(d1)):
                                arrows = {
                                    A: {B, a2},
                                    B: {b1, b2},
                                    C: {c1, c2},
                                    D: {d1, d2},
                                }

                                if any(arrows == seen
                                       for seen in SEEN):
                                    continue

                                SEEN.append(arrows)

                                if not is_connected(arrows):
                                    continue

                                if any(is_isomorphic(arrows, graph)
                                       for graph in ISOS):
                                    continue

                                ISOS.append(arrows)

                                # print(flatten(arrows))

                                print(dump_dot(arrows))
