from unittest import TestCase

from tm.tape import Color, Tape, TagTape, Signature, Span

def stringify_sig(sig: Signature) -> str:
    scan, lspan, rspan = sig

    l_sig = '|'.join(
        str(c if isinstance(c, int) else c[0])
        for c in lspan)
    r_sig = '|'.join(reversed(
        [str(c if isinstance(c, int) else c[0])
         for c in rspan]))

    return f'{l_sig}[{scan}]{r_sig}'


class TestTape(TestCase):
    tape: Tape

    def assert_signature(
            self,
            expected: str,
            tape: Tape | None = None,
    ) -> None:
        self.assertEqual(
            expected,
            stringify_sig(
                (tape or self.tape).signature))

    def test_marks(self):
        self.assertFalse(
            Tape([], 0, []).marks)

    def test_copy(self):
        self.tape = Tape(
            [[1, 1], [0, 1], [1, 1]],
            2,
            list(reversed([[2, 1], [1, 2]]))
        )

        self.assert_signature(
            '1|0|1[2]2|1')

        copy_1 = self.tape.copy()
        copy_2 = self.tape.copy()

        _ = copy_1.step(False, 2, False)
        _ = copy_2.step( True, 1, False)

        self.assert_signature(
            '1|0|1[2]2|1')

        self.assert_signature(
            '1|0[1]2|1',
            tape = copy_1)

        self.assert_signature(
            '1|0|1[2]1',
            tape = copy_2)


