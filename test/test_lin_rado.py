from __future__ import annotations

from unittest import TestCase

import re
from itertools import product
from typing import TYPE_CHECKING

from test.utils import read_progs

from tm.show import show_state
from tm.lin_rec import (
    StrictLinRecMachine,
    quick_term_or_rec,
)

if TYPE_CHECKING:
    from collections.abc import Iterator

SHIFTS = LEFT, RIGHT = 'L', 'R'
HALT   = '_'


class TestLinRado(TestCase):
    progs_strict: set[str]
    progs_loose: set[str]

    def assert_progs_equal(self, other: set[str]):
        self.assertEqual(self.progs_strict, other)
        self.assertEqual(self.progs_loose, other)

    def assert_progs_count(self, count: int):
        self.assertEqual(len(self.progs_strict), count)
        self.assertEqual(len(self.progs_loose), count)

    def run_lin_rado(
            self,
            states: int,
            colors: int,
            halt: int,
            strict: int,
            loose: int,
            rejects: list[str] | None = None,
    ):
        self.progs_strict = {
            prog
            for prog in
            yield_programs(
                states,
                colors,
                bool(halt),
                rejects)
            if
            StrictLinRecMachine(prog).run(
                sim_lim = strict,
                check_rec = 0,
            ).xlimit is not None
        }

        self.progs_loose = {
            prog
            for prog in
            yield_programs(
                states,
                colors,
                bool(halt),
                rejects)
            if
            not quick_term_or_rec(prog, loose)
        }

    def test_22(self):
        # h
        self.run_lin_rado(
            2, 2, 1,
            6,
            8,
        )

        self.assert_progs_count(
            0)

        # q
        self.run_lin_rado(
            2, 2, 0,
            13,
            22,
            rejects = [],
        )

        self.assert_progs_equal(
            {
                "1RB 1LA  0LA 0RB",  # counter
                "1RB 1LA  1LA 1RB",  # xmas classic
                "1RB 1LA  0LA 1RB",  # xmas one-side
                "1RB 0LB  1LA 0RA",  # xmas spaces
            })

        self.assert_progs_count(
            4)

    def test_32h(self):
        self.run_lin_rado(
            3, 2, 1,
            29,
            45,
            rejects = NOT_CONNECTED_32,
        )

        self.assert_progs_count(
            40)

        self.assert_progs_equal(
            LIN_HOLDOUTS)

        self.assertEqual(
            LIN_HOLDOUTS,
            BRADY_HOLDOUTS | LR_NOT_BRADY)


LR_HOLDOUTS = {
    # Lot 1
    0o73037233,
    0o73137233,
    0o73137123,
    0o73136523,
    0o73133271,
    0o73133251,
    0o73132742,
    0o73132542,
    0o73032532,
    0o73032632,
    0o73033132,
    0o73033271,
    0o73073271,
    0o73075221,
    # Lot 2
    0o73676261,
    0o73736122,
    0o71536037,
    0o73336333,
    0o71676261,
    0o73336133,
    0o73236333,
    0o73236133,
    # Lot 3
    0o70537311,
    0o70636711,
    0o70726711,
    0o72737311,
    0o71717312,
    0o72211715,
    0o72237311,
    0o72311715,
    0o72317716,
    0o72331715,
    0o72337311,
    0o72337315,
    # Lot 4
    0o70513754,
    0o70612634,
    0o70712634,
    0o72377034,
    0o72377234,
    0o72613234,
}

def lr_convert(rado_string: int) -> str:
    def oct_to_bin(oct_string: int) -> str:
        return f'{oct_string:b}'

    def bin_to_prog(bin_string: str) -> str:
        a0, a1, b0, b1, c0, c1 = map(
            convert_bin_instr,
            (
                tuple(bin_string[i : i + 4])
                for i in range(0, len(bin_string), 4)
            ),
        )

        return f'{a0} {a1}  {b0} {b1}  {c0} {c1}'

    def convert_bin_instr(bin_instr: tuple[str, ...]) -> str:
        pr, sh, *tr =  bin_instr

        v_sh = LEFT if int(sh) == 0 else RIGHT

        v_tr = show_state(int(''.join(tr), 2) - 1)

        return f'{pr}{v_sh}{v_tr}'

    return bin_to_prog(
        oct_to_bin(
            rado_string))

LIN_HOLDOUTS = set(map(lr_convert, LR_HOLDOUTS))

AB_LOOP = '^1RB ..[AB]  ..[AB] ..[AB]  ... ...'
BC_LOOP = '^1RB ...  ..[BC] ..[BC]  ..[BC] ..[BC]'

NOT_CONNECTED_32 = [
    AB_LOOP,
    BC_LOOP,
]

LR_NOT_BRADY = read_progs('lr_not_brady')
BRADY_HOLDOUTS = read_progs('brady_holdouts')

LIN_EXAMPLES = {
    "1RB ...  0RC 1LB  1LA 0RB",  # total recurrence (blank)
    "1RB ...  1LB 0LC  1LA 1RA",  # left barrier
    "1RB ...  1LC 1RA  1LA 0LC",  # right barrier
}


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
