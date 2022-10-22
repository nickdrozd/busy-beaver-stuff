# pylint: disable = attribute-defined-outside-init
from unittest import TestCase
from typing import List, Tuple

from tm import Machine
from tm.tape import BlockTape, Signature

def stringify_sig(sig: Signature) -> str:
    lspan, scan, rspan = sig

    l_sig = '|'.join(str(c) for c in lspan)
    r_sig = '|'.join(reversed([str(c) for c in rspan]))

    return f'{l_sig}[{scan}]{r_sig}'


class TestTape(TestCase):
    def run_bb(self, prog, **opts):
        self.machine = Machine(prog).run(
            watch_tape = True,
            **opts)

        print(self.machine)

        self.tape = self.machine.tape

    def assert_signature(self, expected: str, tape = None):
        self.assertEqual(
            stringify_sig((tape or self.tape).signature),
            expected)

    def assert_string(self, expected: str, tape = None):
        self.assertEqual(
            str(tape or self.tape),
            expected)

    def assert_head(self, expected: int, tape = None):
        self.assertEqual(
            (tape or self.tape).head,
            expected)

    def assert_ptr_tape(self, expected: List[int]):
        self.assertEqual(
            self.ptr.tape,
            expected)

    def assert_ptr_positions(self, expected: Tuple[int, int, int]):
        self.assertEqual(
            (self.ptr.l_end, self.ptr.init, self.ptr.r_end),
            expected)

        self.assertEqual(
            abs(self.ptr.l_end) + self.ptr.r_end,
            len(self.ptr.tape))

    def test_blank(self):
        self.run_bb(
            "1RB 1LB  1LA 1LC  1RC 0LC")

        self.assert_signature(
            '[0]0')

    def test_copy(self):
        self.run_bb(
            "1RB 2LA 1R_  1LB 1LA 0RA")

        self.assert_signature(
            '1|0|1[2]2|1')

        copy_1 = self.tape.copy()
        copy_2 = self.tape.copy()

        _ = copy_1.step(0, 2, False)
        _ = copy_2.step(1, 1, False)

        self.assert_signature(
            '1|0|1[2]2|1')

        self.assert_signature(
            '1|0[1]2|1',
            tape = copy_1)

        self.assert_signature(
            '1|0|1[2]1',
            tape = copy_2)

    def test_slice(self):
        self.run_bb(
            "1RB 2LB 1LA  2LB 2RA 0RA")

        self.assert_string(
            "[0] 2^1 1^7")

        self.assert_head(
            -3)

        self.ptr = ptr = self.tape.to_ptr()

        init_tape = [0, 2, 1, 1, 1, 1, 1, 1, 1]

        ########################################

        self.assert_ptr_tape(
            init_tape)

        self.assert_ptr_positions(
            (-3, 3, 6))

        ########################################

        self.assertEqual(
            ptr[-3:0],
            [0, 2, 1])

        self.assertEqual(
            ptr[0:3],
            [1, 1, 1])

        self.assert_ptr_tape(
            init_tape)

        self.assert_ptr_positions(
            (-3, 3, 6))

        ########################################

        self.assertEqual(
            ptr[-3:3],
            [0, 2, 1, 1, 1, 1])

        self.assert_ptr_tape(
            init_tape)

        self.assert_ptr_positions(
            (-3, 3, 6))

        ########################################

        self.assertEqual(
            ptr[-3:4],
            [0, 2, 1, 1, 1, 1, 1])

        self.assert_ptr_tape(
            init_tape + [0])

        self.assert_ptr_positions(
            (-3, 3, 7))

        ########################################

        self.assertEqual(
            ptr[-3:6],
            [0, 2, 1, 1, 1, 1, 1, 1, 1])

        self.assert_ptr_tape(
            init_tape + [0, 0, 0])

        self.assert_ptr_positions(
            (-3, 3, 9))

        ########################################

        self.assertEqual(
            ptr[-3:7],
            [0, 2, 1, 1, 1, 1, 1, 1, 1, 0])

        self.assert_ptr_tape(
            init_tape + [0, 0, 0, 0])

        self.assert_ptr_positions(
            (-3, 3, 10))

        ########################################

        self.assertEqual(
            ptr[-5:10],
            [0, 0, 0, 2, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0])

        self.assert_ptr_tape(
            [0, 0] + init_tape + [0, 0, 0, 0, 0, 0, 0])

        self.assert_ptr_positions(
            (-5, 5, 13))

    def test_trace_blocks(self):
        # 1RB 1LC  1RD 1RB  0RD 0RC  1LD 1LA : BBB(4, 2)
        #    49 |   144 | D1 | 1^15 [1] 1^6
        #    54 |   167 | D1 | 1^12 [1] 1^11

        tape = BlockTape([[1, 15]], 1, [[1, 6]])

        self.assertEqual(
            [1, 15],
            tape.lspan[0])

        self.assertEqual(
            [1, 6],
            tape.rspan[0])

        tape.lspan[0].append(0)
        tape.rspan[0].append(0)

        tape.step(0, 1, False)
        tape.step(0, 1, False)
        tape.step(1, 0, True)
        tape.step(1, 0, False)
        tape.step(0, 1, True)

        self.assertEqual(
            [1, 12, 0],
            tape.lspan[0])

        self.assertEqual(
            [1, 11, 0],
            tape.rspan[0])
