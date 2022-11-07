# pylint: disable = attribute-defined-outside-init
from unittest import TestCase

from tm import Machine
from tm.tape import BlockTape, Signature, Color, Span

def stringify_sig(sig: Signature) -> str:
    scan, lspan, rspan = sig

    l_sig = '|'.join(str(c) for c in lspan)
    r_sig = '|'.join(reversed([str(c) for c in rspan]))

    return f'{l_sig}[{scan}]{r_sig}'


class TestTape(TestCase):
    def run_bb(self, prog: str, **opts) -> None:
        self.machine = Machine(prog).run(
            watch_tape = True,
            **opts)

        print(self.machine)

        self.tape = self.machine.tape

    def assert_signature(
            self,
            expected: str,
            tape: BlockTape | None = None,
    ) -> None:
        if tape is None:
            assert self.tape is not None
            tape = self.tape

        self.assertEqual(
            stringify_sig(
                tape.signature),
            expected)

    def assert_tape(
            self,
            lspan: Span,
            scan: Color,
            rspan: Span,
    ):
        self.assertEqual(self.lspan, lspan)
        self.assertEqual(self.scan, scan)
        self.assertEqual(self.rspan, rspan)

    @property
    def scan(self) -> Color:
        assert self.tape is not None
        return self.tape.scan

    @property
    def lspan(self) -> Span:
        assert self.tape is not None
        return self.tape.lspan

    @property
    def rspan(self) -> Span:
        assert self.tape is not None
        return self.tape.rspan

    def step(self, shift: int, color: int, skip: int) -> None:
        assert self.tape is not None
        self.tape.step(shift, color, bool(skip))

    def test_blank(self):
        self.run_bb(
            "1RB 1LC  1RC 1LD  1LA 0LB  1RD 0LD",
            prover = 10)

        self.assert_signature(
            '[0]0')

    def test_copy(self):
        self.run_bb(
            "1RB 2LA 1R_  1LB 1LA 0RA")

        self.assert_signature(
            '1|0|1[2]2|1')

        assert self.tape is not None
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

    def test_trace_blocks_1(self):
        # 1RB 1LC  1RD 1RB  0RD 0RC  1LD 1LA : BBB(4, 2)
        #    49 |   144 | D1 | 1^15 [1] 1^6
        #    54 |   167 | D1 | 1^12 [1] 1^11

        self.tape = BlockTape([[1, 15]], 1, [[1, 6]])

        self.assert_tape([[1, 15]], 1, [[1,  6]])

        self.lspan[0].append(0)
        self.rspan[0].append(0)

        self.step(0, 1, 0)
        self.step(0, 1, 0)
        self.step(1, 0, 1)
        self.step(1, 0, 0)
        self.step(0, 1, 1)

        self.assert_tape([[1, 12, 0]], 1, [[1, 11, 0]])

    def test_trace_blocks_2(self):
        # 1RB 1LA  0LA 0RB: counter
        #    43 |    51 | B0 | 1^4 [0]
        #    44 |    52 | A1 | 1^3 [1] 0^1
        #    45 |    56 | A0 | [0] 1^4 0^1
        #    46 |    57 | B1 | 1^1 [1] 1^3 0^1
        #    47 |    61 | B0 | 1^1 0^4 [0]
        #    Applying 3 times: ((1, -1), ())
        #    48 |    64 | B0 | 1^4 0^1 [0]
        #    49 |    65 | A0 | 1^4 [0] 0^1
        #    50 |    66 | B0 | 1^5 [0]

        self.tape = BlockTape([[1, 4]], 0, [])

        self.assert_tape([[1, 4]], 0, [])

        self.lspan[0].append(0)

        self.assert_tape([[1, 4, 0]], 0, [])

        self.step(0, 0, 0)
        self.step(0, 1, 1)
        self.step(1, 1, 0)
        self.step(1, 0, 1)

        self.assert_tape([[1, 1], [0, 4, 0]], 0, [])

        self.lspan[0][1] += 3
        self.lspan[1][1] -= 3

        self.assert_tape([[1, 4], [0, 1, 0]], 0, [])

        self.step(0, 0, 0)

        self.assert_tape([[1, 4]], 0, [[0, 1, 0]])

        self.step(1, 1, 0)

        self.assert_tape([[1, 5, 0]], 0, [])
