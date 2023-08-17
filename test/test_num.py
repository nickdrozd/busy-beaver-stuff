from __future__ import annotations

from typing import TYPE_CHECKING
from unittest import TestCase

from tm.num import Div, Exp

if TYPE_CHECKING:
    from tm.num import Count


class TestNum(TestCase):
    def assert_string(self, num: Count, val: str):
        self.assertEqual(str(num), val)

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
