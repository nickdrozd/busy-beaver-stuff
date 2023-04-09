from unittest import TestCase

from tm.rules import Rule
from tm.instrs import Color
from tm.tape import Tape, TagTape, EnumTape, Signature, Block

BlockSpan = list[list[int]]

def stringify_sig(sig: Signature) -> str:
    scan, lspan, rspan = sig

    l_sig = '|'.join(reversed(
        [str(c if isinstance(c, int) else c[0])
         for c in lspan]))
    r_sig = '|'.join(
        str(c if isinstance(c, int) else c[0])
        for c in rspan)

    return f'{l_sig}[{scan}]{r_sig}'


class TestTape(TestCase):
    tape: Tape

    def set_tape(
            self,
            lspan: BlockSpan,
            scan: Color,
            rspan: BlockSpan,
    ) -> None:
        self.tape = Tape(
            [Block(color, count) for color, count in lspan],
            scan,
            [Block(color, count) for color, count in rspan])

    def assert_tape(self, tape: str):
        self.assertEqual(tape, str(self.tape))

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
            Tape.init().marks)

    def test_copy(self):
        self.set_tape(
            [[1, 1], [0, 1], [1, 1]],
            2,
            [[2, 1], [1, 2]]
        )

        self.assert_tape(
            '1^1 0^1 1^1 [2] 2^1 1^2')

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

    def test_rule_1(self):
        self.set_tape(
            [[1, 12], [2, 3]],
            3,
            [[4, 15], [5, 2], [6, 2]])

        self.assert_tape(
            "2^3 1^12 [3] 4^15 5^2 6^2")

        self.tape.apply_rule({
            (0, 1): 3,
            (1, 0): -2,
        })

        self.assert_tape(
            "2^24 1^12 [3] 4^1 5^2 6^2")

        self.tape.apply_rule({
            (0, 0): -2,
            (1, 2): (2, 3),
        })

        self.assert_tape(
            "2^24 1^2 [3] 4^1 5^2 6^157")

    def test_rule_2(self):
        self.set_tape(
            [[4, 2]],
            4,
            [[5, 60], [2, 1], [4, 1], [5, 7], [1, 1]])

        self.assert_tape(
            "4^2 [4] 5^60 2^1 4^1 5^7 1^1")

        self.tape.apply_rule({
            (0, 0): 4,
            (1, 0): -2,
        })

        self.assert_tape(
            "4^118 [4] 5^2 2^1 4^1 5^7 1^1")


