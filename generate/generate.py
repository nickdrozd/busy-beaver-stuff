from itertools import product
import re

SHIFTS = 'R', 'L'
HALT   = 'H'

def yield_all_programs(state_count, color_count, halt=False):
    states = tuple(map(chr, range(65, 65 + state_count)))
    colors = tuple(map(str, range(color_count)))

    actions = (
        ''.join(prod) for prod
        in product(
            colors,
            SHIFTS,
            states + ((HALT,) if halt else ()))
    )

    transitions = len(colors) * len(states)

    for prog in product(actions, repeat = transitions - 1):
        yield '1RB ' + ' '.join(prog)


HALT_NORMAL = [
    re.compile('^[^H]+$'),
    re.compile('.*H.*H.*'),
    re.compile('.*0.H.*'),
    re.compile('.*.LH.*'),
]


def B0_halt(colors):
    dots = ' '.join(('...' for _ in range(colors)))
    return f'^{dots} ..H'


def R_on_0(states, colors):
    r0 = ' '.join(['.R.'] + ['...' for _ in range(colors - 1)])
    return f"^{' '.join(r0 for _ in range(states))}"


def reject(rejects, states, colors, halt=False):
    rejects = [re.compile(regex) for regex in rejects]
    rejects.insert(0, re.compile(R_on_0(states, colors)))

    if halt:
        rejects.insert(0, re.compile(B0_halt(colors)))
        rejects = HALT_NORMAL + rejects

    def reject_prog(prog):
        for regex in rejects:
            if regex.match(prog):
                return True

        return False

    return reject_prog


def compress(prog_string):
    return prog_string[4:].replace(' ', '')


def yield_programs(states, colors, rejects=None, halt=False):
    if rejects is None:
        rejects = []

    reject_prog = reject(rejects, states, colors, halt)

    for prog in yield_all_programs(states, colors, halt):
        if not reject_prog(prog):
            yield prog


def print_programs(progs, full=True):
    try:
        for prog in progs:
            print(prog if full else compress(prog))
    except BrokenPipeError:
        pass
