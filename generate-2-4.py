from itertools import product

COLORS = 0, 1, 2, 3
POSITS = 1, 2, 3
SHIFTS = L, R = 'L', 'R'
STATES = A, B = 'A', 'B'

def main():
    for colors in product(COLORS, repeat=7):
        if (colors.count(0) > 1
            or colors.count(1) == 0
            or colors.count(2) == 0
            or colors.count(3) == 0):
            continue

        c2, c3, c4, c5, c6, c7, c8 = colors

        for states in product(STATES, repeat=7):
            t2, t3, t4, t5, t6, t7, t8 = states

            if A not in (t5, t6, t7, t8):
                continue

            for shifts in product(SHIFTS, repeat=7):
                if not (2 < shifts.count(L) < 6):
                    continue

                s2, s3, s4, s5, s6, s7, s8 = shifts

                print(f'1RB {c2}{s2}{t2} {c3}{s3}{t3} {c4}{s4}{t4} {c5}{s5}{t5} {c6}{s6}{t6} {c7}{s7}{t7} {c8}{s8}{t8}')


if __name__ == '__main__':
    try:
        main()
    except BrokenPipeError:
        pass
