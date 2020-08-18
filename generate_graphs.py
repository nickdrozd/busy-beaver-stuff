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


SEEN = []
ISOS = []


if __name__ == '__main__':
    for dest_a in STATES.difference(A):
        for rest_a in STATES.difference(dest_a):
            for dest_b in STATES.difference(B):
                for rest_b in STATES.difference(dest_b):
                    for dest_c in STATES.difference(C):
                        for rest_c in STATES.difference(dest_c):
                            for dest_d in STATES.difference(D):
                                for rest_d in STATES.difference(dest_d):
                                    arrows = {
                                        A: {dest_a, rest_a},
                                        B: {dest_b, rest_b},
                                        C: {dest_c, rest_c},
                                        D: {dest_d, rest_d},
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

                                    # print(arrows)

                                    print(
                                        dump_dot(
                                            arrows))
