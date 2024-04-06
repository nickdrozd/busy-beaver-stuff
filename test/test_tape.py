from __future__ import annotations

from unittest import TestCase
from typing import TYPE_CHECKING

from tm.tape import Tape, Block

if TYPE_CHECKING:
    from tm.tape import Color, TagTape, EnumTape


class TestTape(TestCase):
    tape: Tape

    def set_tape(
            self,
            lspan: list[tuple[int, int]],
            scan: Color,
            rspan: list[tuple[int, int]],
    ) -> None:
        self.tape = Tape(
            lspan = [Block(color, count) for color, count in lspan],
            scan = scan,
            rspan = [Block(color, count) for color, count in rspan],
        )

    def assert_tape(self, tape_str: str):
        self.assertEqual(tape_str, str(self.tape))

    def test_marks(self):
        self.assertFalse(
            Tape().marks)

    def test_rule_1(self):
        self.set_tape(
            [(1, 12), (2, 3)],
            3,
            [(4, 15), (5, 2), (6, 2)])

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
            "2^24 1^2 [3] 4^1 5^2 6^(-3 + (5 * (2 ** 5)))")

    def test_rule_2(self):
        self.set_tape(
            [(4, 2)],
            4,
            [(5, 60), (2, 1), (4, 1), (5, 7), (1, 1)])

        self.assert_tape(
            "4^2 [4] 5^60 2^1 4^1 5^7 1^1")

        self.tape.apply_rule({
            (0, 0): 4,
            (1, 0): -2,
        })

        self.assert_tape(
            "4^118 [4] 5^2 2^1 4^1 5^7 1^1")

    def test_rule_3(self):
        self.set_tape([(1, 152), (2, 655345), (3, 1)], 0, [])

        self.assert_tape(
            "3^1 2^655345 1^152 [0]")

        self.tape.apply_rule({
            (0, 1): -2,
            (0, 0): (2, 8),
        })

        exp = "(-8 + (5 * (2 ** 327677)))"

        self.assert_tape(
            f"3^1 2^1 1^{exp} [0]")


class TestTags(TestCase):
    tape: TagTape

    init_tags: int

    def count_tags(self) -> int:
        return (
            (1 if self.tape.scan_info else 0)
            + sum(1 for block in self.tape.lspan if block.tags)
            + sum(1 for block in self.tape.rspan if block.tags)
        )

    def set_tape(
            self,
            lspan: list[list[int]],
            scan: Color | tuple[Color, list[int]],
            rspan: list[list[int]],
    ):
        if isinstance(scan, tuple):
            scan, scan_info = scan
        else:
            scan_info = []

        # pylint: disable = unnecessary-comprehension
        self.tape = Tape(
            [Block(color, count) for color,count,*_ in reversed(lspan)],
            scan,
            [Block(color, count) for color, count, *_ in rspan],
        ).to_tag()

        for blk, (_, _, *tags) in zip(self.tape.lspan, reversed(lspan)):
            blk.tags = tags

        for blk, (_, _, *tags) in zip(self.tape.rspan, rspan):
            blk.tags = tags

        self.tape.scan_info = scan_info

        self.init_tags = self.count_tags()

    def assert_tape(
            self,
            lspan: list[list[int]],
            scan: Color | tuple[Color, list[int]],
            rspan: list[list[int]],
    ):
        self.assertEqual(
            (self.tape.scan, self.tape.scan_info),
            (scan if isinstance(scan, tuple) else (scan, [])))

        self.assertEqual(
            lspan,
            [[block.color, block.count, *block.tags]
                 for block in reversed(self.tape.lspan)])

        self.assertEqual(
            rspan,
            [[block.color, block.count, *block.tags]
                 for block in self.tape.rspan])

        self.assertGreaterEqual(
            self.init_tags,
            self.count_tags())

    def step(self, shift: int, color: int, skip: int) -> None:
        self.tape.step(bool(shift), color, bool(skip))

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

        self.tape.apply_rule({(0, 0): -1, (0, 1): 1})

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

        self.tape.apply_rule({(0, 0): 4, (1, 0): -2})

        self.step(0, 1, 1)
        self.step(0, 1, 0)
        self.step(1, 1, 0)
        self.step(1, 2, 1)
        self.step(0, 1, 1)
        self.step(1, 2, 1)

        self.assert_tape([[2, 63, 0]], 0, [])

    def test_trace_4(self):
        # Lynn exception
        # 1RB 1RA  1LC 0LD  0RA 1LB  ... 0LE  1RC 1RB
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

    def test_trace_d(self):
        self.set_tape(
            [[3, 1]],
            0,
            [[0, 5, 1], [3, 1, 3], [0, 1, 5], [1, 1]])

        self.step(1, 3, 1)

        self.assert_tape(
            [[3, 7, 1]],
            (3, [3]),
            [[0, 1, 5], [1, 1]])

    def test_trace_e(self):
        self.set_tape(
            [[3, 7, 3], [2, 1]],
            (1, [5]),
            [])

        self.step(0, 0, 0)

        self.assert_tape(
            [[3, 7, 3]],
            2,
            [[0, 1, 5]])

        self.step(0, 1, 0)

        self.assert_tape(
            [[3, 6, 3]],
            3,
            [[1, 1, 5]])

    def test_trace_f(self):
        self.set_tape(
            [[3, 6, 0], [1, 1], [3, 1, 1], [0, 8]],
            0,
            [[2, 1]])

        self.step(0, 3, 1)

        self.assert_tape(
            [[3, 6, 0], [1, 1]],
            3,
            [[3, 9, 1], [2, 1]])

        ########################################

        self.set_tape(
            [[3, 6, 0], [1, 1, 1], [0, 8]],
            0,
            [[2, 1]])

        self.step(0, 3, 1)

        self.assert_tape(
            [[3, 6, 0]],
            1,
            [[3, 9, 1], [2, 1]])

    def test_trace_g(self):
        self.set_tape(
            [[1, 2, 6]],
            1,
            [[1, 1], [2, 1, 0], [0, 1], [2, 1]])

        self.step(1, 3, 1)

        self.assert_tape(
            [[1, 2, 6], [3, 2]],
            (2, [0]),
            [[0, 1], [2, 1]])


