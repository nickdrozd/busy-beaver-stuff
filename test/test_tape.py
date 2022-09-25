# pylint: disable = attribute-defined-outside-init
from unittest import TestCase

from tm import Machine

class TestTape(TestCase):
    def run_bb(self, prog, **opts):
        self.machine = Machine(prog).run(
            watch_tape = True,
            **opts)

        print(self.machine)

        self.tape = self.machine.tape

    def test_blank(self):
        self.run_bb(
            "1RB 1LB  1LA 1LC  1RC 0LC")

        self.assertEqual(
            self.tape.signature,
            '[0]0')

    def test_copy(self):
        self.run_bb(
            "1RB 2LA 1R_  1LB 1LA 0RA")

        self.assertEqual(
            self.tape.signature,
            '1|0|1[2]2|1')

        copy_1 = self.tape.copy()
        copy_2 = self.tape.copy()

        _ = copy_1.step(0, 2)
        _ = copy_2.step(1, 1)

        self.assertEqual(
            self.tape.signature,
            '1|0|1[2]2|1')

        self.assertEqual(
            copy_1.signature,
            '1|0[1]2|1')

        self.assertEqual(
            copy_2.signature,
            '1|0|1[2]1')

    def test_slice(self):
        self.run_bb("1RB 2LB 1LA  2LB 2RA 0RA")

        self.assertEqual(
            str(self.tape),
            "[0] 2^1 1^7")

        self.assertEqual(
            self.tape.head,
            -3)

        ptr = self.tape.to_ptr()

        self.assertEqual(
            ptr.tape,
            [0, 2, 1, 1, 1, 1, 1, 1, 1])

        self.assertEqual(
            (ptr.l_end, ptr.init, ptr.r_end),
            (-3, 3, 6))

        self.assertEqual(
            ptr[-3:0],
            [0, 2, 1])

        self.assertEqual(
            (ptr.l_end, ptr.init, ptr.r_end),
            (-3, 3, 6))

        self.assertEqual(
            ptr[-3:3],
            [0, 2, 1, 1, 1, 1])

        self.assertEqual(
            (ptr.l_end, ptr.init, ptr.r_end),
            (-3, 3, 6))

        self.assertEqual(
            ptr[-3:4],
            [0, 2, 1, 1, 1, 1, 1])

        self.assertEqual(
            (ptr.l_end, ptr.init, ptr.r_end),
            (-3, 3, 7))

        self.assertEqual(
            ptr[-3:6],
            [0, 2, 1, 1, 1, 1, 1, 1, 1])

        self.assertEqual(
            (ptr.l_end, ptr.init, ptr.r_end),
            (-3, 3, 9))

        self.assertEqual(
            ptr[-5:10],
            [0, 0, 0, 2, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0])

        self.assertEqual(
            (ptr.l_end, ptr.init, ptr.r_end),
            (-5, 5, 13))
