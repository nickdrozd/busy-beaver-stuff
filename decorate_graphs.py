import sys

SHIFTS = ('L', 'R')
COLORS = ('1', '0')

if __name__ == '__main__':
    for i, graph in enumerate(sys.stdin):
        n1, n2, n3, n4, n5, n6, n7, n8 = graph.split()

        for s1 in SHIFTS:
            for s2 in SHIFTS:
                for s3 in SHIFTS:
                    for s4 in SHIFTS:
                        for s5 in SHIFTS:
                            for s6 in SHIFTS:
                                for s7 in SHIFTS:
                                    for s8 in SHIFTS:
                                        if (s1 == s2 == s3 == s4 == s5 == s6 == s7 == s8):
                                            continue

                                        for c1 in COLORS:
                                            for c2 in COLORS:
                                                for c3 in COLORS:
                                                    for c4 in COLORS:
                                                        for c5 in COLORS:
                                                            for c6 in COLORS:
                                                                for c7 in COLORS:
                                                                    for c8 in COLORS:
                                                                        if (c1 == c2 == c3 == c4 == c5 == c6 == c7 == c8):
                                                                            continue

                                                                        print(f'{c1}{s1}{n1} {c2}{s2}{n2} {c3}{s3}{n3} {c4}{s4}{n4} {c5}{s5}{n5} {c6}{s6}{n6} {c7}{s7}{n7} {c8}{s8}{n8}')
