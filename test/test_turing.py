# pylint: disable = attribute-defined-outside-init

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

    # Milton Green (1964)
    "1RB 1LA 0LH 1RB": (1, 2),
    "1RB 1LH 0RC 1RC 0RD 0RC 1RE 1LA 0RF 0RE 1LF 1LD": (35, 436),

    # Lynn (1971)
    "1RB 1RA 1LC 0LD 0RA 1LB 1RH 0LE 1RC 1RB": (15, 435),
    "1RB 1RC 1LC 1LD 0RA 1LB 1RE 0LB 1RH 1RD": (22, 292),
    "1RB 0RC 1LC 0LB 1RD 1LB 1RE 0RA 0RB 1RH": (22, 217),
    # Lynn reports 522 steps
    "1RB 0LB 1LC 1RH 0LD 0LC 1LE 0RA 0LF 0LE 1RF 1RD": (42, 521),

    # Uwe (1981)

    # Castor diligentissimus et primus et perpetuus (Castor schultis)
    "1RB 0LC 1RC 1RD 1LA 0RB 0RE 1RH 1LC 1RA": (501, 134467),

    # Castor ministerialis: the Civil Servant Beaver, who cares most
    # for his progress, but does not produce anything.
    "1RB 1RA 1RC 0RE 1LD 0RA 1LB 1LD 0RH 0RB": (0, 52),

    # Castor scientificus: the Scientific Beaver, who does not produce
    # anything either, but with more effort and less effect on his
    # position.
    "0RB 0LA 0RC 0RH 1RD 1LE 1LA 0LD 1RC 1RE": (0, 187),

    # Castor exflippus: the Beaver Freak, who tries to survive as long
    # as possible without producing anything, moving on the tape, and
    # changing his state.
    "0RB 0LA 1RC 0RH 0LC 1RD 0LD 1RE 1LA 0LE": (0, 67),
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
    "1RB 1RH 0RC 1LB 1LA 0RB": ( 9, 10),  # total recurrence
    "1RB 1RH 1LB 0LC 1LA 1RA": (12,  7),  # left barrier
    "1RB 1RH 1LC 1RA 1LA 0LC": (12,  8),  # right barrier

    # 2-2
    "1RB 0LB 1LA 0RB": (9, 3),
    "1RB 1LA 0LA 1RA": (7, 5),
    "1RB 1LB 1LA 0RB": (7, 3),

    # 3-2
    "1RB 1LB 0RC 0LA 1LC 0LA": (101, 24),
    "1RB 1LA 1LC 1RC 1LA 0RB": ( 69, 16),
    "1RB 1LB 1RC 0LA 1LA 1RC": ( 65, 16),
    "1RB 0LC 1LC 1RB 1RA 1LA": ( 50, 16),
    "1RB 0LC 1LC 1RB 1RB 1LA": ( 50, 12),
    "1RB 0LB 1LC 0RC 1RA 1LA": ( 38, 21),
    "1RB 1LA 0RC 0RA 1LC 0LA": ( 17, 36),
    "1RB 1LB 0RC 0RB 1LC 0LA": (  4, 38),
    "1RB 0LA 0RC 1LA 1LC 0RB": (  0, 92),
    "1RB 0LA 0RC 0RC 1LC 1LA": (  0, 48),

    # 2-3
    "1RB 0LA 0RH 1LB 2LA 0RB": (165, 54),
    "1RB 0LA 0LH 1LB 2LA 0RB": (165, 54),
    "1RB 0LA 1RH 1LB 2LA 0RB": (165, 54),
    "1RB 0LA 1LH 1LB 2LA 0RB": (165, 54),
    "1RB 0LA 0RA 1LB 2LA 0RB": (165, 54),
    "1RB 0LA 0RB 1LB 2LA 0RB": (165, 54),
    "1RB 0LA 0LA 1LB 2LA 0RB": (165, 54),
    "1RB 0LA 0LB 1LB 2LA 0RB": (165, 54),
    "1RB 0LA 1RA 1LB 2LA 0RB": (165, 54),
    "1RB 0LA 1RB 1LB 2LA 0RB": (165, 54),
    "1RB 0LA 1LA 1LB 2LA 0RB": (165, 54),
    "1RB 0LA 1LB 1LB 2LA 0RB": (165, 54),
    "1RB 0LA 2RA 1LB 2LA 0RB": (165, 54),
    "1RB 0LA 2RB 1LB 2LA 0RB": (165, 54),
    "1RB 0LA 2LA 1LB 2LA 0RB": (165, 54),
    "1RB 0LA 2LB 1LB 2LA 0RB": (165, 54),

    "1RB 1LB 2LA 1LA 2RB 0RA": (101, 26),
    "1RB 2RB 1LB 1LA 2RB 0LA": ( 97, 14),
    "1RB 2LA 0RB 1LA 1RB 1RA": ( 94, 20),
    "1RB 2LA 0RB 1LA 2LB 1RA": ( 89, 26),
    "1RB 1LA 1LB 1LA 2RB 0LA": ( 80, 20),
    "1RB 2LA 0RB 1LA 2LA 1RA": ( 78, 14),
    "1RB 2LA 0RB 1LB 2LA 1RA": ( 76, 14),
    "1RB 2LA 0RB 1LA 0LB 1RA": ( 75,  4),
    "1RB 2LB 2LA 2LA 0LB 0RA": ( 63, 32),
    "1RB 0RA 2LB 2LA 2RA 0LB": ( 59, 32),
    "1RB 1LB 1LB 1LA 2RB 0LA": ( 58,  8),
    "1RB 2LA 2LB 1LA 2RA 0LB": ( 57, 60),
    "1RB 1LA 2LB 2LA 2RA 0LB": ( 57, 30),
    "1RB 2LA 0RB 1LB 1RA 1RA": ( 55, 10),
    "1RB 0RB 0LB 2LA 2RA 1LB": ( 54, 40),
    "1RB 2LA 1RB 1LB 1LA 2RA": ( 24, 46),
    "1RB 1LA 2LB 1LA 2RA 0LB": ( 20, 48),
    "1RB 2RB 2LA 1LB 1RA 0LA": ( 14, 54),
    "1RB 0RB 1LA 2LA 2RA 0LB": ( 10, 48),
    "1RB 0RA 1LB 2LA 2RB 0LA": (  6, 48),
    "1RB 2LA 0RB 0LB 1LA 0RA": (  2, 57),
    "1RB 2LB 0RA 1LA 2RB 2RA": (  0, 60),
    "1RB 2LA 1LB 0LA 0RB 1RA": (  0, 47),

    # 4-2
    "1RB 1RC 1LC 0RB 1LD 0RA 1RA 0LB": (1727, 622),
    "1RB 0LC 1RD 0RD 1LA 0RC 1LB 1RC": (1527, 522),
    "1RB 0LC 1RC 1RD 1LD 0RC 1LA 0RB": (1301, 622),
    "1RB 1LC 1RD 0RB 0LC 1LA 1RC 0RA": (1111, 131),
    "1RB 1RC 1LB 1LC 1RD 0LB 1RA 0RD": (1033, 174),
    "1RB 0LC 1RD 0RB 1LC 1LA 1RC 1RA": (1004, 174),
    "1RB 1LA 1RC 0RD 0LA 0RC 1RC 1LC": ( 979, 144),
    "1RB 1RC 1LC 0LD 0RA 1LB 1RD 0LA": ( 928, 128),
    "1RB 0RA 0LB 0LC 1RD 1LC 1RA 1LB": ( 868, 404),
    "1RB 0LA 1LC 1LD 1RD 1LB 1RA 0RD": (  79, 481),
    "1RB 1LA 1RC 0RC 1LD 0RD 0LA 1LA": (  66, 284),
    "1RB 1RC 0RC 1RA 1LD 0RB 0LD 1LA": (  50, 597),
    "1RB 1RA 1LC 0RB 1RC 0LD 1LA 1LD": (  45, 228),
    "1RB 1LA 1LC 0RA 1LD 0LC 1RA 0LA": (   5, 385),
    "1RB 0RA 1LC 1RA 1LD 0LC 1LA 0RB": (   5, 244),
    "1RB 1RC 0LD 1RA 1LB 0RD 1LA 0RC": (   2, 294),
    "1RB 0LC 1LD 1LC 1RD 0LA 0RA 1LB": (   0, 294),
    "1RB 1LA 1LB 0RC 1LC 1LD 0RA 0LD": (   0, 238),
    "1RB 0LA 1LB 0RC 1RD 1RC 1LA 1LD": (   0, 228),
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

        self.machine = run_bb(prog, **opts)

    def test_halting(self):
        for prog, (marks, steps) in HALTING.items():
            self.run_bb(prog)

            self.assert_marks(marks)
            self.assert_steps(steps)

            self.assert_final('HALTED')

    def test_quasihalting(self):
        for prog, (marks, steps, period) in QUASIHALTING.items():
            self.run_bb(
                prog,
                check_rec=(
                    0
                    if steps < 256 else
                    steps))

            self.assert_final(('QSIHLT', steps, period))

            self.run_bb(
                prog,
                x_limit=steps,
                print_prog=False)

            self.assert_marks(marks)

            self.assert_final('XLIMIT')

    def test_recurrence(self):
        for prog, (start, period) in RECURRENCE.items():
            self.run_bb(
                prog,
                check_rec=(
                    0
                    if start < 256 else
                    start))

            self.assert_final(('RECURR', start, period))
