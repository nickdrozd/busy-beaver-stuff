# pylint: disable = line-too-long, too-many-lines, expression-not-assigned

from __future__ import annotations

from typing import TYPE_CHECKING
from unittest import TestCase

import tm.num as num_mod
from tm.num import Exp, Tet, show_number, NUM_COUNTS, NumException

if TYPE_CHECKING:
    from tm.num import Count


def profile_nums(func):  # no-cover
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
    except AssertionError:  # no-cover
        err = [
            f'            "{cat}": {val},'
            for cat, val in sorted(NUM_COUNTS.items())
        ]
    finally:
        for cat in NUM_COUNTS:
            NUM_COUNTS[cat] = 0

    if err:  # no-cover
        raise AssertionError(
            '\n' + '\n'.join(err))


class TestNum(TestCase):
    @classmethod
    def setUpClass(cls):
        num_mod.PROFILE = True

    @classmethod
    def tearDownClass(cls):
        num_mod.PROFILE = False

        assert_num_counts({
            "adds": 2641,
            "divs": 2147,
            "exps": 2299,
            "muls": 1877,
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

    def assert_rep(self, num: Count, rep: str):
        self.assertEqual(
            str(num),
            rep)

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
            self.assert_rep(num, rep)

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

    def assert_digits(self, val: Count, digits: int):
        assert not isinstance(val, int)
        self.assertEqual(digits, val.digits())

    def assert_estimate(self, val: Count, estimate: Count):
        assert not isinstance(val, int)

        self.assertEqual(
            val.estimate(),
            estimate)

    def test_subexpression(self):
        expr = (-4 + (7 * (2 ** ((8 + (7 * Exp(2, 3))) // 3)))) // 3

        self.assertIn(
            7 * Exp(2, 3),
            expr)  # type: ignore[arg-type]

        self.assertNotIn(
            -4 + (7 * Exp(2, 3)),
            expr)  # type: ignore[arg-type]

        self.assertNotIn(
            (-4 + (7 * Exp(2, 3))) // 3,
            expr)  # type: ignore[arg-type]

        self.assertNotIn(
            expr,
            Tet(10, 2))  # type: ignore[arg-type]

    def test_depth(self):
        self.assert_depth(Exp(3, 3), 1)
        self.assert_depth(-Exp(2, 3), 2)
        self.assert_depth(Exp(3, 3) + -Exp(2, 3), 3)

        self.assert_num(
            ((Exp(2, 3) * (-1 + (2 ** (-3 + (2 ** Exp(2, 3))))))
                 + -(Exp(2, 3) * (-1 + (2 ** (-3 + Exp(2, 3)))))),
            115792089237316195423570985008687907853269984665640564039457584007913129639680,
            "(((2 ** 3) * (-1 + (2 ** (-3 + (2 ** (2 ** 3)))))) + -((2 ** 3) * (-1 + (2 ** (-3 + (2 ** 3))))))")

        self.assert_num(
            (-(Exp(2, 3) * (-1 + (2 ** (-3 + Exp(2, 3)))))
                + (Exp(2, 3) * (-1 + (2 ** (-3 + (2 ** Exp(2, 3))))))),
            115792089237316195423570985008687907853269984665640564039457584007913129639680,
            "(-((2 ** 3) * (-1 + (2 ** (-3 + (2 ** 3))))) + ((2 ** 3) * (-1 + (2 ** (-3 + (2 ** (2 ** 3)))))))")

    def test_digits(self):
        self.assert_digits(
            Exp(2, 2147483647),
            646456993)

        self.assert_digits(
            Exp(2, 11),
            3)

        self.assert_digits(
            (Exp(2, 11) * Exp(3, 11)),
            8)

        self.assert_digits(
            (Exp(3, 11) - 1) // 2,
            5)

        self.assert_digits(
            2 ** 2 ** 2 ** Exp(2, 2),
            19728)

        self.assert_digits(
            3 ** Exp(3, 3),
            13)

        self.assert_digits(
            4 ** Exp(4, 4),
            154)

        self.assert_digits(
            5 ** Exp(5, 5),
            2184)

        self.assert_digits(
            6 ** Exp(6, 6),
            36305)

        with self.assertRaises(OverflowError):
            Tet(10, 3).digits()

    def test_estimate(self):
        self.assert_estimate(
            -1 + Exp(2, 31),
            Exp(10, 9))

        self.assert_estimate(
            Exp(2, 2147483647),
            Exp(10, 646456993))

        for i in range(1, 4):
            self.assert_estimate(
                i + (i * Exp(2, 2147483647)),
                Exp(10, 646456993))

        self.assert_estimate(
            Exp(2, 5),
            Exp(10, 2))

        self.assert_estimate(
            2 ** Exp(2, 5),
            Exp(10, 10))

        self.assert_estimate(
            2 ** 2 ** Exp(2, 5),
            Tet(10, 3))

        self.assert_estimate(
            (469761947 + (19 * Exp(2, 69174))) // 9,
            Exp(10, 20823))

        self.assert_estimate(
            -(469761947 + (19 * Exp(2, 69174))) // 9,
            -Exp(10, 20821))

        self.assert_estimate(
            13 * Exp(2, 345103),
            Exp(10, 103887))

        self.assert_estimate(
            13 * -Exp(2, 345103),
            -Exp(10, 103885))

        self.assert_estimate(
            -13 * Exp(2, 345103),
            -Exp(10, 103885))

        self.assert_estimate(
            -13 * -Exp(2, 345103),
            Exp(10, 103887))

        self.assert_estimate(
            Exp(2, 33) * (2 + Exp(2, 35)),
            Exp(10, 20))

        self.assert_estimate(
            7 * (2 ** (-3 + (7 * Exp(2, 111)))),
            Tet(10, 3))

        self.assert_estimate(
            Tet(10, 2),
            Tet(10, 2))

        self.assert_estimate(
            10 ** 10 ** 10 ** Exp(10, 10),
            Tet(10, 5))

    def test_add(self):
        self.assert_num(
            -(Exp(2, 3) + Exp(3, 2)),
            -17,
            "-((2 ** 3) + (3 ** 2))")

        self.assert_num(
            -((3 + Exp(2, 3)) + Exp(3, 2)),
            -20,
            "(-3 + -((3 ** 2) + (2 ** 3)))")

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

        self.assertEqual(
            "(~10^19728)",
            show_number(
                int(2 ** 2 ** 2 ** Exp(2, 2))))

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

        self.assert_num(
            2 * Exp(3, 2),
            18,
            "(2 * (3 ** 2))",
            (2, 0))

        self.assert_num(
            Exp(3, 11) - 1,
            177146,
            "(-1 + (3 ** 11))",
            (2, 0))

        with self.assertRaises(NumException):
            (-3 + (3 ** ((-3 + Exp(3, 5)) // 8))) % 2

    def test_exp_mod_special_case(self):
        self.assert_mod(
            3 ** ((7 + Exp(3, 2)) // 8),
            6,
            3)

        self.assert_mod(
            2 ** ((-2 + (13 * Exp(2, 3))) // 3),
            12,
            4)

        self.assert_mod(
            2 ** ((-5 + (13 * Exp(2, 3))) // 3),
            6,
            2)

        self.assert_mod(
            2 ** ((-55 + (61 * Exp(2, 8))) // 9),
            54,
            2)

        self.assert_mod(
            2 ** ((-11 + (19 * Exp(2, 7))) // 9),
            162,
            122)

        self.assert_mod(
            2 ** ((-55 + (61 * Exp(2, 8))) // 9),
            486,
            326)

        self.assert_mod(
            2 ** ((62 + (49 * Exp(2, 16))) // 27),
            1458,
            1126)

    def test_mod_1(self):
        nums = (
            Exp(2, 3),
            Exp(2, 3) + Exp(3, 2),
            Exp(2, 3) * Exp(3, 2),
            (1 + Exp(2, 3)) // 3,
        )

        for num in nums:
            self.assert_mod(num, 1, 0)

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
        self.assert_num(((1 + Exp(2, 3)) // 3) // 1, 3)

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
            2 ** (-2 + Exp(3, 3)) + 2 ** (-3 + Exp(3, 3)),
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

        self.assertFalse(
            Exp(2, 4)
                < Exp(2, 4) + -Exp(3, 2))

        self.assertGreaterEqual(
            Exp(5, 6),
            Exp(3, 4))

        self.assertFalse(
            Exp(2, 3) < -Exp(2, 5))

        self.assertLess(
            Exp(2, 3),
            Exp(2, 3) + Exp(3, 3))

        with self.assertRaises(NotImplementedError):
            Exp(2, 5) < Exp(3, 4)

        self.assertLess(
            Exp(2, 3),
            (Exp(2, 3) * (1 + Exp(2, 3))))

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
            (49 + (13 * Exp(2, 15))),
            (113 + (13 * Exp(2, 16))))

        self.assertFalse(
            Exp(10, (3 + Exp(10, Exp(10, 3))))
                < Exp(10, (3 + Exp(10, 3))))

        self.assertTrue(
            Exp(10, (3 + Exp(10, 3)))
                < Exp(10, (3 + Exp(10, Exp(10, 3)))))

        self.assertLess(
            Exp(2, 3) - Exp(3, 2),
            Exp(2, 3))

        with self.assertRaises(NotImplementedError):
            self.assertLess(
                Exp(2, 3) * (-1 + Exp(2, 3)),
                2 ** (-3 + Exp(2, 3)))

        self.assertLess(
            -Exp(10, 14050258128),
            Exp(10, 14050259810))

        self.assertLess(
            Exp(2, 5) * (-1 + (2 ** (-5 + Exp(2, 5)))),
            -1 + (Exp(2, 5) * (-1 + (2 ** (-5 + (2 ** Exp(2, 5)))))))

        self.assertLess(
            Exp(2, 5) * (-1 + (2 ** (-5 + Exp(2, 5)))),
            Exp(2, 5) * (-1 + (2 ** (-5 + (2 ** Exp(2, 5))))))

        self.assertLess(
            -1 + (2 ** (-5 + Exp(2, 5))),
            -1 + (2 ** (-5 + (2 ** Exp(2, 5)))))

        self.assertLess(
            2 ** (-5 + Exp(2, 5)),
            2 ** (-5 + (2 ** Exp(2, 5))))

        self.assertFalse(
            -(Exp(2, 65536) * (-1 + 2 ** (-65536 + Exp(2, 65536))))
            < -(Exp(2, 65536) * (-1 + 2 ** (-65536 + Exp(2, 65536)))))

        self.assertFalse(
            ((1 + Exp(2, 2)) * 2 ** (-3 + (Exp(2, 655357) * (1 + Exp(2, 2)))))
                < (Exp(2, 655357) * (1 + Exp(2, 2))))

        self.assertGreater(
            2 + Exp(2, 3),
            Exp(2, 3))

        self.assertFalse(
            ((2 + Exp(2, 3)) * 2 ** ((-4 + (5 * Exp(2, 11))) // 3))
                < Exp(2, 3))

        self.assertLess(
            Exp(2, 5) + Exp(3, 2),
            Exp(5, 2) + (Exp(2, 5) + Exp(3, 2)))

        self.assertLess(
            Exp(2, 33) * (1 + Exp(2, 2)),
            (1 + Exp(2, 2)) * 2 ** (-3 + (Exp(2, 33) * (1 + Exp(2, 2)))))

        self.assertFalse(
            (10 ** Exp(10, 8274649522))
                < 8274649524 + Exp(10, 8274649522))

        with self.assertRaises(NotImplementedError):
            self.assertLess(
                -(Exp(2, 11760) * (1 + Exp(2, 5879))) + (Exp(2, 20578) * (-1 + (11 * Exp(2, 1469)))),
                (Exp(2, 44097) * (-1 + (11 * Exp(2, 1469)))) + -(Exp(2, 23520) * (1 + (Exp(2, 11759) * (1 + Exp(2, 5879))))))

        self.assertLess(
            (Exp(2, 3) * (2 + (Exp(2, 3) * (2 + (Exp(2, 3) * (2 + (Exp(2, 3) * (2 + Exp(2, 3))))))))),
            (Exp(2, 3) * (2 + (Exp(2, 3) * (2 + (Exp(2, 3) * (2 + (Exp(2, 3) * (2 + (Exp(2, 3) * (2 + Exp(2, 3))))))))))))

        self.assertFalse(
            (3 * 2 ** (5 + (Exp(2, 5) * (1 + 2 ** Exp(2, 5)))))
                < 2 ** Exp(2, 5))

        self.assertFalse(
            (-(Exp(2, 8) * (-1 + 2 ** (-8 + Exp(2, 8)))) + (Exp(2, 8) * (-1 + 2 ** (-8 + 2 ** 2 ** Exp(2, 8)))))
                < ((Exp(2, 8) * (-1 + 2 ** (-8 + 2 ** Exp(2, 8)))) + -(Exp(2, 8) * (-1 + 2 ** (-8 + Exp(2, 8))))))

        self.assertLess(
            Tet(10, 2),
            Tet(10, 3))

        self.assertGreater(
            Tet(10, 3),
            3 + Tet(10, 2))

        with self.assertRaises(NotImplementedError):
            self.assertGreater(
                Tet(10, 2),
                Exp(2, 5))

        with self.assertRaises(NotImplementedError):
            self.assertLess(
                Tet(10, 2),
                Tet(8, 3))

        with self.assertRaises(NotImplementedError):
            self.assertLess(
                Exp(2, 13),
                Exp(2, 10) * (4 + Exp(2, 13)))

        with self.assertRaises(NotImplementedError):
            self.assertLess(
                Exp(2, 13),
                Exp(2, 12) * (1 + Exp(2, 11)))

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
            Exp(2, 3) + (-4 + Exp(2, 3)),
            12,
            "(-4 + (2 ** 4))")

        self.assert_num(
            (Exp(2, 3) * 7) + (Exp(2, 5) * 11),
            408,
            "((2 ** 3) * (7 + (11 * (2 ** 2))))")

        self.assert_num(
            (Exp(2, 3) + Exp(3, 3)) + (-5 + Exp(3, 3)),
            57,
            "(-5 + ((3 ** 3) + ((2 ** 3) + (3 ** 3))))")

        self.assert_num(
            (-12 + 2 ** (-1 + Exp(2, 3))) - (-11 + Exp(2, 3)),
            119,
            "(-1 + ((2 ** 3) * (-1 + (2 ** (-4 + (2 ** 3))))))")

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
            Exp(2, 3) * 2 ** Exp(2, 5),
            34359738368,
            "(2 ** (3 + (2 ** 5)))")

        self.assert_num(
            Exp(2, 3) * 2 ** Exp(2, 4),
            524288,
            "(2 ** (3 + (2 ** 4)))")

        self.assert_num(
            Exp(2, 3) * 2 ** (-3 + Exp(2, 4)),
            65536,
            "(2 ** (2 ** 4))")

        self.assert_num(
            (Exp(2, 3) * 5) + (Exp(2, 3) * 2 ** (-3 + Exp(2, 4))),
            65576,
            "((2 ** 3) * (5 + (2 ** (-3 + (2 ** 4)))))")

        self.assert_num(
            (Exp(2, 3) * 5) + 2 ** Exp(2, 4),
            65576,
            "((2 ** 3) * (5 + (2 ** (-3 + (2 ** 4)))))")

        self.assert_num(
            (5 * Exp(2, 3)) + 2 ** Exp(2, 4),
            65576,
            "((2 ** 3) * (5 + (2 ** (-3 + (2 ** 4)))))")

        self.assert_num(
            Exp(2, 3) * (5 + 2 ** (-3 + Exp(2, 4))),
            65576,
            "((2 ** 3) * (5 + (2 ** (-3 + (2 ** 4)))))")

        self.assert_num(
            Exp(2, 3) * (5 + 2 ** Exp(2, 4)),
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

        self.assert_num(
            Exp(2, 3) * ((-13 * Exp(2, 5)) * (1 + Exp(2, 7))),
            -429312,
            "((-13 * (2 ** 8)) * (1 + (2 ** 7)))")

        self.assert_num(
            (3 * Exp(2, 3)) * Exp(2, 5),
            768,
            "(3 * (2 ** 8))")

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
            (-20 + (2 ** ((-5 + (13 * Exp(2, 2))) // 3) * (3 + (5 * 2 ** ((-2 + ((13 * Exp(2, 2)) + -(-5 + (13 * Exp(2, 2))))) // 3))))) // 3,
            141988,
            "((-20 + (13 * (2 ** ((-5 + (13 * (2 ** 2))) // 3)))) // 3)")

        self.assert_num(
            (-6 + ((1 + Exp(2, 2)) * 2 ** Exp(2, 3))) - (-6 + ((1 + Exp(2, 2)) * Exp(2, 5))),
            1120,
            "(((1 + (2 ** 2)) * (2 ** (2 ** 3))) + -((2 ** 5) * (1 + (2 ** 2))))")

    def test_mul_add(self):
        self.assert_num(
            (Exp(2, 3) * Exp(3, 2)) + (Exp(2, 3) * Exp(5, 3)),
            1072,
            "((2 ** 3) * ((3 ** 2) + (5 ** 3)))")

        self.assert_num(
            (Exp(3, 2) * Exp(2, 3)) + (Exp(5, 3) * Exp(2, 3)),
            1072,
            "((2 ** 3) * ((3 ** 2) + (5 ** 3)))")

        self.assert_num(
            (-3 * Exp(2, 3)) * (-1 + Exp(2, 5)),
            -744,
            "((-3 * (2 ** 3)) * (-1 + (2 ** 5)))")

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
            Exp(3, 7),
            128,
            11)

    def test_add_exp(self):
        self.assert_num(
            (Exp(2, 3) * (1 + 2 ** (5 + Exp(2, 3)))) - Exp(2, 3),
            65536,
            "(2 ** (8 + (2 ** 3)))")

        self.assert_num(
            2 ** (3 * Exp(2, 3)) + (3 * 2 ** (10 + Exp(2, 3))),
            17563648,
            "((2 ** (3 * (2 ** 3))) + (3 * (2 ** (10 + (2 ** 3)))))")

        self.assert_num(
            (5 * 2 ** ((1 + Exp(2, 3)) // 3)) + (-5 * 2 ** 2 ** ((-1 + Exp(2, 4)) // 3)),
            -21474836440,
            "((5 * (2 ** ((1 + (2 ** 3)) // 3))) + (-5 * (2 ** (2 ** ((-1 + (2 ** 4)) // 3)))))")

        self.assert_rep(
            3 ** Exp(3, 5) + (3 ** ((Exp(3, 5) * (-1 + 3 ** Exp(3, 5))) + -(Exp(3, 5) * (-1 + Exp(3, 5)))) * (1 + Exp(3, 5))),
            "((3 ** (3 ** 5)) + ((3 ** (((3 ** 5) * (-1 + (3 ** (3 ** 5)))) + -((3 ** 5) * (-1 + (3 ** 5))))) * (1 + (3 ** (5 + ((((3 ** 5) * (-1 + (3 ** (3 ** 5)))) + -((3 ** 5) * (-1 + (3 ** 5)))) + (((3 ** 5) * (-1 + (3 ** 5))) + -((3 ** 5) * (-1 + (3 ** (3 ** 5)))))))))))")

    def test_tet(self):
        self.assert_rep(
            10 ** Tet(10, 2),
            "(10 ↑↑ 3)")

        self.assert_rep(
            10 ** Tet(11, 2),
            "(10 ** (11 ↑↑ 2))")

    def test_div_div(self):
        self.assert_rep(
            130 * (2 ** ((61 + (13 * Exp(2, 803))) // 15)),
            "(65 * (2 ** ((76 + (13 * (2 ** 803))) // 15)))")

        self.assert_rep(
            Exp(2, 801) * (351 + (65 * (2 ** ((-11954 + (13 * Exp(2, 803))) // 15)))),
            "((2 ** 801) * (351 + (65 * (2 ** ((-11954 + (13 * (2 ** 803))) // 15)))))")

        self.assert_rep(
            Exp(2, 801) * (65 * (2 ** ((-11954 + (13 * Exp(2, 803))) // 15))),
            "(65 * (2 ** ((61 + (13 * (2 ** 803))) // 15)))")

        self.assert_rep(
            (Exp(2, 801) * 351) + (65 * (2 ** ((61 + (13 * Exp(2, 803))) // 15))),
            "((2 ** 801) * (351 + (65 * (2 ** ((-11954 + (13 * (2 ** 803))) // 15)))))")

        self.assert_rep(
            253398 + (130 * (2 ** ((61 + (13 * Exp(2, 803))) // 15))) + (Exp(2, 801) * (351 + (65 * (2 ** ((-11954 + (13 * Exp(2, 803))) // 15))))),
            "(253398 + ((65 * (2 ** ((76 + (13 * (2 ** 803))) // 15))) + ((2 ** 801) * (351 + (65 * (2 ** ((-11954 + (13 * (2 ** 803))) // 15)))))))")

        self.assert_rep(
            (65 * (2 ** ((76 + (13 * Exp(2, 803))) // 15))) + (Exp(2, 801) * (351 + (65 * (2 ** ((-11954 + (13 * Exp(2, 803))) // 15))))),
            "((65 * (2 ** ((76 + (13 * (2 ** 803))) // 15))) + ((2 ** 801) * (351 + (65 * (2 ** ((-11954 + (13 * (2 ** 803))) // 15))))))")

        self.assert_rep(
            (((65 * (2 ** ((76 + (13 * Exp(2, 803))) // 15))) + (Exp(2, 801) * (351 + (65 * (2 ** ((-11954 + (13 * Exp(2, 803))) // 15)))))) // 3),
            "(((65 * (2 ** ((76 + (13 * (2 ** 803))) // 15))) + ((2 ** 801) * (351 + (65 * (2 ** ((-11954 + (13 * (2 ** 803))) // 15)))))) // 3)")

        self.assert_rep(
            84466 + (((65 * (2 ** ((76 + (13 * Exp(2, 803))) // 15))) + (Exp(2, 801) * (351 + (65 * (2 ** ((-11954 + (13 * Exp(2, 803))) // 15)))))) // 3),
            "((253398 + ((65 * (2 ** ((76 + (13 * (2 ** 803))) // 15))) + ((2 ** 801) * (351 + (65 * (2 ** ((-11954 + (13 * (2 ** 803))) // 15))))))) // 3)")


class TestRecursionError(TestCase):
    def test_recursion_error(self):
        num1 = (
            253398
            + (130 * (2 ** ((61 + (13 * Exp(2, 803))) // 15)))
            + (Exp(2, 801) * (351 + (65 * (2 ** ((-11954 + (13 * Exp(2, 803))) // 15)))))
        )

        self.assertEqual(
            str(num1),
            "(253398 + ((65 * (2 ** ((76 + (13 * (2 ** 803))) // 15))) + ((2 ** 801) * (351 + (65 * (2 ** ((-11954 + (13 * (2 ** 803))) // 15)))))))")

        with self.assertRaises(RecursionError):
            # pylint: disable = pointless-statement
            num1 // 15

        num2 = 84466 + (((65 * (2 ** ((76 + (13 * Exp(2, 803))) // 15))) + (Exp(2, 801) * (351 + (65 * (2 ** ((-11954 + (13 * Exp(2, 803))) // 15)))))) // 3)

        self.assertEqual(
            str(num2),
            "((253398 + ((65 * (2 ** ((76 + (13 * (2 ** 803))) // 15))) + ((2 ** 801) * (351 + (65 * (2 ** ((-11954 + (13 * (2 ** 803))) // 15))))))) // 3)")

        with self.assertRaises(RecursionError):
            # pylint: disable = pointless-statement
            num2 // 5

        num3 = (
            (65 * (2 ** ((76 + (13 * Exp(2, 803))) // 15)))
            + (Exp(2, 801) * (351 + (65 * (2 ** ((-11954 + (13 * Exp(2, 803))) // 15)))))
        ) // 3

        self.assertEqual(
            str(num3),
            "(((65 * (2 ** ((76 + (13 * (2 ** 803))) // 15))) + ((2 ** 801) * (351 + (65 * (2 ** ((-11954 + (13 * (2 ** 803))) // 15)))))) // 3)")

        num4 = (
            (351 * Exp(2, 801))
            + (Exp(2, 801) * (65 * (2 ** ((-11954 + (13 * Exp(2, 803))) // 15))))
        )

        self.assertEqual(
            str(num4),
            "((2 ** 801) * (351 + (65 * (2 ** ((-11954 + (13 * (2 ** 803))) // 15)))))")
