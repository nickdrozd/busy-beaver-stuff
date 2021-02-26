from itertools import product

COLORS = 0, 1, 2, 3
SHIFTS = L, R = 'L', 'R'
STATES = A, B = 'A', 'B'


def is_normal(colors):
    for color in colors:
        if color == 2:
            return True
        if color == 3:
            return False

    return False


def yield_progs():
    for colors in product(COLORS, repeat=7):
        if (1 not in colors
            or 2 not in colors
            or 3 not in colors):
            continue

        if 0 not in colors:
            continue

        if not is_normal(colors):
            continue

        c2, c3, c4, c5, c6, c7, c8 = colors

        if (c2 == c3 == c4 == 1
            or c5 == c6 == c7 == c8):
            continue

        for states in product(STATES, repeat=7):
            t2, t3, t4, t5, t6, t7, t8 = states

            if (A not in (t2, t3, t4)
                or A not in (t5, t6, t7, t8)
                or B not in (t5, t6, t7, t8)):
                continue

            for shifts in product(SHIFTS, repeat=7):
                s2, s3, s4, s5, s6, s7, s8 = shifts

                if s5 == R:
                    continue

                if s2 == s3 == s4 or s5 == s6 == s7 == s8:
                    continue

                # pylint: disable=line-too-long
                yield f'1RB {c2}{s2}{t2} {c3}{s3}{t3} {c4}{s4}{t4} {c5}{s5}{t5} {c6}{s6}{t6} {c7}{s7}{t7} {c8}{s8}{t8}'


def compress(prog_string):
    return prog_string[4:].replace(' ', '')


def main():
    for prog in yield_progs():
        print((prog))


if __name__ == '__main__':
    try:
        main()
    except BrokenPipeError:
        pass
