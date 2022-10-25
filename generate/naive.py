import re
from itertools import product
from typing import Optional
from collections.abc import Callable, Iterator

SHIFTS = 'R', 'L'
HALT   = '_'

Rejects = list[re.Pattern[str]]

def yield_all_programs(
        state_count: int,
        color_count: int,
        halt: bool = False,
) -> Iterator[str]:
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
        if max(colors) not in prog:
            continue

        if prog[:3] != '1RB':
            continue

        if halt and prog.count(HALT) != 1:
            continue

        yield prog



def b0_halt(colors: int) -> str:
    return '  '.join([
        ' '.join(('...' for _ in range(colors))),
        '.._',
    ])


def r_on_0(states: int, colors: int) -> str:
    return '^' + '  '.join(
        ' '.join(['.R.'] + ['...' for _ in range(colors - 1)])
        for _ in range(states)
    )


def reject(
        rejects: Rejects,
        states: int,
        colors: int,
        halt: bool = False,
) -> Callable[[str], bool]:
    rejects = [re.compile(regex) for regex in rejects]
    rejects.insert(0, re.compile(r_on_0(states, colors)))

    if halt:
        rejects.insert(0, re.compile(b0_halt(colors)))

    def reject_prog(prog: str) -> bool:
        return any(regex.match(prog) for regex in rejects)

    return reject_prog


def yield_programs(
        states: int,
        colors: int,
        halt: bool,
        rejects: Optional[Rejects] = None,
) -> Iterator[str]:
    if rejects is None:
        rejects = []

    reject_prog = reject(rejects, states, colors, halt)

    for prog in yield_all_programs(states, colors, halt):
        if not reject_prog(prog):
            yield prog
