from __future__ import annotations

from typing import TYPE_CHECKING
from unittest import TestCase

from tm.num import Exp

if TYPE_CHECKING:
    from tm.num import Count


class TestNum(TestCase):
    def assert_num(
            self,
            num: Count,
            val: int,
            rep: str | None = None,
            mod_rem: tuple[int, int] | None = None,
    ):
        self.assertEqual(val, int(num))

        self.assertEqual(-val, int(-num))

        if rep is not None:
            self.assertEqual(rep, str(num))

        self.assertEqual(
            val,
            eval(str(num)))  # pylint: disable = eval-used

        if mod_rem is not None:
            mod, rem = mod_rem

            self.assertEqual(
                num % mod,
                rem)

            self.assertEqual(
                int(num) % mod,
                rem)

    def assert_less(self, val1: Count, val2: Count):
        self.assertLess(val1, val2)
        self.assertLess(int(val1), int(val2))

    def test_div(self):
        self.assert_num(
            (-2 + Exp(2, 3)) // 3,
            2,
            "((-2 + (2 ** 3)) // 3)")

        self.assert_num(
            16 * ((-4 + Exp(2, 8)) // 3),
            1344,
            "((-64 + (2 ** 12)) // 3)")

        self.assert_num(
            21 * ((Exp(3, 4) - 3) // 2),
            819,
            "((-63 + (7 * (3 ** 5))) // 2)")

        self.assert_num(
            85 * ((Exp(2, 80) - 16) // 15),
            6850579644482898656668240,
            "((-1360 + (85 * (2 ** 80))) // 15)")

        self.assert_num(
            ((3 * Exp(5, 4)) // 5) // 15,
            25,
            "(5 ** 2)")

        self.assert_num(
            ((3 * Exp(5, 4)) // 15) // 5,
            25,
            "(5 ** 2)")

        self.assert_num(
            ((3 * Exp(2, 3)) // 4) * 3,
            18,
            "18")

        self.assert_num(
            (3 * Exp(5, 2)) // 5,
            15,
            "15")

        self.assert_num(
            (Exp(8, 4) - 64)  # type: ignore[operator]
                // (7 * Exp(3, 2)),
            64,
            "((-64 + (8 ** 4)) // (7 * (3 ** 2)))")

        self.assert_num(
            (6 * Exp(2, 3))  # type: ignore[operator]
                // (6 * Exp(2, 2)),
            2,
            "((3 * (2 ** 4)) // (3 * (2 ** 3)))")

    def test_div_mod(self):
        self.assert_num(
            (Exp(2, 2) * Exp(5, 2)) // 2,
            50,
            "(2 * (5 ** 2))",
            (3, 2))

        for i in range(2, 1000):
            self.assert_num(
                (Exp(4, i) - 4) // 3,
                ((4 ** i) - 4) // 3,
                f"((-4 + (2 ** {2 * i})) // 3)",
                (2, 0))

        div1 = (Exp(4, 8188) - 4) // 3

        self.assertEqual(div1 % 2, 0)
        self.assertEqual(int(div1) % 2, 0)

        div2 = (
            (2042 * Exp(4, 8188) + 7)
            + (18 * (1 + (Exp(4, 8188) - 4) // 3)) - 1
        ) // 2

        self.assertEqual(div2 % 2, 0)
        self.assertEqual(int(div2) % 2, 0)

        self.assert_num(
            ((3 * Exp(2, 3)) // 4),
            6,
            "6",
            (4, 2))

    def test_exp(self):
        self.assert_num(Exp(1, 8), 1)
        self.assert_num(int(Exp(6, 0)), 1)
        self.assert_num(Exp(4, 5), 1024)

        self.assert_num(Exp(2, 3), 8, "(2 ** 3)")

        self.assert_num(
            3 - Exp(2, 5),
            -29,
            "(3 + -(2 ** 5))")

        self.assert_num(
            Exp(2, 3) ** 5,
            32768,
            "(2 ** 15)")

        self.assert_num(
            (Exp(2, 3) * Exp(3, 2)) ** 5,
            1934917632,
            "(((2 ** 3) * (3 ** 2)) ** 5)")

        self.assert_num(
            2 ** Exp(3, 4),
            2417851639229258349412352,
            "(2 ** (3 ** 4))")

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

        self.assert_num(exp1 - exp2, 3)
        self.assert_num(exp2 - exp3, 3)
        self.assert_num(exp1 - exp3, 6)
        self.assert_num(exp3 - exp4, 0)

    def test_mul_triv(self):
        self.assert_num(0 * Exp(2, 3), 0)
        self.assert_num(1 * Exp(2, 3), 8)

    def test_mul_neg(self):
        self.assert_num(
            exp := 3 * Exp(2, 5),
            96,
            "(3 * (2 ** 5))")

        self.assert_num(
            -exp,
            -96,
            "(-3 * (2 ** 5))")

    def test_neg_exp(self):
        for e in range(1, 10):
            for b in range(2, 11):
                self.assert_num(
                    -Exp(b, e),
                    -(b ** e))

        self.assert_num(
            -Exp(2, 3),
            -8,
            "-(2 ** 3)")

        self.assert_num(
            -Exp(2, 4),
            -16,
            "-(2 ** 4)")

        self.assert_num(
            -64 * Exp(2, 20),
            -67108864,
            "-(2 ** 26)")

        self.assert_num(
            -64 * Exp(2, 21),
            -134217728,
            "-(2 ** 27)")

        self.assert_num(
            64 * Exp(2, 21),
            134217728,
            "(2 ** 27)")

        self.assert_num(
            64 * Exp(2, 20),
            67108864,
            "(2 ** 26)")

    def test_join_exp(self):
        exp = Exp(2, 17) * (4 + Exp(2, 15))

        self.assert_num(
            exp,
            (2 ** 17) * (4 + (2 ** 15)))

        self.assert_num(
            exp,
            8193 * (2 ** 19),
            "(8193 * (2 ** 19))")

        self.assert_num(
            exp,
            (2 ** 19) + ((2 ** 17) * (2 ** 15)))

        self.assert_num(
            Exp(2, -2 + Exp(3, 3)) + Exp(2, -3 + Exp(3, 3)),
            50331648,
            "(3 * (2 ** (-3 + (3 ** 3))))")

        self.assert_num(
            Exp(2, 5) + ((3 + Exp(3, 3)) * Exp(2, 3)),
            272,
            "((2 ** 3) * (7 + (3 ** 3)))")

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

    def test_exp_add(self):
        self.assert_num(
            Exp(2, 3) + Exp(2, 5),
            40,
            "(5 * (2 ** 3))")

        self.assert_num(
            Exp(2, 3) + (Exp(2, 5) * 7),
            232,
            "(29 * (2 ** 3))")

        self.assert_num(
            (Exp(2, 3) * 7) + Exp(2, 5),
            88,
            "(11 * (2 ** 3))")

        self.assert_num(
            (Exp(2, 3) * 7) + (Exp(2, 5) * 11),
            408,
            "(51 * (2 ** 3))")

    def test_exp_mul(self):
        self.assert_num(
            Exp(2, 3) * Exp(2, Exp(2, 5)),
            34359738368,
            "(2 ** (3 + (2 ** 5)))")

        self.assert_num(
            Exp(2, 3) * Exp(2, Exp(2, 4)),
            524288,
            "(2 ** (3 + (2 ** 4)))")

        self.assert_num(
            Exp(2, 3) * Exp(2, -3 + Exp(2, 4)),
            65536,
            "(2 ** (2 ** 4))")

        self.assert_num(
            (Exp(2, 3) * 5) + (Exp(2, 3) * Exp(2, -3 + Exp(2, 4))),
            65576,
            "((2 ** 3) * (5 + (2 ** (-3 + (2 ** 4)))))")

        self.assert_num(
            (Exp(2, 3) * 5) + (Exp(2, Exp(2, 4))),
            65576,
            "((2 ** 3) * (5 + (2 ** (-3 + (2 ** 4)))))")

        self.assert_num(
            (5 * Exp(2, 3)) + (Exp(2, Exp(2, 4))),
            65576,
            "((2 ** 3) * (5 + (2 ** (-3 + (2 ** 4)))))")

        self.assert_num(
            Exp(2, 3) * (5 + Exp(2, -3 + Exp(2, 4))),
            65576,
            "((2 ** 3) * (5 + (2 ** (-3 + (2 ** 4)))))")

        self.assert_num(
            Exp(2, 3) * (5 + Exp(2, Exp(2, 4))),
            524328,
            "((2 ** 3) * (5 + (2 ** (2 ** 4))))")

        self.assert_num(
            Exp(2, 3) * (Exp(2, 4) * (-1 + (Exp(3, 3) * Exp(2, 5)))),
            110464,
            "(-(2 ** 7) + ((2 ** 7) * ((3 ** 3) * (2 ** 5))))")

        self.assert_num(
            2 * (Exp(2, 3) * (1 + (Exp(3, 3) * Exp(2, 5)))),
            13840,
            "((2 ** 4) * (1 + ((3 ** 3) * (2 ** 5))))")

    def test_mul_add_neg(self):
        self.assert_num(
            -5 + (5 * Exp(2, 3)),
            35,
            "(-5 + (5 * (2 ** 3)))")

    def test_mul_add(self):
        self.assert_num(
            (Exp(2, 3) * Exp(3, 2)) + (Exp(2, 3) * Exp(5, 3)),
            1072,
            "((2 ** 3) * ((3 ** 2) + (5 ** 3)))")

        self.assert_num(
            (Exp(3, 2) * Exp(2, 3)) + (Exp(5, 3) * Exp(2, 3)),
            1072,
            "((2 ** 3) * ((3 ** 2) + (5 ** 3)))")