class TestEnum(TestCase):
    tape: EnumTape

    def set_tape(
            self,
            lspan: list[tuple[int, int]],
            scan: Color,
            rspan: list[tuple[int, int]],
    ) -> None:
        self.tape = Tape(
            lspan = [Block(color, count) for color, count in lspan],
            scan = scan,
            rspan = [Block(color, count) for color, count in rspan],
        ).to_enum()

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
            lspan: list[tuple[int, int]],
            scan: Color,
            rspan: list[tuple[int, int]],
    ):
        self.assertEqual(
            (lspan, scan, rspan),
            (
                [(blk.color, blk.count) for blk in self.tape.lspan],
                self.tape.scan,
                [(blk.color, blk.count) for blk in self.tape.rspan],
            ))

    def test_offsets_1(self):
        # 1RB 2LA 1RA 2LB 2LA  0LA 2RB 3RB 4RA ...

        #  158 | B0 | 2^1 3^11 4^1 1^11 [0]
        #  159 | A1 | 2^1 3^11 4^1 1^10 [1]
        #  160 | A4 | 2^1 3^11 [4] 2^11
        #  161 | A3 | 2^1 3^10 [3] 2^12
        #  162 | B3 | 2^1 3^9 [3] 2^13
        #  163 | A2 | 2^1 3^9 4^1 [2] 2^12
        #  164 | A0 | 2^1 3^9 4^1 1^13 [0]
        #  165 | B0 | 2^1 3^9 4^1 1^14 [0]

        self.set_tape(
            [(1, 11), (4, 1), (3, 11), (2, 1)], 0, [])

        self.assert_offsets([0, 0])

        self.step(0, 0, 0)  # B0

        self.assert_tape(
            [(1, 10), (4, 1), (3, 11), (2, 1)], 1, [])

        self.assert_offsets([1, 0])

        self.step(0, 2, 1)  # A1

        self.assert_tape(
            [(3, 11), (2, 1)], 4, [(2, 11)])

        self.assert_offsets([2, 0])

        self.step(0, 2, 1)  # A4

        self.assert_tape(
            [(3, 10), (2, 1)], 3, [(2, 12)])

        self.assert_offsets([3, 0])

        self.step(0, 2, 0)  # A3

        self.assert_offsets([3, 0])

        self.step(1, 4, 0)  # B3
        self.step(1, 1, 1)  # A2
        self.step(1, 1, 0)  # A0

        self.assert_tape(
            [(1, 14), (4, 1), (3, 9), (2, 1)], 0, [])

        self.assert_offsets([3, 0])

    def test_offsets_2(self):
        # 1RB ... 2RC  2LC 2RD 0LC  1RA 2RB 0LB  1LB 0LD 2RC

        #  927 | |0 | 3^6 2^414422565 [0]
        #  928 | 2 | 3^6 2^414422564 [2] 5^1
        #  929 | 3 | 3^5 [3] 5^414422566
        #  930 | |5 | 3^5 2^1 [5] 5^414422565
        #  931 | |0 | 3^5 2^414422567 [0]

        self.set_tape(
            [(2, 414422565), (3, 6)], 0, [])

        self.assert_offsets([0, 0])

        self.step(0, 5, 0)
        self.assert_tape(
            [(2, 414422564), (3, 6)], 2, [(5, 1)])
        self.assert_offsets([1, 0])

        self.step(0, 5, 1)
        self.assert_tape(
            [(3, 5)], 3, [(5, 414422566)])
        self.assert_offsets([2, 0])

        self.step(1, 2, 0)
        self.step(1, 2, 1)

        self.assert_tape(
            [(2, 414422567), (3, 5)], 0, [])

    def test_offsets_3(self):
        # 1^1 2^2 3^9 [3] 1^10

        self.set_tape([(3, 9)], 3, [(1, 10)])
        self.step(0, 1, 0)
        self.assert_tape([(3, 8)], 3, [(1, 11)])
        self.assert_offsets([1, 1])

    def test_edges_1(self):
        self.set_tape([], 0, [])
        self.step(0, 1, 0)
        self.assert_tape([], 0, [(1, 1)])
        self.assert_edges((True, False))

    def test_edges_2(self):
        self.set_tape([(1, 3)], 1, [])
        self.step(0, 2, 1)
        self.assert_tape([], 0, [(2, 4)])
        self.assert_edges((True, False))
