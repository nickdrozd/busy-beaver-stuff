from __future__ import annotations

from typing import TYPE_CHECKING
from unittest import TestCase

from tm.num import Add, Mul, Div, Exp

if TYPE_CHECKING:
    from tm.num import Count


class TestNum(TestCase):
    def assert_val(self, num: Count, val: int):
        self.assertEqual(int(num), val)
        self.assertEqual(int(-num), -val)

    def assert_string(self, num: Count, val: str):
        self.assertEqual(str(num), val)

    def test_add(self):
        num = Add(3, 4) + Add(5, 6)

        self.assert_string(num, "((3 + 4) + (5 + 6))")

        num -= 3
        num += 1
        num -= 3
        num += 1

        self.assert_val(num, 14)

        num -= 3

        self.assert_val(num, 11)

        for i in range(1, 12):
            num -= 1
            self.assert_val(num, 11 - i)

        self.assertEqual(num, 0)

    def test_add_mod(self):
        num: Count = Add(
            Add(6, 7),
            Add(8, 8))

        self.assert_val(num, 29)

        vals = {
            2: 1,
            3: 2,
            4: 1,
            5: 4,
            6: 5,
            7: 1,
            29: 0,
        }

        for mod, res in vals.items():
            self.assertEqual(num % mod, res)

        num += 1

        self.assert_val(num, 30)

        vals = {
            2: 0,
            3: 0,
            4: 2,
            5: 0,
            6: 0,
            7: 2,
            29: 1,
            30: 0,
        }

        for mod, res in vals.items():
            self.assertEqual(num % mod, res)

    def test_add_copy(self):
        num: Count = Add(Add(3, 4), Add(5, 6))

        self.assert_val(num, 18)

        _ = 1 + num

        num += 1

        self.assert_val(num, 19)

    def test_mul(self):
        num: Count = Mul(4, 5)

        self.assert_string(num, "(4 * 5)")

        self.assert_val(num, 20)

        num += 1
        num += 1
        num -= 3

        self.assert_val(num, 19)

        num *= 3

        self.assert_val(num, 57)

    def test_mul_mod(self):
        num = Mul(4, 5) * Mul(5, 7)

        self.assert_string(num, "((4 * 5) * (5 * 7))")

        self.assert_val(num, 700)

        vals = {
            1: 0,
            2: 0,
            3: 1,
            4: 0,
            5: 0,
            6: 4,
            7: 0,
            8: 4,
            9: 7,
        }

        for mod, res in vals.items():
            self.assertEqual(num % mod, res)

        num -= 1

        self.assert_val(num, 699)

        vals = {
            1: 0,
            2: 1,
            3: 0,
            4: 3,
            5: 4,
            6: 3,
            7: 6,
            8: 3,
            9: 6,
        }

        for mod, res in vals.items():
            self.assertEqual(num % mod, res)

    def test_div(self):
        vals = {
            (28, 3): 9,
            (28, 4): 7,
            (28, 5): 5,
            (28, 6): 4,
        }

        for (num, den), val in vals.items():
            self.assertEqual(int(Div(num, den)), val)

        self.assert_string(Div(12, 3), "(12 // 3)")

    def test_div_mod(self):
        vals = {
            (100, 2, 3): 2,
        }

        for (num, den, mod), val in vals.items():
            self.assertEqual(Div(num, den) % mod, val)

        for i in range(2, 10000):
            div = Div(Add(Exp(4, i), -4), 3)
            self.assertEqual(div % 2, 0)
            self.assertEqual(int(div) % 2, 0)

        div1 = Div(Add(Exp(4, 8188), -4), 3)

        self.assertEqual(div1 % 2, 0)
        self.assertEqual(int(div1) % 2, 0)

        div2 = (
            (2042 * Exp(4, 8188) + 7)
            + (18 * (1 + (Exp(4, 8188) - 4) // 3)) - 1
        ) // 2

        self.assertEqual(div2 % 2, 0)
        self.assertEqual(int(div2) % 2, 0)

    def test_exp(self):
        self.assertEqual(int(Exp(1, 8)), 1)
        self.assertEqual(int(Exp(6, 0)), 1)
        self.assertEqual(int(Exp(4, 5)), 1024)

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
