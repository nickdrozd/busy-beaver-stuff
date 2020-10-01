import re
from itertools import filterfalse, product

COLORS = 1, 2, 3
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

        if not is_normal(colors):
            continue

        c2, c3, c4, c5, c6, c7, c8 = colors

        if (c2 == 1
            or c2 == c3 == c4
            or c5 == c6 == c7 == c8):
            continue

        for states in product(STATES, repeat=7):
            t2, t3, t4, t5, t6, t7, t8 = states

            if (A not in  (t2, t3, t4)
                or A not in (t5, t6, t7, t8)
                or B not in (t5, t6, t7, t8)):
                continue

            for shifts in product(SHIFTS, repeat=7):
                s2, s3, s4, s5, s6, s7, s8 = shifts

                if s2 == s3 == s4 or s5 == s6 == s7 == s8:
                    continue

                # pylint: disable=line-too-long
                yield f'1RB {c2}{s2}{t2} {c3}{s3}{t3} {c4}{s4}{t4} {c5}{s5}{t5} {c6}{s6}{t6} {c7}{s7}{t7} {c8}{s8}{t8}'


regexps = (
    '^1RB ... ... ... 1R.',
    '^1RB ... ... ... 1LB 1.B',
)

compiled_regexps = [re.compile(regexp) for regexp in regexps]

def is_obviously_stupid(prog_string):
    for regexp in compiled_regexps:
        if regexp.match(prog_string):
            return True

    return False


def compress(prog_string):
    return prog_string[4:].replace(' ', '')


def main():
    for prog in filterfalse(is_obviously_stupid, yield_progs()):
        print(compress(prog))


if __name__ == '__main__':
    try:
        main()
    except BrokenPipeError:
        pass
