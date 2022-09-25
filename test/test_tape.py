# pylint: disable = attribute-defined-outside-init
from unittest import TestCase

from tm import Machine

class TestTape(TestCase):
    def run_bb(self, prog, **opts):
        self.machine = Machine(prog).run(**opts)
        self.tape = self.machine.tape

    def test_blank(self):
        self.run_bb(
            "1RB 1LB  1LA 1LC  1RC 0LC",
            watch_tape = True)

        print(self.machine)

        self.assertEqual(
            self.tape.signature,
            '[0]0')

    def test_copy(self):
        self.run_bb(
            "1RB 2LA 1R_  1LB 1LA 0RA",
            watch_tape = True)

        self.assertEqual(
            self.tape.signature,
            '1|0|1[2]2|1')

        print(self.machine)

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
