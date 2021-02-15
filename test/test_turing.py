# pylint: disable = attribute-defined-outside-init

from unittest import TestCase

from turing import run_bb

HALTING_FAST = {
    # 2/2 BB
    "1RB 1LB 1LA 1RH": (4, 6),

    # 3/2 BB
    "1RB 1RH 1LB 0RC 1LC 1LA": (5, 21),  # shift
    "1RB 1LC 1RC 1RH 1LA 0LB": (6, 11),  # sigma

    # 2/3 BB
    "1RB 2LB 1RH 2LA 2RB 1LB": (9, 38),

    # 4/2 BB
    "1RB 1LB 1LA 0LC 1RH 1LD 1RD 0RA": (13, 107),  # shift
    "1RB 0RC 1LA 1RA 1RH 1RD 1LD 0LB": (13,  96),  # sigma

    # 2/4 Runners-up
    "1RB 3LA 1LA 1RA 2LA 1RH 3RA 3RB": (90, 7195),
    "1RB 3LA 1LA 1RA 2LA 1RH 3LA 3RB": (84, 6445),
    "1RB 3LA 1LA 1RA 2LA 1RH 2RA 3RB": (84, 6445),
    "1RB 2RB 3LA 2RA 1LA 3RB 1RH 1LB": (60, 2351),

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


HALTING_SLOW = {
    # 3/3 Surprise-in-a-box
    "1RB 2LB 1LC 1LA 2RB 1RB 1RH 2LA 0LC": (31, 2315619),

    # 2/4 BB
    "1RB 2LA 1RA 1RA 1LB 1LA 3RB 1RH": (2050, 3932964),

    # 3/3 copy of 2/4 BB
    "1RB 1LC 1RH 1LA 1LC 2RB 1RB 2LC 1RC": (2050, 3932964),

    # 5/2 BB
    "1RB 1LC 1RC 1RB 1RD 0LE 1LA 1LD 1RH 0LA": (4098, 47176870),
}


QUASIHALTING = {
    # 2/2 (not better than BB)
    "1RB 1LB 1LB 1LA": (3, 6, 1),
    "1RB 1LB 0LB 1LA": (2, 6, 1),
    "1RB 0LB 1LB 1LA": (2, 6, 1),
    "1RB 0LB 0LB 1LA": (1, 6, 1),

    # 2/3
    "1RB 2LB 1RA 2LB 2LA 0RA": (10, 43, 1),  # BBB sigma
    "1RB 2LB 1LA 2LB 2RA 0RA":  (8, 59, 1),  # BBB shift
    "1RB 0LB 1RA 1LB 2LA 2RA":  (3, 45, 1),
    "1RB 2RA 2LB 2LB 2LA 0LA":  (5, 40, 1),
    "1RB 1LA 2RA 2LA 2LB 2RB":  (8, 17, 2),

    # 3/2
    "1RB 0LB 1LA 0RC 1LC 1LA": (6, 55, 1),  # BBB shift
    "1RB 1RC 1LC 1RA 1RA 1LA": (6,  9, 2),  # BBB sigma
    "1RB 0LB 1RC 0RC 1LC 1LA": (6, 54, 1),
    "1RB 0LC 1LB 0RC 1LC 1LA": (5, 52, 1),
    "1RB 0LC 0LC 0RC 1LC 1LA": (5, 51, 1),
    "1RB 0LC 1LA 0RC 1RC 1RB": (5, 49, 1),
    "1RB 0LC 0RC 0RC 1LC 1LA": (5, 48, 1),
    "1RB 1LB 1LA 1LC 1RC 0LC": (0, 34, 1),
    "1RB 1LC 1LB 1LA 1RC 0LC": (0, 27, 1),
    "1RB 1LB 1LA 1RC 1LC 0RC": (0, 26, 1),
    "1RB 1RC 1LC 0LB 1RA 1LA": (5, 22, 2),

    # 4/2
    "1RB 0LC 1LD 0LA 1RC 1RD 1LA 0LD": (0, 66349, 1),  # BBB shift
    "1RB 1RC 1LC 1RD 1RA 1LD 0RD 0LB": (69, 2819, 1),  # BBB sigma
    "1RB 1RA 0RC 0RB 0RD 1RA 1LD 1LB": ( 0, 2568, 1),
    "1RB 1RA 0RC 1LA 1LC 1LD 0RB 0RD": ( 0, 2512, 1),
    "1RB 1RC 1RD 0LC 1LD 0LD 1LB 0RA": (56, 2332, 3),
    "1RB 0LC 1RC 1LD 1RD 0RB 0LB 1LA": (35, 1460, 3),  # QH 1459
    "1RB 0LD 1LC 0RD 0LC 1LA 1RA 0RB": (25, 1459, 1),
    "1RB 1LC 1LC 0RD 1LA 0LB 1LD 0RA": (39, 1164, 1),
    "1RB 1LB 1RC 0LD 0RD 0RA 1LD 0LA": (20, 1153, 1),

    # 5/2
    "1RB 1LC 1LC 1RA 1LB 0LD 1LA 0RE 1RD 1RE": (504, 221032, 2),
    "1RB 1LC 1RD 1RA 1LB 0LA 1RE 0RC 1RC 0LE": (  2,   3247, 3),

    # 2/4
    "1RB 2RB 1LB 1LA 1LB 3RA 3LA 2RB": (3340, 2333909, 1),
    "1RB 2RB 3LA 2RA 1LB 1LA 1LB 3RB": (  63,   22465, 1),  # QH 22402
    "1RB 2LA 1RA 1LA 2LB 3LA 2RB 2RA": ( 107,   10459, 3),  # QH 10353
    "1RB 2LA 1RA 1LA 3LA 1LB 2RB 2RA": (  90,    7193, 2),  # QH 7106
    "1RB 2LA 1RA 1LA 3LA 1LB 2RB 2LA": (  84,    6443, 2),  # QH 6362
    "1RB 2RB 1LA 1LA 2LB 2RA 3LB 1LA": (  63,    4068, 1),  # QH 4005
}


RECURRENCE = {
    # Lin-Rado examples
    "1RB 1RH 0RC 1LB 1LA 0RB": (2,  9, 10),  # total recurrence
    "1RB 1RH 1LB 0LC 1LA 1RA": (4, 12,  7),  # left barrier
    "1RB 1RH 1LC 1RA 1LA 0LC": (4, 12,  8),  # right barrier

    # 2/2
    "1RB 0LB 1LA 0RB": (3, 9, 3),
    "1RB 1LA 0LA 1RA": (3, 7, 5),
    "1RB 1LB 1LA 0RB": (2, 7, 3),
    "1RB 0RA 1LB 1LA": (0, 0, 8),
    "1RB 0RA 0LB 1LA": (0, 0, 7),
    "1RB 1LA 0LA 0LB": (0, 0, 7),
    "1RB 0LA 1LB 1RA": (0, 0, 5),
    "1RB 1RB 1LA 0LB": (2, 3, 4),

    # 3/2
    "1RB 1LB 0RC 0LA 1LC 0LA": ( 9, 101, 24),
    "1RB 1LA 1LC 1RC 1LA 0RB": (10,  69, 16),
    "1RB 1LB 1RC 0LA 1LA 1RC": (10,  65, 16),
    "1RB 0LC 1LC 1RB 1RA 1LA": ( 9,  50, 16),
    "1RB 0LC 1LC 1RB 1RB 1LA": ( 9,  50, 12),
    "1RB 0LB 1LC 0RC 1RA 1LA": ( 6,  38, 21),
    "1RB 1LB 1LA 1RC 0RB 0LC": ( 0,  22,  4),
    "1RB 1LA 0RC 0RA 1LC 0LA": ( 4,  17, 36),
    "1RB 0RB 1LC 1RC 0LA 1LA": ( 3,  16, 15),
    "1RB 1LB 0RC 0RB 1LC 0LA": ( 3,   4, 38),
    "1RB 0LA 0RC 1LA 1LC 0RB": ( 0,   0, 92),
    "1RB 0LA 0RC 0RC 1LC 1LA": ( 0,   0, 48),
    "1RB 1LB 0RC 1LA 1LA 0RA": ( 0,   0, 21),

    # 2/3
    "1RB 0LA 0RH 1LB 2LA 0RB": (15, 165, 54),
    "1RB 0LA 0LH 1LB 2LA 0RB": (15, 165, 54),
    "1RB 0LA 1RH 1LB 2LA 0RB": (15, 165, 54),
    "1RB 0LA 1LH 1LB 2LA 0RB": (15, 165, 54),
    "1RB 0LA 0RA 1LB 2LA 0RB": (15, 165, 54),
    "1RB 0LA 0RB 1LB 2LA 0RB": (15, 165, 54),
    "1RB 0LA 0LA 1LB 2LA 0RB": (15, 165, 54),
    "1RB 0LA 0LB 1LB 2LA 0RB": (15, 165, 54),
    "1RB 0LA 1RA 1LB 2LA 0RB": (15, 165, 54),
    "1RB 0LA 1RB 1LB 2LA 0RB": (15, 165, 54),
    "1RB 0LA 1LA 1LB 2LA 0RB": (15, 165, 54),
    "1RB 0LA 1LB 1LB 2LA 0RB": (15, 165, 54),
    "1RB 0LA 2RA 1LB 2LA 0RB": (15, 165, 54),
    "1RB 0LA 2RB 1LB 2LA 0RB": (15, 165, 54),
    "1RB 0LA 2LA 1LB 2LA 0RB": (15, 165, 54),
    "1RB 0LA 2LB 1LB 2LA 0RB": (15, 165, 54),

    "1RB 1LB 2LA 1LA 2RB 0RA": (12, 101, 26),
    "1RB 2RB 1LB 1LA 2RB 0LA": (13,  97, 14),
    "1RB 2LA 0RB 1LA 1RB 1RA": (13,  94, 20),
    "1RB 2LA 0RB 1LA 2LB 1RA": (11,  89, 26),
    "1RB 1LA 1LB 1LA 2RB 0LA": (12,  80, 20),
    "1RB 2LA 0RB 1LA 2LA 1RA": (12,  78, 14),
    "1RB 2LA 0RB 1LB 2LA 1RA": (10,  76, 14),
    "1RB 2LA 0RB 1LA 0LB 1RA": ( 2,  75,  4),
    "1RB 2LB 2LA 2LA 0LB 0RA": ( 8,  63, 32),
    "1RB 0RA 2LB 2LA 2RA 0LB": ( 6,  59, 32),
    "1RB 1LB 1LB 1LA 2RB 0LA": ( 9,  58,  8),
    "1RB 2LA 2LB 1LA 2RA 0LB": ( 8,  57, 60),
    "1RB 1LA 2LB 2LA 2RA 0LB": ( 6,  57, 30),
    "1RB 2LA 0RB 1LB 1RA 1RA": ( 6,  55, 10),
    "1RB 0RB 0LB 2LA 2RA 1LB": ( 7,  54, 40),
    "1RB 2LA 1RB 1LB 1LA 2RA": ( 7,  24, 46),
    "1RB 1LA 2LB 1LA 2RA 0LB": ( 7,  20, 48),
    "1RB 2RB 2LA 1LB 1RA 0LA": ( 4,  14, 54),
    "1RB 0RB 1LA 2LA 2RA 0LB": ( 3,  10, 48),
    "1RB 0RA 1LB 2LA 2RB 0LA": ( 3,   6, 48),
    "1RB 2LA 0RB 0LB 1LA 0RA": ( 1,   2, 57),
    "1RB 2LB 0RA 1LA 2RB 2RA": ( 0,   0, 60),
    "1RB 2LA 1LB 0LA 0RB 1RA": ( 0,   0, 47),

    # 4/2
    "1RB 1RC 1LC 0RB 1LD 0RA 1RA 0LB": (51, 1727, 622),
    "1RB 0LC 1RD 0RD 1LA 0RC 1LB 1RC": (39, 1527, 522),
    "1RB 0LC 1RC 1RD 1LD 0RC 1LA 0RB": (45, 1301, 622),
    "1RB 1LC 1RD 0RB 0LC 1LA 1RC 0RA": (33, 1111, 131),
    "1RB 1RC 1LB 1LC 1RD 0LB 1RA 0RD": (30, 1033, 174),
    "1RB 0LC 1RD 0RB 1LC 1LA 1RC 1RA": (30, 1004, 174),
    "1RB 1LA 1RC 0RD 0LA 0RC 1RC 1LC": (29,  979, 144),
    "1RB 1RC 1LC 0LD 0RA 1LB 1RD 0LA": (24,  928, 128),
    "1RB 0RA 0LB 0LC 1RD 1LC 1RA 1LB": (19,  868, 404),
    "1RB 0RC 0LD 1RA 0LA 0RD 1LC 1LA": (12,  383, 200),
    "1RB 0LA 1LC 1LD 1RD 1LB 1RA 0RD": (12,   79, 481),
    "1RB 1LA 1RC 0RC 1LD 0RD 0LA 1LA": ( 7,   66, 284),
    "1RB 1RC 0RC 1RA 1LD 0RB 0LD 1LA": ( 7,   50, 597),
    "1RB 1RA 1LC 0RB 1RC 0LD 1LA 1LD": ( 8,   45, 228),
    "1RB 1LA 1LC 0RA 1LD 0LC 1RA 0LA": ( 3,    5, 385),
    "1RB 0RA 1LC 1RA 1LD 0LC 1LA 0RB": ( 3,    5, 244),
    "1RB 1RC 0LD 1RA 1LB 0RD 1LA 0RC": ( 1,    2, 294),
    "1RB 0LC 1LD 1LC 1RD 0LA 0RA 1LB": ( 0,    0, 294),
    "1RB 1LA 1LB 0RC 1LC 1LD 0RA 0LD": ( 0,    0, 238),
    "1RB 0LA 1LB 0RC 1RD 1RC 1LA 1LD": ( 0,    0, 228),
}

BLANK_TAPE = {
    # 2/2
    "1RB 0RA 1LB 1LA": 8,
    "1RB 0RA 0LB 1LA": 7,
    "1RB 1LA 0LA 0LB": 6,
    "1RB 0LA 1LB 1RA": 5,
    "1RB 1RB 1LA 0LB": 5,

    # 3/2
    "1RB 1LB 1LA 1LC 1RC 0LC": 34,
    "1RB 1LC 1LB 1LA 1RC 0LC": 27,
    "1RB 1LB 1LA 1RC 1LC 0RC": 26,
    "1RB 1LB 1LA 0LC 1RC 0LC": 25,
    "1RB 0RB 1LC 1RC 0LA 1LA": 25,
    "1RB 0RB 1LC 0LC 1LA 1RA": 23,
    "1RB 0LB 1LA 1LC 1RC 0LC": 23,
    "1RB 1LB 1LA 1RC 0RB 0LC": 22,
    "1RB 1LB 0RC 1LA 1LA 0RA": 21,
    "1RB 1LA 1LA 1RC 1LC 0RC": 20,
    "1RB 1LA 1LA 1LC 1RC 0LC": 20,
    "1RB 0LC 1LB 1LA 1RC 0LC": 20,
    "1RB 0LB 1LA 1LC 0RC 0RB": 20,
    "1RB 1RC 1LC 0LB 1RA 1LA": 16,
    "1RB 1RH 0RC 1LB 1LA 0RB": 15,
    "1RB 1LB 0LC 0RB 1RA 1LA": 14,

    # 2/3
    "1RB 2LA 0RB 1LA 0LB 1RA": 77,
    "1RB 0RB 1LA 2LA 2RA 0LB":  4,

    # 4/2
    "1RB 0LC 1LD 0LA 1RC 1RD 1LA 0LD": 66345,
    "1RB 1RA 0RC 0RB 0RD 1RA 1LD 1LB":  2566,
    "1RB 1RA 0RC 1LA 1LC 1LD 0RB 0RD":  2510,
    "1RB 1LC 1RC 0RD 0RD 0RC 1LD 1LA":   704,
    "1RB 1LC 0RC 0RD 0RD 0RC 1LD 1LA":   702,
    "1RB 1LA 0LC 0LB 1RC 1RD 1LA 1RB":   495,
    "1RB 1LC 0RC 1RB 0RD 0RC 1LD 1LA":   455,
    "1RB 1RA 0RC 0RB 1LC 1LD 1RA 1LB":   426,
    "1RB 1RA 1LC 0RD 1LB 1LD 1RA 0RB":   319,
    # constructed from BB(3) sigma champ
    "1RB 1LC 1RC 1LD 1LA 0LB 1RD 0LD":    77,
    # constructed from BB(3) shift champ
    "1RB 1LD 1LB 0RC 1LC 1LA 1RD 0LD":    66,
    "1RB 0RA 0LB 0LC 1RD 1LC 1RA 1LB":     3,
    "1RB 1RC 0LD 1RA 1LB 0RD 1LA 0RC":     3,

    # 5/2
    "1RB 1LC 1RD 1RA 1LB 0LA 1RE 0RC 1RC 0LE": 3241,
    "1RB 1RC 1LD 0RA 0RB 1RA 1LE 1LD 1RA 0RE":  725,
    "1RB 1LC 1RC 1LD 1RE 1RD 0RE 0LA 1LB 0LE":  362,
    "1RB 1RC 0RC 1LD 1LE 1RD 0LE 1RA 1LB 0LC":  277,
    "1RB 1LC 1LD 0LB 1RA 1RC 0RE 0LD 0RA 0RH":  182,
    "1RB 1LC 1LC 0LD 1LE 1LD 1RE 0RA 0RB 1LA":  134,
    "1RB 1LC 1LD 1RA 0LB 1LE 0LE 1LC 1RA 0RE":  127,
    "1RB 1LC 1RD 0RE 1LA 1LE 1RC 0LE 1RE 0LD":  123,
    "1RB 0RH 0LB 1RC 0LC 1RD 1LE 0LD 0RA 0LE":   63,
    "1RB 1RA 1RC 0RE 1LD 0RA 1LB 1LD 0RH 0RB":   28,
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

    def _test_halting(self, prog_data):
        for prog, (marks, steps) in prog_data.items():
            self.run_bb(prog, check_blanks=False)

            self.assert_marks(marks)
            self.assert_steps(steps)

            self.assert_final(('HALTED', steps, None))

    def test_halting_fast(self):
        self._test_halting(HALTING_FAST)

    def test_halting_slow(self):
        self._test_halting(HALTING_SLOW)

    def _test_recurrence(self, prog_data, final):
        for prog, (marks, steps, period) in prog_data.items():
            self.run_bb(
                prog,
                tape=[0] * 183,
                check_rec=(
                    0
                    if steps < 256 else
                    steps),
                check_blanks=False)

            self.assert_final((final, steps, period))

            self.run_bb(
                prog,
                x_limit=steps,
                print_prog=False,
                check_blanks=False)

            self.assert_marks(marks)

            self.assert_final(('XLIMIT', steps, None))

    def test_quasihalting(self):
        self._test_recurrence(QUASIHALTING, 'QSIHLT')

    def test_recurrence(self):
        self._test_recurrence(RECURRENCE, 'RECURR')

    def test_blank_tape(self):
        for prog, steps in BLANK_TAPE.items():
            self.run_bb(prog)

            self.assert_steps(steps)
            self.assert_final(('BLANKS', steps, None))
