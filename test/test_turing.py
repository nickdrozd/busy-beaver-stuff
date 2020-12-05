from unittest import TestCase

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
    "1RB 1LB 1LB 1LA": (3, 6, 1),
    "1RB 1LB 0LB 1LA": (2, 6, 1),
    "1RB 0LB 1LB 1LA": (2, 6, 1),
    "1RB 0LB 0LB 1LA": (1, 6, 1),

    # BBB_2_3_sigma
    "1RB 2LB 1RA 2LB 2LA 0RA": (10, 43, 1),
    # BBB_2_3_shift
    "1RB 2LB 1LA 2LB 2RA 0RA":  (8, 59, 1),
    "1RB 0LB 1RA 1LB 2LA 2RA":  (3, 45, 1),
    "1RB 2RA 2LB 2LB 2LA 0LA":  (5, 40, 1),
    "1RB 1LA 2RA 2LA 2LB 2RB":  (8, 17, 2),

    # BBB_3_2 shift
    "1RB 0LB 1LA 0RC 1LC 1LA": (6, 55, 1),
    # BBB_3_2 sigma
    "1RB 1RC 1LC 1RA 1RA 1LA": (6,  9, 2),
    "1RB 0LB 1RC 0RC 1LC 1LA": (6, 54, 1),
    "1RB 0LC 1LB 0RC 1LC 1LA": (5, 52, 1),
    "1RB 0LC 0LC 0RC 1LC 1LA": (5, 51, 1),
    "1RB 0LC 1LA 0RC 1RC 1RB": (5, 49, 1),
    "1RB 0LC 0RC 0RC 1LC 1LA": (5, 48, 1),
    "1RB 1RC 1LC 0LB 1RA 1LA": (5, 22, 2),

    # BBB_4_2_shift
    "1RB 0LC 1LD 0LA 1RC 1RD 1LA 0LD": (0, 66349, 1),
    # BBB_4_2_sigma
    "1RB 1RC 1LC 1RD 1RA 1LD 0RD 0LB": (69, 2819, 1),
    "1RB 1RA 0RC 0RB 0RD 1RA 1LD 1LB": ( 0, 2568, 1),
    "1RB 1RA 0RC 1LA 1LC 1LD 0RB 0RD": ( 0, 2512, 1),
    "1RB 1RC 1RD 0LC 1LD 0LD 1LB 0RA": (56, 2332, 3),
    "1RB 0LC 1RC 1LD 1RD 0RB 0LB 1LA": (35, 1460, 3),  # QH 1459
    "1RB 0LD 1LC 0RD 0LC 1LA 1RA 0RB": (25, 1459, 1),
    "1RB 1LC 1LC 0RD 1LA 0LB 1LD 0RA": (39, 1164, 1),
    "1RB 1LB 1RC 0LD 0RD 0RA 1LD 0LA": (20, 1153, 1),

    # 5_2
    "1RB 1LC 1LC 1RA 1LB 0LD 1LA 0RE 1RD 1RE": (504, 221032, 2),
}


RECURRENCE = {
    # Lin-Rado examples
    "1RB 1RH 0RC 1LB 1LA 0RB": (9, 19),   # total recurrence
    "1RB 1RH 1LB 0LC 1LA 1RA": (12, 19),  # left barrier
    "1RB 1RH 1LC 1RA 1LA 0LC": (12, 20),  # right barrier
}


class TuringTest(TestCase):
    def assert_marks(self, marks):
        self.assertEqual(
            self.machine.marks,
            marks)

    def assert_steps(self, steps):
        self.assertEqual(
            self.machine.steps,
            steps)

    def assert_final(self, final):
        self.assertEqual(
            self.machine.final,
            final)

    def run_bb(self, prog, print_prog=True, **opts):
        if print_prog:
            print(prog)

        # pylint: disable = attribute-defined-outside-init
        self.machine = run_bb(prog, **opts)

    def test_halting(self):
        for prog, (marks, steps) in HALTING.items():
            self.run_bb(prog)

            self.assert_marks(marks)
            self.assert_steps(steps)

            self.assert_final('HALTED')

    def test_quasihalting(self):
        for prog, (marks, steps, repeat) in QUASIHALTING.items():
            self.run_bb(
                prog,
                check_rec=(
                    0
                    if steps < 256 else
                    steps - 32))

            self.assert_final(('QSIHLT', steps, steps + repeat))

            self.run_bb(
                prog,
                x_limit=steps,
                print_prog=False)

            self.assert_marks(marks)

            self.assert_final('XLIMIT')

    def test_recurrence(self):
        for prog, (first, repeat) in RECURRENCE.items():
            self.run_bb(prog, check_rec=0)

            self.assert_final(('RECURR', first, repeat))
