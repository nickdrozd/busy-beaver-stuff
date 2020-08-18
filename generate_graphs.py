STATES = {'A', 'B', 'C', 'D'}


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

    reachable_from_a = arrows['A'].difference('A')

    for _ in range(3):
        reachable = {
            node
            for connection in reachable_from_a
            for node in arrows[connection]
        }

        reachable_from_a.update(reachable)

    return reachable_from_a.issuperset(STATES)


SEEN = []


if __name__ == '__main__':
    for dest_a in STATES.difference('A'):
        for rest_a in STATES.difference(dest_a):
            for dest_b in STATES.difference('B'):
                for rest_b in STATES.difference(dest_b):
                    for dest_c in STATES.difference('C'):
                        for rest_c in STATES.difference(dest_c):
                            for dest_d in STATES.difference('D'):
                                for rest_d in STATES.difference(dest_d):
                                    arrows = {
                                        'A': {dest_a, rest_a},
                                        'B': {dest_b, rest_b},
                                        'C': {dest_c, rest_c},
                                        'D': {dest_d, rest_d},
                                    }

                                    if any(arrows == seen for seen in SEEN):
                                        continue

                                    SEEN.append(arrows)

                                    if not is_connected(arrows):
                                        continue

                                    print(arrows)

                                    # print(
                                    #     dump_dot(
                                    #         arrows))
