import unittest

from turing import run_bb

HALTING = {
    # BB_2_2
    "1RB 1LB 1LA 1RH": (4, 6),

    # BB_3_2 shift
    "1RB 1RH 1LB 0RC 1LC 1LA": (5, 21),
    # BB_3_2 sigma
    "1RB 1LC 1RC 1RH 1LA 0LB": (6, 11),

    # BB_2_3
    "1RB 2LB 1RH 2LA 2RB 1LB": (9, 38),

    # BB_4_2 shift
    "1RB 1LB 1LA 0LC 1RH 1LD 1RD 0RA": (13, 107),
    # BB_4_2 sigma
    "1RB 0RC 1LA 1RA 1RH 1RD 1LD 0LB": (13, 96),

    # SIAB_3_3
    "1RB 2LB 1LC 1LA 2RB 1RB 1RH 2LA 0LC": (31, 2315619),

    # RU_2_4
    "1RB 3LA 1LA 1RA 2LA 1RH 3RA 3RB": (90, 7195),

    # BB_2_4
    "1RB 2LA 1RA 1RA 1LB 1LA 3RB 1RH": (2050, 3932964),

    # COPY_3_3_2_4
    "1RB 1LC 1RH 1LA 1LC 2RB 1RB 2LC 1RC": (2050, 3932964),

    # BB_5_2
    "1RB 1LC 1RC 1RB 1RD 0LE 1LA 1LD 1RH 0LA": (4098, 47176870),
}


QUASIHALTING = {
    # 2_2 (not better than BB)
    "1RB 1LB 1LB 1LA": (3, 6),
    "1RB 1LB 0LB 1LA": (2, 6),
    "1RB 0LB 1LB 1LA": (2, 6),
    "1RB 0LB 0LB 1LA": (1, 6),

    # BBB_2_3_sigma
    "1RB 2LB 1RA 2LB 2LA 0RA": (10, 43),
    # BBB_2_3_shift
    "1RB 2LB 1LA 2LB 2RA 0RA": (8, 59),
    "1RB 0LB 1RA 1LB 2LA 2RA": (3, 45),
    "1RB 2RA 2LB 2LB 2LA 0LA": (5, 40),
    "1RB 1LA 2RA 2LA 2LB 2RB": (8, 17),

    # BBB_3_2 shift
    "1RB 0LB 1LA 0RC 1LC 1LA": (6, 55),
    # BBB_3_2 sigma
    "1RB 1RC 1LC 1RA 1RA 1LA": (6, 9),   # 2 SO
    "1RB 0LB 1RC 0RC 1LC 1LA": (6, 54),
    "1RB 0LC 1LB 0RC 1LC 1LA": (5, 52),
    "1RB 0LC 0LC 0RC 1LC 1LA": (5, 51),
    "1RB 0LC 1LA 0RC 1RC 1RB": (5, 49),
    "1RB 0LC 0RC 0RC 1LC 1LA": (5, 48),
    "1RB 1RC 1LC 0LB 1RA 1LA": (5, 22),  # 2 SO

    # BBB_4_2_shift
    "1RB 0LC 1LD 0LA 1RC 1RD 1LA 0LD": (0, 66349),
    # BBB_4_2_sigma
    "1RB 1RC 1LC 1RD 1RA 1LD 0RD 0LB": (69, 2819),
    "1RB 1RA 0RC 0RB 0RD 1RA 1LD 1LB": (0, 2568),
    "1RB 1RA 0RC 1LA 1LC 1LD 0RB 0RD": (0, 2512),
    "1RB 1RC 1RD 0LC 1LD 0LD 1LB 0RA": (56, 2332),
    "1RB 0LD 1LC 0RD 0LC 1LA 1RA 0RB": (25, 1459),
    "1RB 1LC 1LC 0RD 1LA 0LB 1LD 0RA": (39, 1164),
    "1RB 1LB 1RC 0LD 0RD 0RA 1LD 0LA": (20, 1153),
}


RECURRENCE = {
    # Lin-Rado examples

    "1RB 1RH 0RC 1LB 1LA 0RB": (9, 19),   # total recurrence
    "1RB 1RH 1LB 0LC 1LA 1RA": (12, 19),  # left barrier
    "1RB 1RH 1LC 1RA 1LA 0LC": (12, 20),  # right barrier

    # quasihalting champs

    "1RB 1LB 1LB 1LA":         (6, 7),    # 2-2

    "1RB 2LB 1LA 2LB 2RA 0RA": (59, 60),  # 2-3 shift
    "1RB 2LB 1RA 2LB 2LA 0RA": (43, 44),  # 2-3 sigma

    "1RB 1RC 1LC 1RA 1RA 1LA": (9, 11),    # 3-2 sigma
    "1RB 0LB 1LA 0RC 1LC 1LA": (55, 56),  # 3-2 shift
}


class TuringTest(unittest.TestCase):
    def test_halting(self):
        for prog, (sigma, shift) in HALTING.items():
            print(prog)
            machine = run_bb(prog)

            self.assertEqual(
                sigma,
                machine.marks)

            self.assertEqual(
                shift,
                machine.steps)

            self.assertEqual(
                machine.status,
                'HALTED')

    def test_quasihalting(self):
        for prog, (sigma, shift) in QUASIHALTING.items():
            print(prog)
            machine = run_bb(prog, x_limit=shift)

            self.assertEqual(
                sigma,
                machine.marks)

            self.assertEqual(
                machine.status,
                'XLIMIT')

    def test_recurrence(self):
        for prog, (pstep, step) in RECURRENCE.items():
            print(prog)
            machine = run_bb(prog, check_rec=True)

            self.assertEqual(
                machine.status,
                ('RECURR', pstep, step))
