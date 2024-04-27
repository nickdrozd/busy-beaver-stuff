from __future__ import annotations

from unittest import TestCase
from typing import TYPE_CHECKING

from tm.tape import Tape, Block
from tm.rules import apply_rule

if TYPE_CHECKING:
    from tm.rules import Rule
    from tm.tape import Color, EnumTape


class TestTape(TestCase):
    tape: Tape

    def set_tape(
            self,
            scan: Color,
            lspan: list[tuple[int, int]],
            rspan: list[tuple[int, int]],
    ) -> None:
        self.tape = Tape(
            scan = scan,
            lspan = [Block(color, count) for color, count in lspan],
            rspan = [Block(color, count) for color, count in rspan],
        )

    def assert_tape(self, tape_str: str):
        self.assertEqual(tape_str, str(self.tape))

    def apply_rule(self, rule: Rule) -> None:
        _ = apply_rule(rule, self.tape)

    def test_marks(self):
        self.assertFalse(
            Tape().marks)

    def test_rule_1(self):
        self.set_tape(
            3,
            [(1, 12), (2, 3)],
            [(4, 15), (5, 2), (6, 2)])

        self.assert_tape(
            "2^3 1^12 [3] 4^15 5^2 6^2")

        self.apply_rule({
            (0, 1): 3,
            (1, 0): -2,
        })

        self.assert_tape(
            "2^24 1^12 [3] 4^1 5^2 6^2")

        self.apply_rule({
            (0, 0): -2,
            (1, 2): (2, 3),
        })

        self.assert_tape(
            "2^24 1^2 [3] 4^1 5^2 6^(-3 + (5 * (2 ** 5)))")

    def test_rule_2(self):
        self.set_tape(
            4,
            [(4, 2)],
            [(5, 60), (2, 1), (4, 1), (5, 7), (1, 1)])

        self.assert_tape(
            "4^2 [4] 5^60 2^1 4^1 5^7 1^1")

        self.apply_rule({
            (0, 0): 4,
            (1, 0): -2,
        })

        self.assert_tape(
            "4^118 [4] 5^2 2^1 4^1 5^7 1^1")

    def test_rule_3(self):
        self.set_tape(0, [(1, 152), (2, 655345), (3, 1)], [])

        self.assert_tape(
            "3^1 2^655345 1^152 [0]")

        self.apply_rule({
            (0, 1): -2,
            (0, 0): (2, 8),
        })

        exp = "(-8 + (5 * (2 ** 327677)))"

        self.assert_tape(
            f"3^1 2^1 1^{exp} [0]")


class TestEnum(TestCase):
    tape: EnumTape

    def set_tape(
            self,
            scan: Color,
            lspan: list[tuple[int, int]],
            rspan: list[tuple[int, int]],
    ) -> None:
        self.tape = Tape(
            scan = scan,
            lspan = [Block(color, count) for color, count in lspan],
            rspan = [Block(color, count) for color, count in rspan],
        ).to_enum()

    def step(self, shift: int, color: int, skip: int) -> None:
        self.tape.step(bool(shift), color, bool(skip))

    def assert_tape(
            self,
            tape: str,
            offsets: tuple[int, int],
            edges: tuple[int, int],
    ):
        self.assertEqual(
            tape,
            str(self.tape.tape))

        self.assertEqual(
            offsets,
            self.tape.offsets)

        self.assertEqual(
            tuple(map(bool, edges)),
            self.tape.edges)

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
            0, [(1, 11), (4, 1), (3, 11), (2, 1)], [])

        self.assert_tape(
            "2^1 3^11 4^1 1^11 [0]",
            (0, 0), (0, 0))

        self.step(0, 0, 0)  # B0

        self.assert_tape(
            "2^1 3^11 4^1 1^10 [1]",
            (1, 0), (0, 0))

        self.step(0, 2, 1)  # A1

        self.assert_tape(
            "2^1 3^11 [4] 2^11",
            (2, 0), (0, 0))

        self.step(0, 2, 1)  # A4

        self.assert_tape(
            "2^1 3^10 [3] 2^12",
            (3, 0), (0, 0))

        self.step(0, 2, 0)  # A3

        self.assert_tape(
            "2^1 3^9 [3] 2^13",
            (3, 0), (0, 0))

        self.step(1, 4, 0)  # B3

        self.assert_tape(
            "2^1 3^9 4^1 [2] 2^12",
            (3, 0), (0, 0))

        self.step(1, 1, 1)  # A2

        self.assert_tape(
            "2^1 3^9 4^1 1^13 [0]",
            (3, 0), (0, 1))

        self.step(1, 1, 0)  # A0

        self.assert_tape(
            "2^1 3^9 4^1 1^14 [0]",
            (3, 0), (0, 1))

    def test_offsets_2(self):
        # 1RB ... 2RC  2LC 2RD 0LC  1RA 2RB 0LB  1LB 0LD 2RC

        #  927 | |0 | 3^6 2^414422565 [0]
        #  928 | 2 | 3^6 2^414422564 [2] 5^1
        #  929 | 3 | 3^5 [3] 5^414422566
        #  930 | |5 | 3^5 2^1 [5] 5^414422565
        #  931 | |0 | 3^5 2^414422567 [0]

        self.set_tape(
            0, [(2, 414422565), (3, 6)], [])

        self.assert_tape(
            "3^6 2^414422565 [0]",
            (0, 0), (0, 0))

        self.step(0, 5, 0)

        self.assert_tape(
            "3^6 2^414422564 [2] 5^1",
            (1, 0), (0, 0))

        self.step(0, 5, 1)

        self.assert_tape(
            "3^5 [3] 5^414422566",
            (2, 0), (0, 0))

        self.step(1, 2, 0)

        self.assert_tape(
            "3^5 2^1 [5] 5^414422565",
            (2, 0), (0, 0))

        self.step(1, 2, 1)

        self.assert_tape(
            "3^5 2^414422567 [0]",
            (2, 0), (0, 1))

    def test_offsets_3(self):
        # 1^1 2^2 3^9 [3] 1^10

        self.set_tape(3, [(3, 9)], [(1, 10)])

        self.step(0, 1, 0)

        self.assert_tape(
            "3^8 [3] 1^11",
            (1, 1), (0, 0))

    def test_edges_1(self):
        self.set_tape(0, [], [])

        self.step(0, 1, 0)

        self.assert_tape(
            "[0] 1^1",
            (0, 0), (1, 0))

    def test_edges_2(self):
        self.set_tape(1, [(1, 3)], [])

        self.step(0, 2, 1)

        self.assert_tape(
            "[0] 2^4",
            (1, 0), (1, 0))
