# pylint: disable = line-too-long, too-many-lines
# pylint: disable = expression-not-assigned, pointless-statement

from __future__ import annotations

from typing import TYPE_CHECKING
from unittest import TestCase

import tm.num
from tm.num import make_exp as Exp, Tet, show_number

if TYPE_CHECKING:
    from tm.num import Count


CACHES: dict[str, dict[Count, dict[Count, Count]]] = {
    "adds": tm.num.ADDS,  # type: ignore[dict-item]
    "muls": tm.num.MULS,  # type: ignore[dict-item]
    "divs": tm.num.DIVS,  # type: ignore[dict-item]
    "exps": tm.num.EXPS,  # type: ignore[dict-item]
}


def clear_caches() -> None:
    for cache in CACHES.values():
        cache.clear()


def assert_num_counts(expected: dict[str, int]):
    err = None

    num_counts = {
        name: sum(len(vals) for vals in cache.values())
        for name, cache in CACHES.items()
    }

    num_counts['totl'] = sum(num_counts.values())

    try:
        assert num_counts == expected, num_counts
    except AssertionError:  # no-cover
        err = [
            f'            "{cat}": {val},'
            for cat, val in sorted(num_counts.items())
        ]
    finally:
        clear_caches()

    if err:  # no-cover
        raise AssertionError(
            '\n' + '\n'.join(err))


