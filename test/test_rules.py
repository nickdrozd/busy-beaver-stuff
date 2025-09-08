from typing import TYPE_CHECKING
from unittest import TestCase

from tm.rules import Exp as ExpT
from tm.rules import apply_mult, apply_ops

if TYPE_CHECKING:
    from tm.rules import OpSeq

Exp = ExpT.make


def apply_loop(count: int, times: int, mul: int, add: int) -> int:
    val = count

    for _ in range(times):
        val = add + (mul * val)

    return val


VALUES = {
    ( 1,  4, 2, 3): 61,
    (45, 99, 2, 7): 32958915605933964438914283339769,
}


class TestApply(TestCase):
    def test_apply_mult(self):
        for (count, times, mul, add), val in VALUES.items():
            self.assertEqual(
                int(apply_mult(count, times, mul, add)),
                val)

            self.assertEqual(
                apply_loop(count, times, mul, add),
                val)

    def test_apply_ops_1(self):
        count1 = (-4 + (7 * Exp(2, 3))) // 3

        count2 = (-4 + (7 * (2 ** ((8 + (7 * Exp(2, 3))) // 3)))) // 3

        count3 = (-4 + (7 * (2 ** ((8 + (7 * (2 ** ((8 + (7 * Exp(2, 3))) // 3)))) // 3)))) // 3

        ops: OpSeq = (
            ('*', 3),
            ('+', 4),
            ('+', 8),
            ('//', 3),
            ('**', 2),
            ('*', 7),
            ('+', -4),
            ('//', 3),
        )

        self.assertEqual(
            apply_ops(count1, 1, ops),
            count2)

        self.assertEqual(
            apply_ops(count2, 1, ops),
            count3)

    def test_apply_ops_2(self):
        count = -6 + Exp(2, 13)

        times = 32764

        ops: OpSeq = (
            ('+', 6),
            ('~', 2),
            ('+', 3),
            ('**', 2),
            ('**', 2),
            ('+', -3),
            ('**', 2),
            ('+', -6),
        )

        result = apply_ops(count, times, ops)

        assert not isinstance(result, int)

        self.assertEqual(
            str(result.estimate()),
            "(10 ↑↑ 65530)")
