STATES = {'A', 'B', 'C', 'D'}


def dump_dot(arrows):
    return 'digraph NAME {{init -> A;{}}}'.format(''.join([
        f' {node} -> {target};'
        for node, targets in arrows.items()
        for target in targets
    ]))


if __name__ == '__main__':
    for dest_a in STATES:
        for rest_a in STATES.difference('A'):
            for dest_b in STATES:
                for rest_b in STATES.difference('B'):
                    for dest_c in STATES:
                        for rest_c in STATES.difference('C'):
                            for dest_d in STATES:
                                for rest_d in STATES.difference('D'):
                                    arrows = {
                                        'A': {dest_a, rest_a},
                                        'B': {dest_b, rest_b},
                                        'C': {dest_c, rest_c},
                                        'D': {dest_d, rest_d},
                                    }

                                    print(
                                        dump_dot(
                                            arrows))
