from __future__ import annotations

import re
from itertools import product
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Iterator

SHIFTS = 'R', 'L'
HALT   = '_'

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
        rejects: list[str] | None = None,
) -> Iterator[str]:
    yield from filter(
        lambda prog: not any(
            re.compile(regex).match(prog)
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