class TestNum(TestCase):
    maxDiff = None

    @classmethod
    def setUpClass(cls):
        clear_caches()

    @classmethod
    def tearDownClass(cls):
        assert_num_counts({
            "adds": 2280,
            "divs": 2073,
            "exps": 1249,
            "muls": 1410,
            "totl": 7012,
        })

    def assert_mod(
            self,
            num: Count,
            mod: int,
            rem: int,
            skip_num: bool = False,
    ):
        self.assertEqual(
            rem,
            num % mod)

        if skip_num:
            return

        self.assertEqual(
            rem,
            int(num) % mod)

    def assert_num(
            self,
            num: Count,
            val: int | str,
            rep: str | None = None,
            mod_rem: tuple[int, int] | None = None,
    ):
        if isinstance(val, str):
            assert not isinstance(num, int)
            self.assertEqual(val, str(num.estimate()))
        else:
            self.assertEqual(val, int(num))

            self.assertEqual(-val, int(-num))

            self.assertEqual(
                val,
                eval(str(num)))  # pylint: disable = eval-used

        if rep is not None:
            self.assertEqual(rep, str(num))

        if mod_rem is not None:
            mod, rem = mod_rem

            self.assert_mod(num, mod, rem)

    def assert_less(
            self,
            val1: Count,
            val2: Count,
            estimate: bool = False,
    ):
        self.assertLess(val1, val2)

        if not estimate:
            self.assertLess(
                int(val1),
                int(val2))

        else:
            assert not isinstance(val1, int)
            assert not isinstance(val2, int)

            self.assertLessEqual(
                val1.estimate(),
                val2.estimate())

    def assert_less_not_implemented(
            self,
            val1: Count,
            val2: Count,
            comps: tuple[Count, Count],
    ):
        with self.assertRaises(NotImplementedError) as ctx:
            self.assertLess(val1, val2)

        self.assertEqual(comps, ctx.exception.args)

        comp1, comp2 = comps

        with self.assertRaises(NotImplementedError):
            self.assertLess(comp1, comp2)

    def assert_depth(self, val: Count, depth: int):
        assert not isinstance(val, int)
        self.assertEqual(depth, val.depth)

    def assert_digits(self, val: Count, digits: int):
        assert not isinstance(val, int)
        self.assertEqual(digits, val.digits())

    def assert_estimate(self, val: Count, estimate: Count | str):
        assert not isinstance(val, int)

        est = val.estimate()

        self.assertEqual(
            estimate,
            str(est) if isinstance(estimate, str) else est,
            est)

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

        self.assert_num(
            ((Exp(2, 3) * (-1 + (2 ** (-3 + (2 ** Exp(2, 3))))))
                 + -(Exp(2, 3) * (-1 + (2 ** (-3 + Exp(2, 3)))))),
            115792089237316195423570985008687907853269984665640564039457584007913129639680,
            "((2 ** (2 ** 3)) * (-1 + (2 ** ((2 ** 3) * (-1 + (2 ** (-3 + (2 ** 3))))))))")

        self.assert_num(
            (-(Exp(2, 3) * (-1 + (2 ** (-3 + Exp(2, 3)))))
                + (Exp(2, 3) * (-1 + (2 ** (-3 + (2 ** Exp(2, 3))))))),
            115792089237316195423570985008687907853269984665640564039457584007913129639680,
            "((2 ** (2 ** 3)) * (-1 + (2 ** ((2 ** 3) * (-1 + (2 ** (-3 + (2 ** 3))))))))")

    def test_digits(self):
        self.assert_digits(
            Exp(2, 2147483647),
            646456993)

        self.assert_digits(
            Exp(2, 11),
            3)

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
        self.assert_num((5 * Exp(2, 3)) - 0, 40)

        self.assert_num(
            (7 * Exp(3, 3)) + -(5 * Exp(3, 3)),
            54,
            "(2 * (3 ** 3))")

        self.assert_num(
            -(5 * Exp(3, 3)) + (7 * Exp(3, 3)),
            54,
            "(2 * (3 ** 3))")

        self.assert_num(
            -(341 * Exp(2, 53)) + (3095 * Exp(2, 53)),
            24805826747556691968,
            "(1377 * (2 ** 54))")

        self.assert_num(
            (7 * Exp(2, 42)) + (-(223 * Exp(2, 42)) + ((7 * Exp(2, 47)) + ((11 * Exp(2, 53)) + -(223 * Exp(2, 48))))),
            36345456367763456,
            "(1033 * (2 ** 45))")

        self.assert_num(
            -(341 * Exp(2, 53)) + ((3095 * Exp(2, 53)) + ((4227601 * Exp(2, 60)) + -(341 * Exp(2, 56)))),
            4874092339984591505850368,
            "(270566477 * (2 ** 54))")

        self.assert_estimate(
            1 + Tet(10, 5),
            Tet(10, 5))

        self.assert_estimate(
            Tet(10, 5) + 1,
            Tet(10, 5))

        self.assert_num(
            (2 ** (-1 + (2 ** Exp(2, 258)))) - (-1 + (2 ** Exp(2, 258))),
            "(10 ↑↑ 4)",
            "(1 + ((2 ** (2 ** 258)) * (-1 + (2 ** (-1 + ((2 ** 258) * (-1 + (2 ** (-258 + (2 ** 258))))))))))")

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
            (3 * Exp(2, 3)) // 4,
            6,
            "6")

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
            "(2186 * (3 ** 13))")

    def test_div_mod(self):
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
            "(171 * (2 ** 2))")

        self.assert_num(
            (19 * Exp(2, 2)) + (19 * Exp(2, 5)),
            684,
            "(171 * (2 ** 2))")

        self.assert_num(
            19 * (Exp(2, 2) * (1 + Exp(2, 3))),
            684,
            "(171 * (2 ** 2))")

        self.assert_num(
            Exp(2, 2) * (19 * (1 + Exp(2, 3))),
            684,
            "(171 * (2 ** 2))")

        self.assert_num(
            Exp(2, 2) * (19 + (19 * Exp(2, 3))),
            684,
            "(171 * (2 ** 2))")

        self.assertEqual(
            "(~10^19728)",
            show_number(
                int(2 ** 2 ** 2 ** Exp(2, 2))))

        self.assert_num(
            2 ** Exp(2, 5),
            4294967296,
            "(2 ** (2 ** 5))")

        self.assert_num(
            Exp(2, 5) * (-1 + (2 ** (-5 + Exp(2, 5)))),
            4294967264,
            "((2 ** 5) * (-1 + (2 ** (-5 + (2 ** 5)))))")

    def test_exp_mod(self):
        vals = {
            (4, 5, 3): 1,
            (5, 3, 13): 8,
            (4, 13, 497): 445,
            (2, 31,  4): 0,
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

        self.assert_mod(
            (-3 + (3 ** ((-3 + Exp(3, 5)) // 8))),
            2,
            0)

        self.assert_mod(
            2 ** Exp(3, 11),
            4,
            0)

        self.assert_mod(
            6 ** ((1 + (69 * Exp(6, 13))) // 5),
            10,
            6,
            skip_num = True)

    def test_exp_mod_special_case(self):
        self.assert_mod(
            Exp(2, 3),
            6,
            2)

        self.assert_mod(
            -4 + Exp(2, 4),
            6,
            0)

        self.assert_mod(
            13 * Exp(2, 3),
            6,
            2)

        self.assert_mod(
            -2 + (13 * Exp(2, 3)),
            6,
            0)

        self.assert_mod(
            (-4 + Exp(2, 4)) // 3,
            2,
            0)

        self.assert_mod(
            (-2 + (13 * Exp(2, 3))) // 3,
            2,
            0)

        self.assert_mod(
            2 ** ((-2 + (13 * Exp(2, 3))) // 3),
            12,
            4)

        self.assert_mod(
            2 ** ((-5 + (13 * Exp(2, 3))) // 3),
            6,
            2)

        self.assert_mod(
            -55 + (61 * Exp(2, 8)),
            162,
            9)

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

        self.assert_mod(
            3 ** ((7 + Exp(3, 2)) // 8),
            6,
            3)

        self.assert_mod(
            Exp(7, 741),
            12,
            7)

        self.assert_mod(
            Exp(2, 37),
            20,
            12)

        self.assert_mod(
            (13 + (7 * Exp(2, 6373))) // 9,
            6,
            3)

    def test_mod_1(self):
        nums = (
            Exp(2, 3),
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
        self.assert_num(1 * (5 * Exp(2, 3)), 40)
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
            "(8193 * (2 ** 19))")

        self.assert_num(
            exp,
            (2 ** 19) + ((2 ** 17) * (2 ** 15)))

        self.assert_num(
            2 ** (-2 + Exp(3, 3)) + 2 ** (-3 + Exp(3, 3)),
            50331648,
            "(3 * (2 ** (-3 + (3 ** 3))))")

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

        self.assert_less(
            4 + Exp(2, 4),
            3 + Exp(2, 5))

        self.assertFalse(
            Exp(2, 3) < -Exp(2, 5))

        self.assert_less(
            Exp(2, 3),
            Exp(2, 3) * (1 + Exp(2, 3)))

        self.assert_less(
            3 + Exp(2, 3),
            4 * Exp(2, 5))

        self.assert_less(
            -1 + (3 * Exp(2, 3)),
            3 * Exp(2, 3))

        self.assert_less(
            3 * Exp(2, 3),
            1 + (3 * Exp(2, 3)))

        self.assert_less(
            (1 + Exp(2, 3)) // 3,
            (1 + Exp(2, 5)) // 3)

        self.assert_less(
            1,
            (1 + Exp(2, 3)) // 3)

        self.assert_less(
            49 + (13 * Exp(2, 15)),
            113 + (13 * Exp(2, 16)))

        self.assertFalse(
            Exp(10, 3 + Exp(10, Exp(10, 3)))
                < Exp(10, 3 + Exp(10, 3)))

        self.assert_less(
            Exp(10, 3 + Exp(10, 3)),
            Exp(10, 3 + Exp(10, Exp(10, 3))),
            estimate = True)

        self.assert_less_not_implemented(
            Exp(2, 13) * (-1 + Exp(2, 13)),
            2 ** (-3 + Exp(2, 13)),
            (
                8191 * Exp(2, 13),
                2 ** (-3 + Exp(2, 13)),
            ))

        self.assert_less(
            -Exp(10, 14050258128),
            Exp(10, 14050259810),
            estimate = True)

        self.assert_less_not_implemented(
            Exp(3, 5) * (-243 + (3 ** Exp(3, 5))),
            3 ** Exp(3, 5),
            (
                Exp(3, 10) * (-1 + (3 ** (-5 + Exp(3, 5)))),
                3 ** Exp(3, 5),
            ))

        self.assert_less(
            Exp(2, 5) * (-1 + (2 ** (-5 + Exp(2, 5)))),
            -1 + (Exp(2, 5) * (-1 + (2 ** (-5 + (2 ** Exp(2, 5)))))),
            estimate = True)

        self.assert_less(
            Exp(2, 5) * (-1 + (2 ** (-5 + Exp(2, 5)))),
            Exp(2, 5) * (-1 + (2 ** (-5 + (2 ** Exp(2, 5))))),
            estimate = True)

        self.assert_less(
            -1 + (2 ** (-5 + Exp(2, 5))),
            -1 + (2 ** (-5 + (2 ** Exp(2, 5)))),
            estimate = True)

        self.assert_less(
            2 ** (-5 + Exp(2, 5)),
            2 ** (-5 + (2 ** Exp(2, 5))),
            estimate = True)

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

        self.assert_less(
            Exp(2, 33) * (1 + Exp(2, 2)),
            (1 + Exp(2, 2)) * 2 ** (-3 + (Exp(2, 33) * (1 + Exp(2, 2)))),
            estimate = True)

        self.assertFalse(
            (10 ** Exp(10, 8274649522))
                < 8274649524 + Exp(10, 8274649522))

        self.assert_less(
            -(Exp(2, 11760) * (1 + Exp(2, 5879))) + (Exp(2, 20578) * (-1 + (11 * Exp(2, 1469)))),
            (Exp(2, 44097) * (-1 + (11 * Exp(2, 1469)))) + -(Exp(2, 23520) * (1 + (Exp(2, 11759) * (1 + Exp(2, 5879))))),
            estimate = True)

        self.assert_less(
            Exp(2, 3),
            Exp(2, 3) * (2 + Exp(2, 3)))

        self.assert_less(
            Exp(2, 3) * (2 + (Exp(2, 3) * (2 + (Exp(2, 3) * (2 + (Exp(2, 3) * (2 + Exp(2, 3)))))))),
            Exp(2, 3) * (2 + (Exp(2, 3) * (2 + (Exp(2, 3) * (2 + (Exp(2, 3) * (2 + (Exp(2, 3) * (2 + Exp(2, 3)))))))))),
            estimate = True)

        self.assertFalse(
            (3 * 2 ** (5 + (Exp(2, 5) * (1 + 2 ** Exp(2, 5)))))
                < 2 ** Exp(2, 5))

        self.assertFalse(
            (-(Exp(2, 8) * (-1 + 2 ** (-8 + Exp(2, 8)))) + (Exp(2, 8) * (-1 + 2 ** (-8 + 2 ** 2 ** Exp(2, 8)))))
                < ((Exp(2, 8) * (-1 + 2 ** (-8 + 2 ** Exp(2, 8)))) + -(Exp(2, 8) * (-1 + 2 ** (-8 + Exp(2, 8))))))

        self.assert_less(
            Tet(10, 2),
            Tet(10, 3),
            estimate = True)

        self.assertGreater(
            Tet(10, 3),
            3 + Tet(10, 2))

        self.assert_less_not_implemented(
            Tet(10, 2),
            Exp(2, 5),
            (
                Tet(10, 2),
                Exp(2, 5),
            ))

        self.assert_less_not_implemented(
            Tet(10, 2),
            Tet(8, 3),
            (
                Tet(10, 2),
                Tet(8, 3),
            ))

        self.assert_less(
            Exp(2, 13),
            Exp(2, 10) * (4 + Exp(2, 13)))

        self.assert_less(
            Exp(2, 13),
            Exp(2, 12) * (1 + Exp(2, 11)))

        self.assert_less_not_implemented(
            Exp(2, 7) * (4 + (Exp(2, 8) * (4 + (Exp(2, 9) * (4 + (Exp(2, 10) * (4 + Exp(2, 13)))))))),
            Exp(2, 6) * (4 + (Exp(2, 7) * (4 + (Exp(2, 8) * (4 + (Exp(2, 9) * (4 + (Exp(2, 10) * (4 + Exp(2, 13)))))))))),
            (
                275012256001 * Exp(2, 9),
                35201568768129 * Exp(2, 8),
            ))

        self.assert_less_not_implemented(
            Exp(2, 10) * (4 + Exp(2, 13)),
            Exp(2, 13),
            (
                2049 * Exp(2, 12),
                Exp(2, 13),
            ))

        self.assert_less(
            2 ** (Exp(2, 19) * (-1 + (2 ** (-17 + Exp(2, 19))))),
            2 ** (2 + (Exp(2, 19) * (-1 + (2 ** (-17 + Exp(2, 19)))))),
            estimate = True)

        self.assertGreater(
            Tet(10, 5),
            5)

        self.assertFalse(
            ((Exp(2, 88) * (-1 + (2 ** ((-256 + Exp(2, 88)) // 3)))) + ((2 ** ((8 + Exp(2, 88)) // 3)) * (-1 + (2 ** ((Exp(2, 88) * (-1 + (2 ** ((-256 + Exp(2, 88)) // 3)))) // 3)))))
                < (Exp(2, 88) * (-1 + (2 ** ((-256 + Exp(2, 88)) // 3)))))

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
            Exp(2, 3) + (-4 + Exp(2, 3)),
            12,
            "(-4 + (2 ** 4))")

        self.assert_num(
            (Exp(2, 3) * 7) + (Exp(2, 5) * 11),
            408,
            "(51 * (2 ** 3))")

        self.assert_num(
            (-12 + 2 ** (-1 + Exp(2, 3))) - (-11 + Exp(2, 3)),
            119,
            "(-1 + ((2 ** 3) * (-1 + (2 ** (-4 + (2 ** 3))))))")

        self.assert_num(
            (2 ** (3 * Exp(2, 3))) + (2 ** (-1 + (3 * Exp(2, 3)))),
            25165824,
            "(3 * (2 ** (-1 + (3 * (2 ** 3)))))")

        self.assert_num(
            ((2 ** (3 * Exp(2, 3))) + (2 ** (1 + (3 * Exp(2, 3))))),
            50331648,
            "(3 * (2 ** (3 * (2 ** 3))))")

        self.assert_num(
            (-1 + Exp(3, 4)) // 2,
            40,
            "((-1 + (3 ** 4)) // 2)")

        self.assert_num(
            (2 * Exp(3, 18)) * ((-1 + Exp(3, 4)) // 2),
            30993639120,
            "(80 * (3 ** 18))")

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
            -Exp(2, 4) * 32,
            -512,
            "-(2 ** 9)")

        self.assert_num(
            Exp(2, 10) * (-Exp(2, 6) * (1 + Exp(2, 4))),
            -1114112,
            "(-17 * (2 ** 16))")

        self.assert_num(
            Exp(2, 4) * (1 + (Exp(2, 3) * (1 + Exp(2, 2)))),
            656,
            "(41 * (2 ** 4))")

        self.assert_num(
            Exp(2, 4) * (1 + (Exp(2, 3) + (Exp(2, 3) * Exp(2, 2)))),
            656,
            "(41 * (2 ** 4))")

        self.assert_num(
            (Exp(2, 4) + ((Exp(2, 4) * Exp(2, 3)) + (Exp(2, 4) * (Exp(2, 3) * Exp(2, 2))))),
            656,
            "(41 * (2 ** 4))")

        self.assert_num(
            (Exp(2, 4) + (Exp(2, 7) + Exp(2, 9))),
            656,
            "(41 * (2 ** 4))")

        self.assert_num(
            Exp(2, 4) + (Exp(2, 4) * (Exp(2, 3) * (1 + Exp(2, 2)))),
            656,
            "(41 * (2 ** 4))")

        self.assert_num(
            Exp(2, 3) * ((-13 * Exp(2, 5)) * (1 + Exp(2, 7))),
            -429312,
            "(-1677 * (2 ** 8))")

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
            "((2 ** 5) * (-5 + ((1 + (2 ** 2)) * (2 ** (-5 + (2 ** 3))))))")

    def test_mul_add(self):
        self.assert_num(
            (-3 * Exp(2, 3)) * (-1 + Exp(2, 5)),
            -744,
            "((2 ** 3) * (3 + (-3 * (2 ** 5))))")

        self.assert_num(
            Exp(2, 17) * (5 + (2 ** (-17 + Exp(2, 19)))),
            "(10 ** 157826)",
            "((2 ** 17) * (5 + (2 ** (-17 + (2 ** 19)))))")

        self.assert_num(
            Exp(2, 17) * (2 ** (-17 + Exp(2, 19))),
            "(10 ** 157826)",
            "(2 ** (2 ** 19))")

        self.assert_num(
            (Exp(2, 17) * 5) + (Exp(2, 17) * (2 ** (-17 + Exp(2, 19)))),
            "(10 ** 157826)",
            "((2 ** 17) * (5 + (2 ** (-17 + (2 ** 19)))))")

        self.assert_num(
            (5 * Exp(2, 17)) + ((2 ** (-17 + Exp(2, 19))) * Exp(2, 17)),
            "(10 ** 157826)",
            "((2 ** 17) * (5 + (2 ** (-17 + (2 ** 19)))))")

        self.assert_num(
            (5 * Exp(2, 17)) + (Exp(2, 17) * (2 ** (-17 + Exp(2, 19)))),
            "(10 ** 157826)",
            "((2 ** 17) * (5 + (2 ** (-17 + (2 ** 19)))))")

        self.assert_num(
            (5 * Exp(2, 17)) + (2 ** Exp(2, 19)),
            "(10 ** 157826)",
            "((2 ** 17) * (5 + (2 ** (-17 + (2 ** 19)))))")

        self.assert_num(
            Exp(3, 10) * (7 + (4 * (3 ** ((-67 + Exp(3, 11)) // 8)))),
            "(10 ** 10567)",
            "((3 ** 10) * (7 + (4 * (3 ** ((-67 + (3 ** 11)) // 8)))))")

        self.assert_num(
            (Exp(3, 10) * 7) + (Exp(3, 10) * (4 * (3 ** ((-67 + Exp(3, 11)) // 8)))),
            "(10 ** 10567)",
            "((3 ** 10) * (7 + (4 * (3 ** ((-67 + (3 ** 11)) // 8)))))")

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
            "(5 * (2 ** 3))")

        self.assert_num(
            -Exp(2, 3) + -Exp(2, 5),
            -40,
            "(-5 * (2 ** 3))")

        self.assert_num(
            -Exp(2, 3) + Exp(2, 5),
            24,
            "(3 * (2 ** 3))")

    def test_div_gcd(self):
        self.assert_num(
            (9 * (2 ** (-5 + Exp(2, 5)))) // 3,
            402653184,
            "(3 * (2 ** (-5 + (2 ** 5))))")

        self.assert_num(
            (52 + (32 * Exp(7, 7))) // 12,
            2196119,
            "((13 + (8 * (7 ** 7))) // 3)")

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
            Exp(2, 6) * (-5 + (5 * Exp(2, 14))),
            5242560,
            "(81915 * (2 ** 6))")

        self.assert_num(
            (Exp(2, 6) * (-5 + (5 * Exp(2, 14)))) // 12,
            436880,
            "(27305 * (2 ** 4))")

        self.assert_num(
            (13164 + ((70837131 * Exp(2, 12)) + ((-118784 + (29 * Exp(2, 12))) // 9))) // 3,
            96716300580,
            "((-308 + (19 * (2 ** 37))) // 27)")

        self.assert_num(
            (384 + (Exp(2, 5) * (1 + (13 * Exp(2, 7))))) // 24,
            2236,
            "(16 + (555 * (2 ** 2)))")

        self.assert_num(
            (2 + (23 * Exp(6, 14))) // 10,
            180237577421,
            "((1 + (69 * (6 ** 13))) // 5)")

        self.assert_num(
            (151 + (13 * Exp(2, 803))) // 15,
            "(10 ** 242)",
            "((151 + (13 * (2 ** 803))) // 15)")

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
            "(-13 * (3 ** 5))")

        self.assert_num(
            -(((1 + Exp(3, 3)) // 2) * Exp(3, 5)),
            -3402,
            "(-14 * (3 ** 5))")

        self.assert_num(
            -(((1 + Exp(3, 3)) * Exp(3, 5)) // 2),
            -3402,
            "(-14 * (3 ** 5))")

        self.assert_num(
            -(((1 * Exp(3, 5)) + (Exp(3, 3) * Exp(3, 5))) // 2),
            -3402,
            "(-14 * (3 ** 5))")

        self.assert_num(
            -((Exp(3, 5) + Exp(3, 8)) // 2),
            -3402,
            "(-14 * (3 ** 5))")

        self.assert_num(
            -(Exp(3, 5) * (1 + Exp(3, 3))),
            -6804,
            "(-28 * (3 ** 5))")

        self.assert_num(
            -Exp(3, 5) * (1 + Exp(3, 3)),
            -6804,
            "(-28 * (3 ** 5))")

        self.assert_num(
            Exp(3, 5) * -(1 + Exp(3, 3)),
            -6804,
            "(-28 * (3 ** 5))")

        self.assert_num(
            Exp(3, 5) * (-1 + -Exp(3, 3)),
            -6804,
            "(-28 * (3 ** 5))")

    def test_cycle_mod(self):
        self.assert_mod(
            Exp(3, 7),
            128,
            11)

    def test_add_exp(self):
        self.assert_num(
            -46 + Exp(2, 17),
            131026,
            "(-46 + (2 ** 17))")

        self.assert_num(
            -(14 + Exp(2, 5)) + Exp(2, 17),
            131026,
            "(-14 + (4095 * (2 ** 5)))")

        self.assert_num(
            -46 + (13 * Exp(2, 17)),
            1703890,
            "(-46 + (13 * (2 ** 17)))")

        self.assert_num(
            -(14 + Exp(2, 5)) + (13 * Exp(2, 17)),
            1703890,
            "(-14 + (53247 * (2 ** 5)))")

        self.assert_num(
            (Exp(2, 3) * (1 + 2 ** (5 + Exp(2, 3)))) - Exp(2, 3),
            65536,
            "(2 ** (8 + (2 ** 3)))")

        self.assert_num(
            2 ** (3 * Exp(2, 3)) + (3 * 2 ** (10 + Exp(2, 3))),
            17563648,
            "((2 ** (3 * (2 ** 3))) + (3 * (2 ** (10 + (2 ** 3)))))")

        self.assert_num(
            (351 + (65 * (2 ** Exp(2, 3)))),
            16991,
            "(351 + (65 * (2 ** (2 ** 3))))")

        self.assert_num(
            (1 + (2 ** (-1 + Exp(2, 31)))) + 0,
            "(10 ** 646456993)",
            "(1 + (2 ** (-1 + (2 ** 31))))")

        self.assert_num(
            (1 + (2 ** (-1 + Exp(2, 31)))) - 0,
            "(10 ** 646456993)",
            "(1 + (2 ** (-1 + (2 ** 31))))")

        self.assert_num(
            (5 * 2 ** ((1 + Exp(2, 3)) // 3)) + (-5 * 2 ** 2 ** ((-1 + Exp(2, 4)) // 3)),
            -21474836440,
            "((5 * (2 ** ((1 + (2 ** 3)) // 3))) + (-5 * (2 ** (2 ** ((-1 + (2 ** 4)) // 3)))))")

        self.assert_num(
            (Exp(3, 5) * (-243 + (3 ** Exp(3, 5)))) + (3 ** Exp(3, 5)),
            "(10 ** 118)",
            "((3 ** 10) * (-1 + (244 * (3 ** (-10 + (3 ** 5))))))")

        self.assert_num(
            3 ** Exp(3, 5) + (3 ** ((Exp(3, 5) * (-1 + 3 ** Exp(3, 5))) + -(Exp(3, 5) * (-1 + Exp(3, 5)))) * (1 + Exp(3, 5))),
            "(10 ↑↑ 3)",
            "((3 ** (3 ** 5)) * (1 + (244 * (3 ** ((3 ** 5) * (-244 + (3 ** (3 ** 5))))))))")

        self.assert_num(
            (-20 + (13 * (2 ** ((-5 + (13 * Exp(2, 553))) // 3)))) + ((Exp(2, 552) * (117 + (117 * (2 ** Exp(2, 553))))) + (117 * (2 ** ((-8 + (13 * Exp(2, 553))) // 3)))),
            "(10 ↑↑ 3)",
            "(-20 + (((2 ** 552) * (117 + (117 * (2 ** (2 ** 553))))) + (143 * (2 ** ((-8 + (13 * (2 ** 553))) // 3)))))")

        self.assert_num(
            (2 ** ((2 ** Exp(2, 65536)) * (-1 + (2 ** (Exp(2, 65536) * (-1 + (2 ** (-65536 + (2 ** Exp(2, 65536)))))))))) + (2 ** (-1 + ((2 ** Exp(2, 65536)) * (-1 + (2 ** (Exp(2, 65536) * (-1 + (2 ** (-65536 + (2 ** Exp(2, 65536))))))))))),
            "(10 ↑↑ 6)",
            "((2 ** ((2 ** (2 ** 65536)) * (-1 + (2 ** ((2 ** 65536) * (-1 + (2 ** (-65536 + (2 ** (2 ** 65536)))))))))) + (2 ** (-1 + ((2 ** (2 ** 65536)) * (-1 + (2 ** ((2 ** 65536) * (-1 + (2 ** (-65536 + (2 ** (2 ** 65536))))))))))))")

    def test_tet(self):
        self.assert_num(
            10 ** Tet(10, 2),
            "(10 ↑↑ 3)",
            "(10 ↑↑ 3)")

        self.assert_num(
            10 ** Tet(11, 2),
            "(10 ↑↑ 3)",
            "(10 ** (11 ↑↑ 2))")

    def test_div_div(self):
        self.assert_num(
            130 * (2 ** ((61 + (13 * Exp(2, 803))) // 15)),
            "(10 ↑↑ 3)",
            "(65 * (2 ** ((76 + (13 * (2 ** 803))) // 15)))")

        self.assert_num(
            Exp(2, 801) * (351 + (65 * (2 ** ((-11954 + (13 * Exp(2, 803))) // 15)))),
            "(10 ↑↑ 3)",
            "((2 ** 801) * (351 + (65 * (2 ** ((-11954 + (13 * (2 ** 803))) // 15)))))")

        self.assert_num(
            Exp(2, 801) * (65 * (2 ** ((-11954 + (13 * Exp(2, 803))) // 15))),
            "(10 ↑↑ 3)",
            "(65 * (2 ** ((61 + (13 * (2 ** 803))) // 15)))")

        self.assert_num(
            (Exp(2, 801) * 351) + (65 * (2 ** ((61 + (13 * Exp(2, 803))) // 15))),
            "(10 ↑↑ 3)",
            "((2 ** 801) * (351 + (65 * (2 ** ((-11954 + (13 * (2 ** 803))) // 15)))))")

        self.assert_num(
            253398 + (130 * (2 ** ((61 + (13 * Exp(2, 803))) // 15))) + (Exp(2, 801) * (351 + (65 * (2 ** ((-11954 + (13 * Exp(2, 803))) // 15))))),
            "(10 ↑↑ 3)",
            "(253398 + ((2 ** 801) * (351 + (195 * (2 ** ((-11954 + (13 * (2 ** 803))) // 15))))))")

        self.assert_num(
            (65 * (2 ** ((76 + (13 * Exp(2, 803))) // 15))) + (Exp(2, 801) * (351 + (65 * (2 ** ((-11954 + (13 * Exp(2, 803))) // 15))))),
            "(10 ↑↑ 3)",
            "((2 ** 801) * (351 + (195 * (2 ** ((-11954 + (13 * (2 ** 803))) // 15)))))")

        self.assert_num(
            (((65 * (2 ** ((76 + (13 * Exp(2, 803))) // 15))) + (Exp(2, 801) * (351 + (65 * (2 ** ((-11954 + (13 * Exp(2, 803))) // 15)))))) // 3),
            "(10 ↑↑ 3)",
            "((2 ** 801) * (117 + (65 * (2 ** ((-11954 + (13 * (2 ** 803))) // 15)))))")

        self.assert_num(
            84466 + (((65 * (2 ** ((76 + (13 * Exp(2, 803))) // 15))) + (Exp(2, 801) * (351 + (65 * (2 ** ((-11954 + (13 * Exp(2, 803))) // 15)))))) // 3),
            "(10 ↑↑ 3)",
            "(84466 + ((2 ** 801) * (117 + (65 * (2 ** ((-11954 + (13 * (2 ** 803))) // 15))))))")

    def test_exp_div(self):
        self.assert_num(
            Exp(2, 10) // Exp(2, 9),
            2,
            "2")

        self.assert_num(
            Exp(2, 13) // Exp(2, 10),
            8,
            "(2 ** 3)")

        self.assert_num(
            (2 ** Exp(2, 7)) // Exp(2, 10),
            332306998946228968225951765070086144,
            "(2 ** (-10 + (2 ** 7)))")

        self.assert_num(
            (2 ** Exp(2, 7)) // (2 ** Exp(2, 5)),
            79228162514264337593543950336,
            "(2 ** (3 * (2 ** 5)))")

    def test_mul_pow(self):
        self.assert_num(
            (2 ** (-259 + Exp(2, 258))) * ((2 ** (Exp(2, 258) * (-1 + (2 ** (-258 + Exp(2, 258)))))) + (2 ** (2 + (Exp(2, 258) * (-1 + (2 ** (-258 + Exp(2, 258)))))))),
            "(10 ↑↑ 4)",
            "(5 * (2 ** (-259 + (2 ** (2 ** 258)))))")

    def test_recursion_error(self):
        num1 = (
            253398
            + (130 * (2 ** ((61 + (13 * Exp(2, 803))) // 15)))
            + (Exp(2, 801) * (351 + (65 * (2 ** ((-11954 + (13 * Exp(2, 803))) // 15)))))
        )

        self.assert_num(
            num1,
            "(10 ↑↑ 3)",
            "(253398 + ((2 ** 801) * (351 + (195 * (2 ** ((-11954 + (13 * (2 ** 803))) // 15))))))")

        num1 // 15

        num2 = 84466 + (((65 * (2 ** ((76 + (13 * Exp(2, 803))) // 15))) + (Exp(2, 801) * (351 + (65 * (2 ** ((-11954 + (13 * Exp(2, 803))) // 15)))))) // 3)

        self.assert_num(
            num2,
            "(10 ↑↑ 3)",
            "(84466 + ((2 ** 801) * (117 + (65 * (2 ** ((-11954 + (13 * (2 ** 803))) // 15))))))")

        num2 // 5

        num3 = (
            (65 * (2 ** ((76 + (13 * Exp(2, 803))) // 15)))
            + (Exp(2, 801) * (351 + (65 * (2 ** ((-11954 + (13 * Exp(2, 803))) // 15)))))
        ) // 3

        self.assert_num(
            num3,
            "(10 ↑↑ 3)",
            "((2 ** 801) * (117 + (65 * (2 ** ((-11954 + (13 * (2 ** 803))) // 15)))))")

        num4 = (
            (351 * Exp(2, 801))
            + (Exp(2, 801) * (65 * (2 ** ((-11954 + (13 * Exp(2, 803))) // 15))))
        )

        self.assert_num(
            num4,
            "(10 ↑↑ 3)",
            "((2 ** 801) * (351 + (65 * (2 ** ((-11954 + (13 * (2 ** 803))) // 15)))))")
