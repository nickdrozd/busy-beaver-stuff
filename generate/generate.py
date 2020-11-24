from itertools import product, filterfalse
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
    '^[^H]+$',
    '.*H.*H.*',
    '.*0.H.*',
    '.*.LH.*',
]


def B0_halt(colors):
    dots = ' '.join(('...' for _ in range(colors)))
    return f'^{dots} ..H'


def R_on_0(states, colors):
    r0 = ' '.join(['.R.'] + ['...' for _ in range(colors - 1)])
    return f"^{' '.join(r0 for _ in range(states))}"


def reject(rejects, states, colors, halt=False):
    rejects = [R_on_0(states, colors)] + rejects

    if halt:
        rejects = HALT_NORMAL + [B0_halt(colors)] + rejects

    def reject_prog(prog):
        return any(
            re.match(regex, prog)
            for regex in rejects
        )

    return reject_prog


def compress(prog_string):
    return prog_string[4:].replace(' ', '')


def yield_programs(states, colors, rejects=None, halt=False):
    if rejects is None:
        rejects = []

    all_progs = yield_all_programs(states, colors, halt)
    reject_filter = reject(rejects, states, colors, halt)

    yield from filterfalse(reject_filter, all_progs)


def print_programs(progs, full=True):
    try:
        for prog in progs:
            print(prog if full else compress(prog))
    except BrokenPipeError:
        pass
