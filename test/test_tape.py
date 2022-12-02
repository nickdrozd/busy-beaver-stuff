from unittest import TestCase

from tm import Machine
from tm.tape import BlockTape, Signature, Color, Span

def stringify_sig(sig: Signature) -> str:
    scan, lspan, rspan = sig

    l_sig = '|'.join(str(c) for c in lspan)
    r_sig = '|'.join(reversed([str(c) for c in rspan]))

    return f'{l_sig}[{scan}]{r_sig}'


class TestTape(TestCase):
    tape: BlockTape
    machine: Machine

    init_tags: int

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
            tape = self.tape

        self.assertEqual(
            stringify_sig(
                tape.signature),
            expected)

    def count_tags(self) -> int:
        return (
            (1 if self.scan_info is not None else 0)
            + sum(1 for block in self.lspan if len(block) > 2)
            + sum(1 for block in self.rspan if len(block) > 2)
        )

    def set_tape(
            self,
            lspan: Span,
            scan: Color | tuple[Color, int],
            rspan: Span,
    ):
        if isinstance(scan, tuple):
            scan, scan_info = scan
        else:
            scan_info = None

        self.tape = BlockTape(lspan, scan, list(reversed(rspan)))

        if scan_info is not None:
            self.tape.scan_info = scan_info

        self.init_tags = self.count_tags()

    def assert_tape(
            self,
            lspan: Span,
            scan: Color | tuple[Color, int],
            rspan: Span,
    ):
        self.assertEqual(self.lspan, lspan)
        self.assertEqual(self.rspan, list(reversed(rspan)))

        if isinstance(scan, Color):
            self.assertIsNone(self.scan_info)
            self.assertEqual(scan, self.scan)
        else:
            self.assertEqual(
                scan,
                (
                    self.scan,
                    self.scan_info,
                ))

        self.assertGreaterEqual(
            self.init_tags,
            self.count_tags())

    @property
    def scan(self) -> Color:
        return self.tape.scan

    @property
    def scan_info(self) -> Color | None:
        return self.tape.scan_info

    @property
    def lspan(self) -> Span:
        return self.tape.lspan

    @property
    def rspan(self) -> Span:
        return self.tape.rspan

    def step(self, shift: int, color: int, skip: int) -> None:
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

        self.set_tape([[1, 15, 0]], 1, [[1, 6, 0]])

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

        self.set_tape([[1, 4, 0]], 0, [])

        self.step(0, 0, 0)
        self.step(0, 1, 1)
        self.step(1, 1, 0)
        self.step(1, 0, 1)

        self.assert_tape([[1, 1], [0, 4, 0]], 0, [])

        self.lspan[0][1] += 3
        self.lspan[1][1] -= 3

        self.assert_tape([[1, 4], [0, 1, 0]], 0, [])

        self.step(0, 0, 0)

        self.assert_tape([[1, 4]], (0, 0), [[0, 1]])

        self.step(1, 1, 0)

        self.assert_tape([[1, 5, 0]], 0, [])

    def test_trace_blocks_3(self):
        # 1RB 0RB 2LA  1LA 2RB 1LB: counter
        #    73 |   421 | B0 | 2^31 [0]
        #    74 |   422 | A2 | 2^30 [2] 1^1
        #    75 |   453 | A0 | [0] 2^31 1^1
        #    76 |   454 | B2 | 1^1 [2] 2^30 1^1
        #    77 |   455 | B1 | [1] 1^1 2^30 1^1
        # Applying 14 times: ((), (0, -2, 4))
        #    78 |   469 | B1 | [1] 1^57 2^2 1^1
        #    79 |   527 | B2 | 2^58 [2] 2^1 1^1
        #    80 |   586 | B0 | [0] 1^59 2^1 1^1
        #    81 |   587 | A0 | [0] 1^60 2^1 1^1
        #    82 |   588 | B1 | 1^1 [1] 1^59 2^1 1^1

        #    83 |   648 | B2 | 1^1 2^60 [2] 1^1

        #    84 |   709 | B1 | [1] 1^62
        #    85 |   772 | B0 | 2^63 [0]

        self.set_tape([[2, 31, 0]], 0, [])

        self.step(0, 1, 0)
        self.step(0, 2, 1)

        self.assert_tape([], 0, [[2, 31, 0], [1, 1]])

        self.step(1, 1, 0)
        self.step(0, 1, 0)

        self.assert_tape([], 1, [[1, 1], [2, 30, 0], [1, 1]])

        self.rspan[2][1] = 57
        self.rspan[1][1] = 2

        self.assert_tape([], 1, [[1, 57], [2, 2, 0], [1, 1]])

        self.step(1, 2, 1)
        self.step(0, 1, 1)

        self.assert_tape([], 0, [[1, 59], [2, 1, 0], [1, 1]])

        self.step(0, 1, 0)
        self.step(1, 1, 0)

        self.assert_tape([[1, 1]], 1, [[1, 59], [2, 1, 0], [1, 1]])

        self.step(1, 2, 1)

        self.assert_tape([[1, 1], [2, 60]], (2, 0), [[1, 1]])

        self.step(0, 1, 1)

        self.assert_tape([], 1, [[1, 62, 0]])

        self.step(1, 2, 1)

        self.assert_tape([[2, 63, 0]], 0, [])

    def test_trace_blocks_4(self):
        # Lynn exception
        # 1RB 1RA  1LC 0LD  0RA 1LB  1R_ 0LE  1RC 1RB
        #    50 | 54 | C0 | 1^2 [0] 0^1 1^4
        #    51 | 55 | A0 | 1^2 0^1 [0] 1^4
        #    52 | 56 | B1 | 1^2 0^1 1^1 [1] 1^3
        #    53 | 57 | D1 | 1^2 0^1 [1] 0^1 1^3
        #    54 | 58 | E0 | 1^2 [0] 0^2 1^3
        #    55 | 59 | C0 | 1^3 [0] 0^1 1^3

        self.set_tape([[1, 2, 0]], 0, [[0, 1, 0], [1, 4, 0]])

        self.step(1, 0, 0)

        self.assert_tape(
            [[1, 2, 0], [0, 1]], (0, 0), [[1, 4, 0]])

        self.step(1, 1, 0)

        self.assert_tape(
            [[1, 2, 0], [0, 1], [1, 1]], 1, [[1, 3, 0]])

        self.step(0, 0, 0)

        self.assert_tape(
            [[1, 2, 0], [0, 1]], 1, [[0, 1], [1, 3, 0]])

        self.step(0, 0, 0)

        self.assert_tape(
            [[1, 2, 0]], 0, [[0, 2], [1, 3, 0]])

        self.step(1, 1, 0)

        self.assert_tape(
            [[1, 3, 0]], 0, [[0, 1], [1, 3, 0]])

        self.step(1, 0, 0)
        self.step(1, 1, 0)
        self.step(0, 0, 0)
        self.step(0, 0, 0)
        self.step(1, 1, 0)

        self.rspan[-1].append(0)

        self.assert_tape(
            [[1, 4, 0]], 0, [[0, 1, 0], [1, 2, 0]])

        self.step(1, 0, 0)

        self.assert_tape(
            [[1, 4, 0], [0, 1]], (0, 0), [[1, 2, 0]])

        self.step(1, 1, 0)

        self.assert_tape(
            [[1, 4, 0], [0, 1], [1, 1]], 1, [[1, 1, 0]])

        self.step(0, 0, 0)

        self.assert_tape(
            [[1, 4, 0], [0, 1]], 1, [[0, 1], [1, 1, 0]])

        self.step(0, 0, 0)

        self.assert_tape(
            [[1, 4, 0]], 0, [[0, 2], [1, 1, 0]])

        self.step(1, 1, 0)

        self.assert_tape(
            [[1, 5, 0]], 0, [[0, 1], [1, 1, 0]])
