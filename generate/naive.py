import re
from itertools import product
from collections.abc import Iterator, Sequence

SHIFTS = 'R', 'L'
HALT   = '_'

Rejects = Sequence[str | re.Pattern[str]]

def yield_actions(
        states: int,
        colors: int,
        halt: bool = False,
) -> Iterator[str]:
    yield from filter(
        (
            lambda action: action[2] != HALT or action == '1R_'
            if halt else
            lambda action: action
        ),
        (
            ''.join(prod)
            for prod in product(
                tuple(map(str, range(colors))),
                SHIFTS,
                tuple(map(chr, range(65, 65 + states)))
                + ((HALT,) if halt else ())
            )
        )
    )


def yield_programs(
        states: int,
        colors: int,
        halt: bool,
        rejects: Rejects | None = None,
) -> Iterator[str]:
    yield from filter(
        lambda prog: not any(
            re.compile(regex).match(prog)
            if isinstance(regex, str) else
            regex.match(prog)
            for regex in rejects or []
        ),
        (
            prog for prog in (
                '  '.join(state)
                for state in product(
                    (
                        ' '.join(state)
                        for state in
                        product(
                            yield_actions(states, colors, halt),
                            repeat = colors)
                    ),
                    repeat = states)
            ) if (
                prog[:3] == '1RB'
                and (not halt or prog.count(HALT) == 1))
        )
    )
