from itertools import filterfalse, product
import re

COLORS = '0', '1'
SHIFTS = R, L = 'R', 'L'
STATES = A, B, C, H = 'A', 'B', 'C', 'H'


def yield_all_programs():
    actions = (
        ''.join(prod) for prod
        in product(COLORS, SHIFTS, STATES))

    for prog in product(actions, repeat=5):
        yield '1RB ' + ' '.join(prog)


REJECTS = [
    '^[^H]+$',
    '.*H.*H.*',
    '.*0.H.*',
    '.*.LH.*',
    '^1RB ... ..H',

    '^1RB ..H ..[BC] ..[BC] ..[BC] ..[BC]',
    '^1RB ..[AB] ..[AB] ..[AB] ..H ...',
    '^1RB ..[AB] ..[AB] ..[AB] ... ..H',
]

def reject(prog):
    for regex in REJECTS:
        if re.match(regex, prog):
            return True

    return False


if __name__ == '__main__':
    try:
        for prog in yield_all_programs():
            if not reject(prog):
                print(prog)
                # print(prog[4:].replace(' ', ''))
    except BrokenPipeError:
        pass
