from itertools import product
import re

SHIFTS = 'R', 'L'
HALT   = '_'

def yield_all_programs(state_count, color_count, halt = False):
    states = tuple(map(chr, range(65, 65 + state_count)))
    colors = tuple(map(str, range(color_count)))

    actions = filter(
        (
            lambda action: action[2] != HALT or action == '1R_'
            if halt else
            lambda action: action
        ),
        (
            ''.join(prod) for prod
            in product(
                colors,
                SHIFTS,
                states + ((HALT,) if halt else ()))
        )
    )

    progs = (
        '  '.join(state)
        for state in
        product(
            (
                ' '.join(state)
                for state in
                product(actions, repeat = color_count)
            ),
            repeat = state_count))

    for prog in progs:
        if prog[:3] != '1RB':
            continue

        if halt and prog.count(HALT) != 1:
            continue

        if max(colors) not in prog:
            continue

        yield prog



def b0_halt(colors):
    return '  '.join([
        ' '.join(('...' for _ in range(colors))),
        '.._',
    ])


def r_on_0(states, colors):
    prog = '  '.join(
        ' '.join(['.R.'] + ['...' for _ in range(colors - 1)])
        for _ in range(states)
    )

    return '^' + prog


def reject(rejects, states, colors, halt = False):
    rejects = [re.compile(regex) for regex in rejects]
    rejects.insert(0, re.compile(r_on_0(states, colors)))

    if halt:
        rejects.insert(0, re.compile(b0_halt(colors)))

    def reject_prog(prog):
        for regex in rejects:
            if regex.match(prog):
                return True

        return False

    return reject_prog


def compress(prog_string):
    return prog_string[4:].replace(' ', '')


def yield_programs(states, colors, halt, rejects = None):
    if rejects is None:
        rejects = []

    reject_prog = reject(rejects, states, colors, halt)

    for prog in yield_all_programs(states, colors, halt):
        if not reject_prog(prog):
            yield prog


def print_programs(progs, full = True):
    try:
        for prog in progs:
            print(prog if full else compress(prog))
    except BrokenPipeError:
        pass
