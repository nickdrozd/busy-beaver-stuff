# pylint: disable = line-too-long

from unittest import TestCase

from tm.rules import apply_mult, apply_ops, make_exp as Exp


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

    def test_apply_ops(self):
        count1 = (-4 + (7 * Exp(2, 3))) // 3

        count2 = (-4 + (7 * (2 ** ((8 + (7 * Exp(2, 3))) // 3)))) // 3

        count3 = (-4 + (7 * (2 ** ((8 + (7 * (2 ** ((8 + (7 * Exp(2, 3))) // 3)))) // 3)))) // 3

        ops = (
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
