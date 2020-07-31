import unittest

from turing import Machine


BB2 = "1RB   1LB   1LA   1RH"
BB3 = "1RB   1RH   1LB   0RC   1LC   1LA"
BB4 = "1RB   1LB   1LA   0LC   1RH   1LD   1RD   0RA"
TM5 = "1RB   0LC   1RC   1RD   1LA   0RB   0RE   1RH   1LC   1RA"

BB5 = "1RB   1LC   1RC   1RB   1RD   0LE   1LA   1LD   1RH   0LA"


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
            machine = Machine([0], prog)
            machine.run_to_halt()

            self.assertEqual(
                ones_count,
                machine.ones_count)

            self.assertEqual(
                exec_count,
                machine.exec_count)
