from __future__ import annotations

from typing import TYPE_CHECKING
from unittest import TestCase

from tm.num import Div, Exp

if TYPE_CHECKING:
    from tm.num import Count


class TestNum(TestCase):
    def assert_val(self, num: Count, val: int):
        self.assertEqual(int(num), val)

        if not isinstance(num, Div):
            self.assertEqual(int(-num), -val)

    def assert_string(self, num: Count, val: str):
        self.assertEqual(str(num), val)

    def assert_less(self, val1: Count, val2: Count):
        self.assertLess(val1, val2)
        self.assertLess(int(val1), int(val2))

    def test_div(self):
        vals = {
            3: 9,
            4: 7,
            5: 5,
            6: 4,
        }

        for den, val in vals.items():
            self.assert_val((7 * Exp(2, 2)) // den, val)

        div = (-2 + Exp(2, 3)) // 3

        self.assert_val(div, 2)

        self.assert_string(div, "((-2 + (2 ** 3)) // 3)")

    def test_div_mod(self):
        self.assertEqual(
            ((Exp(2, 2) * Exp(5, 2)) // 2) % 3,
            2)

        for i in range(2, 10000):
            div = (Exp(4, i) - 4) // 3
            self.assertEqual(div % 2, 0)
            self.assertEqual(int(div) % 2, 0)

        div1 = (Exp(4, 8188) - 4) // 3

        self.assertEqual(div1 % 2, 0)
        self.assertEqual(int(div1) % 2, 0)

        div2 = (
            (2042 * Exp(4, 8188) + 7)
            + (18 * (1 + (Exp(4, 8188) - 4) // 3)) - 1
        ) // 2

        self.assertEqual(div2 % 2, 0)
        self.assertEqual(int(div2) % 2, 0)

    def test_exp(self):
        self.assert_val(Exp(1, 8), 1)
        self.assert_val(int(Exp(6, 0)), 1)
        self.assert_val(Exp(4, 5), 1024)

        self.assert_string(Exp(2, 3), "(2 ** 3)")

    def test_exp_mod(self):
        vals = {
            (4, 5, 3): 1,
            (5, 3, 13): 8,
            (4, 13, 497): 445,
            (2, 40, 13): 3,
            (2, 50, 13): 4,
            (2, 90, 13): 12,
        }

        for (base, exp, mod), val in vals.items():
            self.assertEqual(Exp(base, exp) % mod, val)

    def test_eq_sub(self):
        # pylint: disable = line-too-long

        exp1 = (6584 + (2266788192 * Exp(64, 11524))) + (6496 * ((-64 + Exp(64, 11524)) // 63))
        exp2 = (6581 + (2266788192 * Exp(64, 11524))) + (6496 * ((-64 + Exp(64, 11524)) // 63))
        exp3 = (6578 + (2266788192 * Exp(64, 11524))) + (6496 * ((-64 + Exp(64, 11524)) // 63))
        exp4 = (6578 + (2266788192 * Exp(64, 11524))) + (6496 * ((-64 + Exp(64, 11524)) // 63))

        self.assertEqual(exp1, exp1)
        self.assertEqual(exp2, exp2)
        self.assertEqual(exp3, exp3)
        self.assertEqual(exp3, exp4)
        self.assertNotEqual(exp1, exp2)
        self.assertNotEqual(exp1, exp3)
        self.assertNotEqual(exp2, exp3)

        self.assert_val(exp1 - exp2, 3)
        self.assert_val(exp2 - exp3, 3)
        self.assert_val(exp1 - exp3, 6)
        self.assert_val(exp3 - exp4, 0)

    def test_mul_triv(self):
        self.assert_val(0 * Exp(2, 3), 0)
        self.assert_val(1 * Exp(2, 3), 8)

    def test_mul_neg(self):
        exp = 3 * Exp(2, 5)

        self.assert_val(exp, 96)

        self.assert_string(
            exp,
            "(3 * (2 ** 5))")

        neg = -exp

        self.assert_val(neg, -96)

        self.assert_string(
            neg,
            "(-3 * (2 ** 5))")

    def test_neg_exp(self):
        for e in range(1, 10):
            for b in range(2, 11):
                self.assert_val(
                    -Exp(b, e),
                    -(b ** e))

    def test_join_exp(self):
        exp = Exp(2, 17) * (4 + Exp(2, 15))

        self.assert_val(
            exp,
            (2 ** 17) * (4 + (2 ** 15)))

        self.assert_val(
            exp,
            8193 * (2 ** 19))

        self.assert_val(
            exp,
            (2 ** 19) + ((2 ** 17) * (2 ** 15)))

        self.assert_string(
            exp,
            "(8193 * (2 ** 19))")

    def test_cover(self):
        self.assert_val(3 - Exp(2, 5), -29)

        self.assert_val(Exp(2, 3) ** 5, 32768)

        self.assert_val(2 ** Exp(3, 4), 2417851639229258349412352)

        self.assert_val(((3 * Exp(2, 3)) // 4) * 3, 18)

        self.assert_val((3 * Exp(5, 2)) // 5, 15)

        self.assert_val(
            (Exp(8, 4) - 64)  # type: ignore[operator]
                // (7 * Exp(3, 2)),
            64)

        self.assert_val(
            (6 * Exp(2, 3))  # type: ignore[operator]
                // (6 * Exp(2, 2)),
            2)

        self.assertEqual(((3 * Exp(2, 3)) // 4) % 4, 0)

    def test_comparisons(self):
        self.assert_less(
            -3 + Exp(2, 4),
            -3 + Exp(2, 5))

        self.assert_less(
            -3 + Exp(2, 5),
            -2 + Exp(2, 5))

        with self.assertRaises(NotImplementedError):
            _ = 3 + Exp(2, 5) < 4 + Exp(2, 4)

        self.assert_less(
            Exp(3, 4),
            Exp(5, 6))

        self.assertGreaterEqual(
            Exp(5, 6),
            Exp(3, 4))

        with self.assertRaises(NotImplementedError):
            _ = Exp(2, 5) < Exp(3, 4)

        with self.assertRaises(NotImplementedError):
            _ = 3 + Exp(2, 3) < 4 * Exp(2, 5)
