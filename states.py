A, B, C, D = 'A', 'B', 'C', 'D'

STATES = {A, B, C, D}

if __name__ == '__main__':
    for a2 in STATES.difference(D):
        for b1 in STATES:
            if b1 == D and a2 != C:
                continue
            for b2 in STATES:
                if b2 == D and a2 != C and b1 != C:
                    continue
                for c1 in STATES:
                    for c2 in STATES:
                        for d1 in STATES:
                            for d2 in STATES:
                                print(
                                    f'B {a2} {b1} {b2} {c1} {c2} {d1} {d2}')
