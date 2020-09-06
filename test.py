import unittest

from turing import (
    run_bb,
    BB_2_2,
    BB_3_2,
    BB_4_2,
    BB_5_2,
    BB_2_3,
    BB_2_4,
)


EXPECTED = {
    BB_2_2: (4, 6),
    BB_3_2: (5, 21),
    BB_2_3: (9, 38),
    BB_4_2: (13, 107),
    BB_2_4: (2050, 3932964),
    BB_5_2: (4098, 47176870),
}


class TuringTest(unittest.TestCase):
    def test_turing(self):
        for prog, (sigma, shift) in EXPECTED.items():
            print(prog)
            machine = run_bb(prog)

            self.assertEqual(
                sigma,
                machine.ones_count)

            self.assertEqual(
                shift,
                machine.exec_count)