class TestTags(TestCase):
    tape: TagTape

    init_tags: int

    def count_tags(self) -> int:
        return (
            (1 if self.scan_info else 0)
            + sum(1 for block in self.tape.lspan if block.other)
            + sum(1 for block in self.tape.rspan if block.other)
        )

    def set_tape(
            self,
            lspan: BlockSpan,
            scan: Color | tuple[Color, list[int]],
            rspan: BlockSpan,
    ):
        if isinstance(scan, tuple):
            scan, scan_info = scan
        else:
            scan_info = []

        self.tape = TagTape(
            [Block(color, count, other)
                 for color, count, *other in reversed(lspan)],
            scan,
            [Block(color, count, other)
                 for color, count, *other in rspan],
        )

        self.tape.scan_info = scan_info

        self.init_tags = self.count_tags()

    def assert_tape(
            self,
            lspan: BlockSpan,
            scan: Color | tuple[Color, list[int]],
            rspan: BlockSpan,
    ):
        self.assertEqual(
            self.tape.lspan,
            [Block(color, count, other)
                 for color, count, *other in reversed(lspan)])

        self.assertEqual(
            self.tape.rspan,
            [Block(color, count, other)
                 for color, count, *other in rspan])

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

    def step(self, shift: int, color: int, skip: int) -> None:
        self.tape.step(bool(shift), color, bool(skip))

    def apply_rule(self, rule: Rule) -> None:
        self.tape.apply_rule(rule)

    def test_trace_1(self):
        # 1RB 1LC  1RD 1RB  0RD 0RC  1LD 1LA : BBB(4, 2)

        self.set_tape([[1, 15, 1]], 1, [[1, 6, 2]])

        self.step(0, 1, 0)
        self.step(0, 1, 0)
        self.step(1, 0, 1)
        self.step(1, 0, 0)
        self.step(0, 1, 1)

        self.assert_tape([[1, 12, 1]], 1, [[1, 11, 2]])

    def test_trace_2(self):
        # 1RB 1LA  0LA 0RB: counter

        self.set_tape([[1, 4, 0]], 0, [])

        self.step(0, 0, 0)
        self.step(0, 1, 1)
        self.step(1, 1, 0)
        self.step(1, 0, 1)

        self.apply_rule({(0, 0): -1, (0, 1): 1})

        self.step(0, 0, 0)
        self.step(1, 1, 0)

        self.assert_tape([[1, 5, 0]], 0, [])

    def test_trace_3(self):
        # 1RB 0RB 2LA  1LA 2RB 1LB: counter

        self.set_tape([[2, 31, 0]], 0, [])

        self.step(0, 1, 0)
        self.step(0, 2, 1)
        self.step(1, 1, 0)
        self.step(0, 1, 0)
        self.step(1, 2, 1)

        self.apply_rule({(0, 0): 4, (1, 0): -2})

        self.step(0, 1, 1)
        self.step(0, 1, 0)
        self.step(1, 1, 0)
        self.step(1, 2, 1)
        self.step(0, 1, 1)
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

        #   46 | B1 | 1^3 0^2 1^3 [1]
        #   47 | C0 | 1^3 0^2 1^3 0^1 [0]
        #   48 | A0 | 1^3 0^2 1^3 0^1 1^1 [0]
        #   49 | B0 | 1^3 0^2 1^3 0^1 1^2 [0]
        #   50 | C1 | 1^3 0^2 1^3 0^1 1^1 [1] 1^1
        #   51 | A1 | 1^3 0^2 1^3 0^1 [1] 1^2
        #   52 | B0 | 1^3 0^2 1^3 [0] 0^1 1^2
        #   53 | C1 | 1^3 0^2 1^2 [1] 1^1 0^1 1^2
        #   54 | A1 | 1^3 0^2 1^1 [1] 1^2 0^1 1^2
        #   55 | B1 | 1^3 0^2 [1] 0^1 1^2 0^1 1^2
        #   56 | C0 | 1^3 0^3 [0] 1^2 0^1 1^2
        #   57 | A1 | 1^3 0^3 1^1 [1] 1^1 0^1 1^2
        #   58 | B1 | 1^3 0^3 [1] 0^1 1^1 0^1 1^2
        #   59 | C0 | 1^3 0^4 [0] 1^1 0^1 1^2
        #   60 | A1 | 1^3 0^4 1^1 [1] 0^1 1^2
        #   61 | B1 | 1^3 0^4 [1] 0^2 1^2
        #   62 | C0 | 1^3 0^5 [0] 0^1 1^2
        #   63 | A0 | 1^3 0^5 1^1 [0] 1^2
        #   64 | B1 | 1^3 0^5 1^2 [1] 1^1
        #   65 | C1 | 1^3 0^5 1^2 0^1 [1]
        #   66 | A0 | 1^3 0^5 1^2 [0] 1^1
        #   67 | B1 | 1^3 0^5 1^3 [1]

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
            1,
            [[0, 2, 2], [1, 2]])

        self.step(1, 0, 0)

        self.assert_tape(
            [[1, 3, 0], [0, 5, 1]],
            0,
            [[0, 1, 2], [1, 2]])

        self.step(1, 1, 0)

        self.assert_tape(
            [[1, 3, 0], [0, 5, 1], [1, 1]],
            (0, [2]),
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

    def test_trace_8(self):
        # 1RB 1RA  1LC 0LA  0RA 0LC

        self.set_tape(
            [[1, 5]], 0, [[1, 1, 0]])

        self.step(1, 1, 0)

        self.assert_tape(
            [[1, 6, 0]], 1, [])

        self.step(0, 0, 0)

        self.assert_tape(
            [[1, 5, 0]], 1, [])

    def test_trace_9(self):
        self.set_tape(
            [[1, 1], [0, 1, 0]], 1, [[1, 6]])

        self.step(0, 1, 0)

        self.assert_tape(
            [[1, 1]], 0, [[1, 7, 0]])

    def test_trace_a(self):
        self.set_tape(
            [], (0, [0]), [[0, 1], [1, 5]])

        self.step(1, 0, 1)

        self.assert_tape(
            [[0, 1, 0]], 1, [[1, 4]])

    def test_trace_b(self):
        self.set_tape(
            [[1, 3], [0, 1], [1, 1]], (1, [0]), [])

        self.step(0, 0, 0)

        self.assert_tape(
            [[1, 3], [0, 1]], 1, [[0, 1, 0]])

    def test_trace_c(self):
        self.set_tape(
            [[1, 1], [0, 1], [1, 1, 1]], 1, [[2, 2, 0]])

        self.step(0, 2, 1)

        self.assert_tape(
            [[1, 1]], 0, [[2, 4, 0, 1]])


class TestEnum(TestCase):
    tape: EnumTape

    def set_tape(
            self,
            lspan: BlockSpan,
            scan: Color,
            rspan: BlockSpan,
    ) -> None:
        self.tape = EnumTape(
            [Block(color, count) for color, count in lspan],
            scan,
            [Block(color, count) for color, count in rspan])

    def step(self, shift: int, color: int, skip: int) -> None:
        self.tape.step(bool(shift), color, bool(skip))

    def assert_offsets(self, offsets: list[int]):
        self.assertEqual(
            offsets,
            self.tape.offsets)

    def assert_edges(self, edges: tuple[bool, bool]):
        self.assertEqual(
            edges,
            self.tape.edges)

    def assert_tape(
            self,
            lspan: BlockSpan,
            scan: Color,
            rspan: BlockSpan,
    ):
        self.assertEqual(
            (lspan, scan, rspan),
            (
                [[block.color, block.count]
                     for block in self.tape.lspan],
                self.tape.scan,
                [[block.color, block.count]
                     for block in self.tape.rspan],
            ))

    def test_offsets_1(self):
        # 1RB 2LA 1RA 2LB 2LA  0LA 2RB 3RB 4RA 1R_

        #  158 | B0 | 2^1 3^11 4^1 1^11 [0]
        #  159 | A1 | 2^1 3^11 4^1 1^10 [1]
        #  160 | A4 | 2^1 3^11 [4] 2^11
        #  161 | A3 | 2^1 3^10 [3] 2^12
        #  162 | B3 | 2^1 3^9 [3] 2^13
        #  163 | A2 | 2^1 3^9 4^1 [2] 2^12
        #  164 | A0 | 2^1 3^9 4^1 1^13 [0]
        #  165 | B0 | 2^1 3^9 4^1 1^14 [0]

        self.set_tape(
            [[1, 11], [4, 1], [3, 11], [2, 1]], 0, [])

        self.assert_offsets([0, 0])

        self.step(0, 0, 0)  # B0

        self.assert_tape(
            [[1, 10], [4, 1], [3, 11], [2, 1]], 1, [])

        self.assert_offsets([1, 0])

        self.step(0, 2, 1)  # A1

        self.assert_tape(
            [[3, 11], [2, 1]], 4, [[2, 11]])

        self.assert_offsets([2, 0])

        self.step(0, 2, 1)  # A4

        self.assert_tape(
            [[3, 10], [2, 1]], 3, [[2, 12]])

        self.assert_offsets([3, 0])

        self.step(0, 2, 0)  # A3

        self.assert_offsets([3, 0])

        self.step(1, 4, 0)  # B3
        self.step(1, 1, 1)  # A2
        self.step(1, 1, 0)  # A0

        self.assert_tape(
            [[1, 14], [4, 1], [3, 9], [2, 1]], 0, [])

        self.assert_offsets([3, 0])

    def test_offsets_2(self):
        # 1RB 1R_ 2RC  2LC 2RD 0LC  1RA 2RB 0LB  1LB 0LD 2RC

        #  927 | |0 | 3^6 2^414422565 [0]
        #  928 | 2 | 3^6 2^414422564 [2] 5^1
        #  929 | 3 | 3^5 [3] 5^414422566
        #  930 | |5 | 3^5 2^1 [5] 5^414422565
        #  931 | |0 | 3^5 2^414422567 [0]

        self.set_tape(
            [[2, 414422565], [3, 6]], 0, [])

        self.assert_offsets([0, 0])

        self.step(0, 5, 0)
        self.assert_tape(
            [[2, 414422564], [3, 6]], 2, [[5, 1]])
        self.assert_offsets([1, 0])

        self.step(0, 5, 1)
        self.assert_tape(
            [[3, 5]], 3, [[5, 414422566]])
        self.assert_offsets([2, 0])

        self.step(1, 2, 0)
        self.step(1, 2, 1)

        self.assert_tape(
            [[2, 414422567], [3, 5]], 0, [])

    def test_offsets_3(self):
        # 1^1 2^2 3^9 [3] 1^10

        self.set_tape([[3, 9]], 3, [[1, 10]])
        self.step(0, 1, 0)
        self.assert_tape([[3, 8]], 3, [[1, 11]])
        self.assert_offsets([1, 1])

    def test_edges_1(self):
        self.set_tape([], 0, [])
        self.step(0, 1, 0)
        self.assert_tape([], 0, [[1, 1]])
        self.assert_edges((True, False))

    def test_edges_2(self):
        self.set_tape([[1, 3]], 1, [])
        self.step(0, 2, 1)
        self.assert_tape([], 0, [[2, 4]])
        self.assert_edges((True, False))
