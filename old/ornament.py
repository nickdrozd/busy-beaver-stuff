import sys
from itertools import product

COLORS = ('1', '0')
SHIFTS = ('R', 'L')


if __name__ == '__main__':
    for i, graph in enumerate(sys.stdin):
        n1, n2, n3, n4, n5, n6, n7, n8 = graph.split()

        for colors in product(COLORS, repeat=7):
            if colors.count('1') < 3:
                continue

            if colors.count('1') == 7:
                continue

            c2, c3, c4, c5, c6, c7, c8 = colors

            for shifts in product(SHIFTS, repeat=7):
                if any(shifts.count(shift) == 7 for shift in shifts):
                    continue

                s2, s3, s4, s5, s6, s7, s8 = shifts

                # pylint: disable=line-too-long
                print(f'1R{n1} {c2}{s2}{n2} {c3}{s3}{n3} {c4}{s4}{n4} {c5}{s5}{n5} {c6}{s6}{n6} {c7}{s7}{n7} {c8}{s8}{n8}')
