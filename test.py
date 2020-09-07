# pylint: disable=unused-wildcard-import,wildcard-import

import unittest

from turing import *

HALTING = {
    BB_2_2: (4, 6),
    BB_3_2: (5, 21),
    BB_2_3: (9, 38),
    BB_4_2: (13, 107),
    SIAB_3_3: (31, 2315619),
    RU_2_4: (90, 7195),
    BB_2_4: (2050, 3932964),
    BB_5_2: (4098, 47176870),
}


QUASIHALTING = {
    BBB_2_3_sigma: (10, 43),
    BBB_2_3_shift: (8, 59),
    "1RB 0LB 1RA 1LB 2LA 2RA": (3, 45),
    "1RB 2RA 2LB 2LB 2LA 0LA": (5, 40),

    BBB_3_2: (6, 55),
    "1LB 0RB 1LC 0LC 1RC 1RA": (6, 54),
    "1LB 0RC 1RB 0LC 1RC 1RA": (5, 52),
    "1LB 0RC 0RC 0LC 1RC 1RA": (5, 51),

    BBB_4_2: (69, 2819),
    "1RB 1RA 0RC 0RB 0RD 1RA 1LD 1LB": (0, 2568),
    "1RB 1RA 0RC 1LA 1LC 1LD 0RB 0RD": (0, 2512),
    "1RB 1LB 1RC 0LD 0RD 0RA 1LD 0LA": (20, 1153),
}


class TuringTest(unittest.TestCase):
    def test_halting(self):
        for prog, (sigma, shift) in HALTING.items():
            print(prog)
            machine = run_bb(prog)

            self.assertEqual(
                sigma,
                machine.ones_count)

            self.assertEqual(
                shift,
                machine.exec_count)

    def test_quasihalting(self):
        for prog, (sigma, shift) in QUASIHALTING.items():
            print(prog)
            machine = run_bb(prog, x_limit=shift)

            self.assertEqual(
                sigma,
                machine.ones_count)
