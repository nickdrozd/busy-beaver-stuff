# pylint: disable = line-too-long, expression-not-assigned

from __future__ import annotations

from math import log10
from typing import TYPE_CHECKING
from unittest import TestCase

import tm.num as num_mod
from tm.num import Exp, NUM_COUNTS

if TYPE_CHECKING:
    from tm.num import Count


def profile_nums(func):  # no-coverage
    def wrapper(*args, **kwargs):
        num_mod.PROFILE = True

        try:
            func(*args, **kwargs)
        finally:
            num_mod.PROFILE = False

    return wrapper


def assert_num_counts(expected: dict[str, int]):
    err = None

    try:
        assert NUM_COUNTS == expected, NUM_COUNTS
    except AssertionError as ass:  # no-coverage
        err = str(ass)
    finally:
        for cat in NUM_COUNTS:
            NUM_COUNTS[cat] = 0

    if err:  # no-coverage
        raise AssertionError(err)


class TestNum(TestCase):
    @classmethod
    def setUpClass(cls):
        num_mod.PROFILE = True

    @classmethod
    def tearDownClass(cls):
        num_mod.PROFILE = False

        assert_num_counts({
            'adds': 2418,
            'muls': 1639,
            'divs': 2112,
            'exps': 1828,
        })

    def assert_mod(
            self,
            num: Count,
            mod: int,
            rem: int,
            skip_num: bool = False,
    ):
        self.assertEqual(
            num % mod,
            rem)

        if skip_num:
            return

        self.assertEqual(
            int(num) % mod,
            rem)

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

            self.assert_mod(num, mod, rem)

    def assert_less(self, val1: Count, val2: Count):
        self.assertLess(val1, val2)
        self.assertLess(int(val1), int(val2))

    def assert_depth(self, val: Count, depth: int):
        assert not isinstance(val, int)
        self.assertEqual(val.depth, depth)

    def assert_estimate(self, val: Count, estimate: int):
        assert not isinstance(val, int)

        self.assertEqual(
            val.estimate(),
            int(log10(int(val))))

        self.assertEqual(
            val.estimate(),
            estimate)

    def test_depth(self):
        self.assert_depth(Exp(3, 3), 1)
        self.assert_depth(-Exp(2, 3), 2)
        self.assert_depth(Exp(3, 3) + -Exp(2, 3), 3)

    def test_estimate(self):
        self.assert_estimate(
            (469761947 + (19 * Exp(2, 69174))) // 9,
            20823)

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
            "((-272 + (17 * (2 ** 80))) // 3)")

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
            (-72 + (9 * (2 ** (-5 + Exp(2, 5))))) // 3,
            402653160,
            "(-24 + (3 * (2 ** (-5 + (2 ** 5)))))")

        self.assert_num(
            (2 * Exp(3, 13)) * ((-1 + Exp(3, 7)) // 2),
            3485190078,
            "((3 ** 13) * (-1 + (3 ** 7)))")

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

        self.assert_mod(
            (Exp(4, 8188) - 4) // 3,
            2,
            0,
            skip_num = True)

        self.assert_mod(
            ((2042 * Exp(4, 8188)) + 7)
            + ((18 * (1 + (Exp(4, 8188) - 4) // 3)) - 1),
            2,
            0)

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
            -54 + Exp(3, 8),
            6507,
            "(-54 + (3 ** 8))")

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

        self.assert_num(
            76 + (19 * Exp(2, 5)),
            684,
            "(76 + (19 * (2 ** 5)))")

        self.assert_num(
            19 * (4 + Exp(2, 5)),
            684,
            "(76 + (19 * (2 ** 5)))")

        self.assert_num(
            19 * (Exp(2, 2) + Exp(2, 5)),
            684,
            "((19 * (2 ** 2)) * (1 + (2 ** 3)))")

        self.assert_num(
            (19 * Exp(2, 2)) + (19 * Exp(2, 5)),
            684,
            "((19 * (2 ** 2)) * (1 + (2 ** 3)))")

        self.assert_num(
            19 * (Exp(2, 2) * (1 + Exp(2, 3))),
            684,
            "((19 * (2 ** 2)) * (1 + (2 ** 3)))")

        self.assert_num(
            Exp(2, 2) * (19 * (1 + Exp(2, 3))),
            684,
            "((19 * (2 ** 2)) * (1 + (2 ** 3)))")

        self.assert_num(
            Exp(2, 2) * (19 + (19 * Exp(2, 3))),
            684,
            "((19 * (2 ** 2)) * (1 + (2 ** 3)))")


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
            self.assert_mod(
                Exp(base, exp),
                mod,
                val)

        self.assertEqual(
            divmod(Exp(2, 3), 2),
            (Exp(2, 2), 0))

        self.assert_num(
            3 * Exp(2, 3),
            24,
            "(3 * (2 ** 3))",
            (2, 0))

    def test_eq_sub(self):
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
        for exp in range(1, 10):
            for base in range(2, 11):
                self.assert_num(
                    -Exp(base, exp),
                    -(base ** exp))

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
            "((2 ** 19) * (1 + (2 ** 13)))")

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
            "((2 ** 3) * (3 + ((2 ** 2) + (3 ** 3))))")

    def test_comparisons(self):
        self.assert_less(
            -3 + Exp(2, 4),
            -3 + Exp(2, 5))

        self.assert_less(
            -3 + Exp(2, 5),
            -2 + Exp(2, 5))

        self.assert_less(
            3 * Exp(2, 4),
            3 * Exp(2, 5))

        self.assert_less(
            3 * Exp(2, 5),
            5 * Exp(2, 5))

        self.assertLess(
            4 + Exp(2, 4),
            3 + Exp(2, 5))

        self.assert_less(
            Exp(3, 4),
            Exp(5, 6))

        self.assertGreaterEqual(
            Exp(5, 6),
            Exp(3, 4))

        self.assertLess(
            Exp(2, 3),
            Exp(2, 3) + Exp(3, 3))

        with self.assertRaises(NotImplementedError):
            Exp(2, 5) < Exp(3, 4)

        self.assertLess(
            3 + Exp(2, 3),
            4 * Exp(2, 5))

        self.assertLess(
            -1 + (3 * Exp(2, 3)),
            3 * Exp(2, 3))

        self.assertLess(
            3 * Exp(2, 3),
            1 + (3 * Exp(2, 3)))

        self.assertLess(
            (1 + Exp(2, 3)) // 3,
            (1 + Exp(2, 5)) // 3)

        self.assertLess(
            1,
            (1 + Exp(2, 3)) // 3)

        self.assertLess(
            Exp(2, 5) * (-1 + (2 ** (-5 + Exp(2, 5)))),
            -1 + (Exp(2, 5) * (-1 + (2 ** (-5 + (2 ** Exp(2, 5)))))))

        self.assertGreater(
            ((Exp(2, 5) * (-1 + Exp(2, (-5 + Exp(2, 5))))) + -(Exp(2, 5) * (-1 + Exp(2, (-5 + Exp(2, Exp(2, 5))))))) + (-(Exp(2, 5) * (-1 + Exp(2, (-5 + Exp(2, 5))))) + (Exp(2, 5) * (-1 + Exp(2, (-5 + Exp(2, Exp(2, Exp(2, Exp(2, 5))))))))),
            (-(Exp(2, 5) * (-1 + Exp(2, (-5 + Exp(2, 5))))) + (Exp(2, 5) * (-1 + Exp(2, (-5 + (2 ** Exp(2, Exp(2, 5)))))))) + ((Exp(2, 5) * (-1 + Exp(2, (-5 + Exp(2, 5))))) + -(Exp(2, 5) * (-1 + Exp(2, (-5 + Exp(2, Exp(2, 5))))))))

        self.assertLess(
            -(Exp(2, 5) * (-1 + Exp(2, (-13 + Exp(2, 5))))) + (Exp(2, 5) * (-1 + Exp(2, (-13 + Exp(2, (-7 + Exp(2, 5))))))),
            1 + (-(Exp(2, 5) * (-1 + Exp(2, (-13 + Exp(2, 5))))) + (Exp(2, 5) * (-1 + Exp(2, (-13 + Exp(2, (-7 + Exp(2, 5)))))))))

    def test_exp_add(self):
        self.assert_num(
            Exp(2, 3) + Exp(2, 5),
            40,
            "((2 ** 3) * (1 + (2 ** 2)))")

        self.assert_num(
            Exp(2, 3) + (Exp(2, 5) * 7),
            232,
            "((2 ** 3) * (1 + (7 * (2 ** 2))))")

        self.assert_num(
            (Exp(2, 3) * 7) + Exp(2, 5),
            88,
            "((2 ** 3) * (7 + (2 ** 2)))")

        self.assert_num(
            (Exp(2, 3) * 7) + (Exp(2, 5) * 11),
            408,
            "((2 ** 3) * (7 + (11 * (2 ** 2))))")

        self.assert_num(
            ((2 ** (3 * Exp(2, 3))) + (2 ** (-1 + (3 * Exp(2, 3))))),
            25165824,
            "(3 * (2 ** (-1 + (3 * (2 ** 3)))))")

        self.assert_num(
            ((2 ** (3 * Exp(2, 3))) + (2 ** (1 + (3 * Exp(2, 3))))),
            50331648,
            "(3 * (2 ** (3 * (2 ** 3))))")

        self.assert_num(
            Exp(3, 18) + ((2 * Exp(3, 18)) * ((-1 + Exp(3, 4)) // 2)),
            31381059609,
            "(3 ** 22)")

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
            Exp(2, 4) * (Exp(3, 3) * Exp(2, 5)),
            13824,
            "((3 ** 3) * (2 ** 9))")

        self.assert_num(
            Exp(2, 3) * (Exp(2, 4) * (-1 + (Exp(3, 3) * Exp(2, 5)))),
            110464,
            "((2 ** 7) * (-1 + ((3 ** 3) * (2 ** 5))))")

        self.assert_num(
            2 * (Exp(2, 3) * (1 + (Exp(3, 3) * Exp(2, 5)))),
            13840,
            "((2 ** 4) * (1 + ((3 ** 3) * (2 ** 5))))")

        self.assert_num(
            Exp(2, 10) * (-Exp(2, 6) * (1 + Exp(2, 4))),
            -1114112,
            "-((2 ** 16) * (1 + (2 ** 4)))")

        self.assert_num(
            Exp(2, 4) * (1 + (Exp(2, 3) * (1 + Exp(2, 2)))),
            656,
            "((2 ** 4) * (1 + ((2 ** 3) * (1 + (2 ** 2)))))")

        self.assert_num(
            Exp(2, 4) * (1 + (Exp(2, 3) + (Exp(2, 3) * Exp(2, 2)))),
            656,
            "((2 ** 4) * (1 + ((2 ** 3) * (1 + (2 ** 2)))))")

        self.assert_num(
            (Exp(2, 4) + ((Exp(2, 4) * Exp(2, 3)) + (Exp(2, 4) * (Exp(2, 3) * Exp(2, 2))))),
            656,
            "((2 ** 4) * (1 + ((2 ** 3) * (1 + (2 ** 2)))))")

        self.assert_num(
            (Exp(2, 4) + (Exp(2, 7) + Exp(2, 9))),
            656,
            "((2 ** 4) * (1 + ((2 ** 3) * (1 + (2 ** 2)))))")

        self.assert_num(
            (Exp(2, 4) + (Exp(2, 4) * (Exp(2, 3) * (1 + Exp(2, 2))))),
            656,
            "((2 ** 4) * (1 + ((2 ** 3) * (1 + (2 ** 2)))))")

    def test_mul_add_neg(self):
        self.assert_num(
            -5 + (5 * Exp(2, 3)),
            35,
            "(-5 + (5 * (2 ** 3)))")

        self.assert_num(
            -(Exp(2, 5) * (-1 + (2 ** (-5 + Exp(2, 5)))))
                + (Exp(2, 5) * (-1 + (2 ** (-5 + Exp(2, 5))))),
            0,
            "0")

        self.assert_num(
            (Exp(2, 5) * (-1 + (2 ** (-5 + Exp(2, 5)))))
                + -(Exp(2, 5) * (-1 + (2 ** (-5 + Exp(2, 5))))),
            0,
            "0")

        self.assert_num(
            (-20 + (Exp(2, (-5 + (13 * Exp(2, 2))) // 3) * (3 + (5 * Exp(2, (-2 + ((13 * Exp(2, 2)) + -(-5 + (13 * Exp(2, 2))))) // 3))))) // 3,
            141988,
            "((-20 + (13 * (2 ** ((-5 + (13 * (2 ** 2))) // 3)))) // 3)")

    def test_mul_add(self):
        self.assert_num(
            (Exp(2, 3) * Exp(3, 2)) + (Exp(2, 3) * Exp(5, 3)),
            1072,
            "((2 ** 3) * ((3 ** 2) + (5 ** 3)))")

        self.assert_num(
            (Exp(3, 2) * Exp(2, 3)) + (Exp(5, 3) * Exp(2, 3)),
            1072,
            "((2 ** 3) * ((3 ** 2) + (5 ** 3)))")

    def test_add_neg(self):
        self.assert_num(
            -5 + (5 * Exp(2, 3)),
            35,
            "(-5 + (5 * (2 ** 3)))")

        self.assert_num(
            5 * (Exp(2, 3) - 1),
            35,
            "(-5 + (5 * (2 ** 3)))")

        self.assert_num(
            Exp(2, 3) + Exp(2, 5),
            40,
            "((2 ** 3) * (1 + (2 ** 2)))")

        self.assert_num(
            -Exp(2, 3) + -Exp(2, 5),
            -40,
            "-((2 ** 3) * (1 + (2 ** 2)))")

        self.assert_num(
            -Exp(2, 3) + Exp(2, 5),
            24,
            "((2 ** 3) * (-1 + (2 ** 2)))")

        self.assert_num(
            -Exp(2, 3) + (-Exp(2, 5) + Exp(3, 3)),
            -13,
            "(-(2 ** 3) + ((3 ** 3) + -(2 ** 5)))")

        self.assert_num(
            -Exp(2, 3) + (Exp(2, 5) * Exp(3, 3)),
            856,
            "(-(2 ** 3) + ((2 ** 5) * (3 ** 3)))")

        self.assert_num(
            -Exp(2, 2) + (-Exp(2, 3) + (Exp(2, 5) * Exp(3, 3))),
            852,
            "(((2 ** 5) * (3 ** 3)) + -(3 * (2 ** 2)))")

    def test_div_gcd(self):
        self.assert_num(
            (-320 + (5 * Exp(2, 20))) // 12,
            436880,
            "((-80 + (5 * (2 ** 18))) // 3)")

        self.assert_num(
            (-160 + (5 * Exp(2, 19))) // 6,
            436880,
            "((-80 + (5 * (2 ** 18))) // 3)")

        self.assert_num(
            (-80 + (5 * Exp(2, 18))) // 3,
            436880,
            "((-80 + (5 * (2 ** 18))) // 3)")

        self.assert_num(
            (Exp(2, 4) * Exp(3, 3)) // 12,
            36,
            "((2 ** 2) * (3 ** 2))")

        self.assert_num(
            (Exp(2, 6) * (-5 + (5 * Exp(2, 14)))) // 12,
            436880,
            "(((2 ** 4) * (-5 + (5 * (2 ** 14)))) // 3)")

        self.assert_num(
            (13164 + ((70837131 * Exp(2, 12)) + ((-118784 + (29 * Exp(2, 12))) // 9))) // 3,
            96716300580,
            "((-308 + (19 * (2 ** 37))) // 27)")

        self.assert_num(
            (384 + (Exp(2, 5) * (1 + (13 * Exp(2, 7))))) // 24,
            2236,
            "((48 + ((2 ** 2) * (1 + (13 * (2 ** 7))))) // 3)")

    def test_div_neg(self):
        self.assert_num(
            (-1 + Exp(3, 3)) // 2,
            13,
            "((-1 + (3 ** 3)) // 2)")

        self.assert_num(
            -((-1 + Exp(3, 3)) // 2),
            -13,
            "((1 + -(3 ** 3)) // 2)")

        self.assert_num(
            ((-1 + Exp(3, 3)) // 2) - ((-1 + Exp(3, 2)) // 2),
            9,
            "(3 ** 2)")

        self.assert_num(
            -((-1 + Exp(3, 3)) // 2) - -((-1 + Exp(3, 2)) // 2),
            -9,
            "-(3 ** 2)")

    def test_div_mul(self):
        self.assert_num(
            (-1 + Exp(3, 3)) // 2,
            13,
            "((-1 + (3 ** 3)) // 2)")

        self.assert_num(
            -((-1 + Exp(3, 3)) // 2),
            -13,
            "((1 + -(3 ** 3)) // 2)")

        self.assert_num(
            ((-1 + Exp(3, 3)) // 2) * 3,
            39,
            "((-3 + (3 ** 4)) // 2)")

        self.assert_num(
            ((-1 + Exp(3, 3)) // 2) * Exp(3, 5),
            3159,
            "(((3 ** 5) * (-1 + (3 ** 3))) // 2)")

        self.assert_num(
            -((-1 + Exp(3, 3)) // 2) * Exp(3, 5),
            -3159,
            "(((3 ** 5) * (1 + -(3 ** 3))) // 2)")

        self.assert_num(
            -(((-1 + Exp(3, 3)) // 2) * Exp(3, 5)),
            -3159,
            "(-((3 ** 5) * (-1 + (3 ** 3))) // 2)")

        self.assert_num(
            -(((1 + Exp(3, 3)) // 2) * Exp(3, 5)),
            -3402,
            "(-((3 ** 5) * (1 + (3 ** 3))) // 2)")

        self.assert_num(
            -(((1 + Exp(3, 3)) * Exp(3, 5)) // 2),
            -3402,
            "(-((3 ** 5) * (1 + (3 ** 3))) // 2)")

        self.assert_num(
            -(((1 * Exp(3, 5)) + (Exp(3, 3) * Exp(3, 5))) // 2),
            -3402,
            "(-((3 ** 5) * (1 + (3 ** 3))) // 2)")

        self.assert_num(
            -((Exp(3, 5) + Exp(3, 8)) // 2),
            -3402,
            "(-((3 ** 5) * (1 + (3 ** 3))) // 2)")

        self.assert_num(
            -(Exp(3, 5) * (1 + Exp(3, 3))),
            -6804,
            "-((3 ** 5) * (1 + (3 ** 3)))")

        self.assert_num(
            -Exp(3, 5) * (1 + Exp(3, 3)),
            -6804,
            "-((3 ** 5) * (1 + (3 ** 3)))")

        self.assert_num(
            Exp(3, 5) * -(1 + Exp(3, 3)),
            -6804,
            "-((3 ** 5) * (1 + (3 ** 3)))")

        self.assert_num(
            Exp(3, 5) * (-1 + -Exp(3, 3)),
            -6804,
            "-((3 ** 5) * (1 + (3 ** 3)))")

    def test_cycle_mod(self):
        self.assert_mod(
            (-11 + Exp(3, ((21 + Exp(3, ((7 + Exp(3, ((23 + Exp(3, ((7 + Exp(3, ((23 + Exp(3, ((7 + Exp(3, ((21 + Exp(3, ((7 + Exp(3, ((23 + Exp(3, ((7 + Exp(3, 22146)) // 8))) // 8))) // 8))) // 8))) // 8))) // 8))) // 8))) // 8))) // 8))) // 8))) // 2,
            4,
            3,
            skip_num = True)
