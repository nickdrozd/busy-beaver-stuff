import unittest

from turing import run_bb, BB2, BB3, BB4, TM5, BB5


EXPECTED = {
    BB2: (4, 6),
    BB3: (5, 21),
    BB4: (13, 107),
    TM5: (501, 134467),
    BB5: (4098, 47176870),
}


class TuringTest(unittest.TestCase):
    def test_turing(self):
        for prog, (ones_count, exec_count) in EXPECTED.items():
            machine = run_bb(prog)

            self.assertEqual(
                ones_count,
                machine.ones_count)

            self.assertEqual(
                exec_count,
                machine.exec_count)