class TestBlocks(TestCase):
    tape: TagTape

    init_tags: int

    def count_tags(self) -> int:
        return (
            (1 if self.scan_info else 0)
            + sum(1 for block in self.lspan if len(block) > 2)
            + sum(1 for block in self.rspan if len(block) > 2)
        )

    def set_tape(
            self,
            lspan: Span,
            scan: Color | tuple[Color, list[int]],
            rspan: Span,
    ):
        if isinstance(scan, tuple):
            scan, scan_info = scan
        else:
            scan_info = []

        self.tape = TagTape(lspan, scan, list(reversed(rspan)))

        self.tape.scan_info = scan_info

        self.init_tags = self.count_tags()

    def assert_tape(
            self,
            lspan: Span,
            scan: Color | tuple[Color, list[int]],
            rspan: Span,
    ):
        self.assertEqual(self.lspan, lspan)
        self.assertEqual(self.rspan, list(reversed(rspan)))

        self.assertEqual(
            (self.scan, self.scan_info),
            ((scan, []) if isinstance(scan, Color) else scan))

        self.assertGreaterEqual(
            self.init_tags,
            self.count_tags())

    @property
    def scan(self) -> Color:
        return self.tape.scan

    @property
    def scan_info(self) -> list[int] | None:
        return self.tape.scan_info

    @property
    def lspan(self) -> Span:
        return self.tape.lspan

    @property
    def rspan(self) -> Span:
        return self.tape.rspan

    def step(self, shift: int, color: int, skip: int) -> None:
        self.tape.step(bool(shift), color, bool(skip))

    def test_trace_1(self):
        # 1RB 1LC  1RD 1RB  0RD 0RC  1LD 1LA : BBB(4, 2)
        #    49 |   144 | D1 | 1^15 [1] 1^6
        #    54 |   167 | D1 | 1^12 [1] 1^11

        self.set_tape([[1, 15, 1]], 1, [[1, 6, 2]])

        self.step(0, 1, 0)
        self.step(0, 1, 0)
        self.step(1, 0, 1)
        self.step(1, 0, 0)
        self.step(0, 1, 1)

        self.assert_tape([[1, 12, 1]], 1, [[1, 11, 2]])

    def test_trace_2(self):
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

        self.assert_tape([[1, 4]], (0, [0]), [])

        self.step(1, 1, 0)

        self.assert_tape([[1, 5, 0]], 0, [])

    def test_trace_3(self):
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

        self.assert_tape([[1, 1], [2, 60]], (2, [0]), [[1, 1]])

        self.step(0, 1, 1)

        self.assert_tape([], 1, [[1, 62, 0]])

        self.step(1, 2, 1)

        self.assert_tape([[2, 63, 0]], 0, [])

    def test_trace_4(self):
        # Lynn exception
        # 1RB 1RA  1LC 0LD  0RA 1LB  1R_ 0LE  1RC 1RB
        #    50 | 54 | C0 | 1^2 [0] 0^1 1^4
        #    51 | 55 | A0 | 1^2 0^1 [0] 1^4
        #    52 | 56 | B1 | 1^2 0^1 1^1 [1] 1^3
        #    53 | 57 | D1 | 1^2 0^1 [1] 0^1 1^3
        #    54 | 58 | E0 | 1^2 [0] 0^2 1^3
        #    55 | 59 | C0 | 1^3 [0] 0^1 1^3

        self.set_tape([[1, 2, 1]], 0, [[0, 1, 2], [1, 4, 3]])

        self.step(1, 0, 0)

        self.assert_tape(
            [[1, 2, 1], [0, 1]], (0, [2]), [[1, 4, 3]])

        self.step(1, 1, 0)

        self.assert_tape(
            [[1, 2, 1], [0, 1], [1, 1, 2]], 1, [[1, 3, 3]])

        self.step(0, 0, 0)

        self.assert_tape(
            [[1, 2, 1], [0, 1]], (1, [2]), [[0, 1], [1, 3, 3]])

        self.step(0, 0, 0)

        self.assert_tape(
            [[1, 2, 1]], 0, [[0, 2, 2], [1, 3, 3]])

        self.step(1, 1, 0)

        self.assert_tape(
            [[1, 3, 1]], 0, [[0, 1, 2], [1, 3, 3]])

    def test_trace_5(self):
        # 1RB 1LC  1LC 0RD  1LA 0LB  1LD 0RA

        # A0 | 1^2 [0] 1^17 0^1 1^3 0^1 1^1
        # B1 | 1^3 [1] 1^16 0^1 1^3 0^1 1^1
        # D1 | 1^3 0^1 [1] 1^15 0^1 1^3 0^1 1^1
        # A1 | 1^3 0^2 [1] 1^14 0^1 1^3 0^1 1^1
        # C0 | 1^3 0^1 [0] 1^15 0^1 1^3 0^1 1^1
        # A0 | 1^3 [0] 1^16 0^1 1^3 0^1 1^1

        self.set_tape([[1, 2, 1]], 0, [[1, 17, 2]])

        self.step(1, 1, 0)
        self.step(1, 0, 0)
        self.step(1, 0, 0)
        self.step(0, 1, 0)
        self.step(0, 1, 0)

        self.assert_tape([[1, 3, 1]], 0, [[1, 16, 2]])

    def test_trace_6(self):
        # 1RB 0LB  1LC 0RC  1RA 1LA

        #   46 | B1 | 0^1 1^3 0^2 1^3 [1]
        #   47 | C0 | 0^1 1^3 0^2 1^3 0^1 [0]
        #   48 | A0 | 0^1 1^3 0^2 1^3 0^1 1^1 [0]
        #   49 | B0 | 0^1 1^3 0^2 1^3 0^1 1^2 [0]
        #   50 | C1 | 0^1 1^3 0^2 1^3 0^1 1^1 [1] 1^1
        #   51 | A1 | 0^1 1^3 0^2 1^3 0^1 [1] 1^2
        #   52 | B0 | 0^1 1^3 0^2 1^3 [0] 0^1 1^2
        #   53 | C1 | 0^1 1^3 0^2 1^2 [1] 1^1 0^1 1^2
        #   54 | A1 | 0^1 1^3 0^2 1^1 [1] 1^2 0^1 1^2
        #   55 | B1 | 0^1 1^3 0^2 [1] 0^1 1^2 0^1 1^2
        #   56 | C0 | 0^1 1^3 0^3 [0] 1^2 0^1 1^2
        #   57 | A1 | 0^1 1^3 0^3 1^1 [1] 1^1 0^1 1^2
        #   58 | B1 | 0^1 1^3 0^3 [1] 0^1 1^1 0^1 1^2
        #   59 | C0 | 0^1 1^3 0^4 [0] 1^1 0^1 1^2
        #   60 | A1 | 0^1 1^3 0^4 1^1 [1] 0^1 1^2
        #   61 | B1 | 0^1 1^3 0^4 [1] 0^2 1^2
        #   62 | C0 | 0^1 1^3 0^5 [0] 0^1 1^2
        #   63 | A0 | 0^1 1^3 0^5 1^1 [0] 1^2
        #   64 | B1 | 0^1 1^3 0^5 1^2 [1] 1^1
        #   65 | C1 | 0^1 1^3 0^5 1^2 0^1 [1]
        #   66 | A0 | 0^1 1^3 0^5 1^2 [0] 1^1
        #   67 | B1 | 0^1 1^3 0^5 1^3 [1]

        self.set_tape(
            [[1, 3, 0], [0, 2, 1], [1, 3, 2]],
            1,
            [])

        self.step(1, 0, 0)
        self.step(1, 1, 0)
        self.step(1, 1, 0)
        self.step(0, 1, 0)
        self.step(0, 1, 0)
        self.step(0, 0, 0)

        self.assert_tape(
            [[1, 3, 0], [0, 2, 1], [1, 3, 2]],
            0,
            [[0, 1], [1, 2]])

        self.step(0, 1, 0)
        self.step(0, 1, 0)

        self.assert_tape(
            [[1, 3, 0], [0, 2, 1], [1, 1, 2]],
            1,
            [[1, 2], [0, 1], [1, 2]])

        self.step(0, 0, 0)

        self.assert_tape(
            [[1, 3, 0], [0, 2, 1]],
            (1, [2]),
            [[0, 1], [1, 2], [0, 1], [1, 2]])

        self.step(1, 0, 0)

        self.assert_tape(
            [[1, 3, 0], [0, 3, 1, 2]],
            0,
            [[1, 2], [0, 1], [1, 2]])

        self.step(1, 1, 0)

        self.assert_tape(
            [[1, 3, 0], [0, 3, 1], [1, 1, 2]],
            1,
            [[1, 1], [0, 1], [1, 2]])

        self.step(0, 0, 0)

        self.assert_tape(
            [[1, 3, 0], [0, 3, 1]],
            (1, [2]),
            [[0, 1], [1, 1], [0, 1], [1, 2]])

        self.step(1, 0, 0)

        self.assert_tape(
            [[1, 3, 0], [0, 4, 1, 2]],
            0,
            [[1, 1], [0, 1], [1, 2]])

        self.step(1, 1, 0)

        self.assert_tape(
            [[1, 3, 0], [0, 4, 1], [1, 1, 2]],
            1,
            [[0, 1], [1, 2]])

        self.step(0, 0, 0)

        self.assert_tape(
            [[1, 3, 0], [0, 4, 1]],
            (1, [2]),
            [[0, 2], [1, 2]])

        self.step(1, 0, 0)

        self.assert_tape(
            [[1, 3, 0], [0, 5, 1, 2]],
            0,
            [[0, 1], [1, 2]])

        self.step(1, 1, 0)

        self.assert_tape(
            [[1, 3, 0], [0, 5, 1], [1, 1, 2]],
            0,
            [[1, 2]])

        self.step(1, 1, 0)

        self.assert_tape(
            [[1, 3, 0], [0, 5, 1], [1, 2, 2]],
            1,
            [[1, 1]])

        self.step(1, 0, 0)

        self.assert_tape(
            [[1, 3, 0], [0, 5, 1], [1, 2, 2], [0, 1]],
            1,
            [])

        self.step(0, 1, 0)

        self.assert_tape(
            [[1, 3, 0], [0, 5, 1], [1, 2, 2]],
            0,
            [[1, 1]])

        self.step(1, 1, 0)

        self.assert_tape(
            [[1, 3, 0], [0, 5, 1], [1, 3, 2]], 1, [])

    def test_trace_7(self):
        # 1RB 1LA 0RB  0LA 2RB ...

        self.set_tape(
            [[1, 1]],
            (2, [0]),
            [[1, 2], [0, 1]])

        self.step(1, 0, 0)

        self.assert_tape(
            [[1, 1], [0, 1, 0]], 1, [[1, 1], [0, 1]])
