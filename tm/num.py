# pylint: disable = useless-parent-delegation, confusing-consecutive-elif
# pylint: disable = too-many-lines, too-many-return-statements

from __future__ import annotations

import operator
from abc import abstractmethod
from math import sqrt, floor, ceil, log, log10, gcd as pgcd
from typing import TYPE_CHECKING
from functools import cached_property

if TYPE_CHECKING:
    from collections.abc import Callable


class NumException(Exception):
    pass


class Num:
    l: Count
    r: Count

    join: str
    op: Callable[[int, int], int]

    def __repr__(self) -> str:
        # pylint: disable = line-too-long

        # return f'{type(self).__name__}({self.l}, {self.r})'

        return f'({show_number(self.l)} {self.join} {show_number(self.r)})'

    def __int__(self) -> int:
        return self.op(
            int(self.l),
            int(self.r),
        )

    @cached_property
    def depth(self) -> int:
        return 1 + max(self.l_depth, self.r_depth)

    @property
    def l_depth(self) -> int:
        return 0 if isinstance(self.l, int) else self.l.depth

    @property
    def r_depth(self) -> int:
        return 0 if isinstance(self.r, int) else self.r.depth

    @abstractmethod
    def estimate(self) -> int: ...

    def estimate_l(self) -> float:
        return (
            l.estimate()
            if isinstance(l := self.l, Num) else
            0
            if l < 1 else
            log10(l)
        )

    def estimate_r(self) -> float:
        return (
            log10(r)
            if isinstance(r := self.r, int) else
            r.estimate()
        )

    def __neg__(self) -> Count:
        return -1 * self

    def __eq__(self, other: object) -> bool:
        return (
            isinstance(other, type(self))
            and self.l == other.l
            and self.r == other.r
        )

    def __lt__(self, other: Count) -> bool:
        if isinstance(other, int):
            return False

        if isinstance(other, Add):
            if self == other.r:
                return other.l > 0

            if self == other.l:
                return other.r > 0

            if isinstance(other.l, int) and abs(other.l) < 10:
                return self < other.r

        raise NotImplementedError

    def __le__(self, other: Count) -> bool:
        return self == other or self < other

    def __gt__(self, other: Count) -> bool:
        return self != other and not self < other

    def __ge__(self, other: Count) -> bool:
        return self == other or self > other

    def __add__(self, other: Count) -> Count:
        if isinstance(other, int):
            return self if other == 0 else Add(other, self)

        if isinstance(other, Add):
            if isinstance(other.l, int):
                return other.l + (self + other.r)

        if isinstance(other, Mul):
            if other.l == -1 and self == other.r:
                return 0

        if isinstance(other, Div):
            return ((other.den * self) + other.num) // other.den

        return Add(self, other)

    def __radd__(self, other: Count) -> Count:
        assert isinstance(other, int)

        return self + other

    def __sub__(self, other: Count) -> Count:
        return self + -other

    def __rsub__(self, other: Count) -> Count:
        return other + -self

    def __mul__(self, other: Count) -> Count:
        if isinstance(other, int):
            return other * self

        if isinstance(other, Div):
            return (self * other.num) // other.den

        return Mul(self, other)

    def __rmul__(self, other: int) -> Count:
        if other == 0:
            return 0

        if other == 1:
            return self

        return Mul(other, self)

    @abstractmethod
    def __mod__(self, other: int) -> int: ...

    def __divmod__(self, other: int) -> tuple[Count, int]:
        mod = self % other

        return (self - mod) // other, mod

    def __floordiv__(self, other: int) -> Count:
        if other == 1:
            return self

        return Div(self, other)

    def __pow__(self, other: Count) -> Exp:
        return Exp(self, other)

    def __rpow__(self, other: Count) -> Exp:
        return Exp(other, self)


class Add(Num):
    join = '+'

    op = operator.add

    def __init__(self, l: Count, r: Num):
        if isinstance(l, Num) and l.depth > r.depth:
            l, r = r, l

        self.l = l
        self.r = r

    def estimate(self) -> int:
        return round(max(self.estimate_l(), self.estimate_r()))

    def __mod__(self, other: int) -> int:
        return ((self.l % other) + (self.r % other)) % other

    def __neg__(self) -> Count:
        return -(self.l) + -(self.r)

    def __add__(self, other: Count) -> Count:
        if isinstance(l := self.l, int):
            return (
                (l + other) + self.r
                if isinstance(other, int) else
                l + (self.r + other)
            )

        return super().__add__(other)

    def __sub__(self, other: Count) -> Count:
        return (
            self.l - other.l
            if isinstance(other, Add) and self.r == other.r else
            self + -other
        )

    def __mul__(self, other: Count) -> Count:
        return (
            other * self
            if isinstance(other, int) else
            super().__mul__(other)
        )

    def __rmul__(self, other: int) -> Count:
        if other == -1:
            return super().__rmul__(other)

        return (other * self.l) + (other * self.r)

    def __floordiv__(self, other: int) -> Count:
        lgcd = gcd(other, l := self.l)
        rgcd = gcd(other, r := self.r)

        if lgcd != rgcd or lgcd == 1:
            return super().__floordiv__(other)

        return ((l // lgcd) + (r // lgcd)) // (other // lgcd)

    def __lt__(self, other: Count) -> bool:
        if other == self.l:
            return self.r < 0

        if other == self.r:
            return self.l < 0

        if isinstance(other, Add):
            if self == other.r:
                return other.l > 0

            if self.l == other.l:
                return self.r < other.r

            if self.r == other.r:
                return self.l < other.l

            if (isinstance(self.l, int)
                    and isinstance(other.l, int)
                    and abs(self.l - other.l) < 10):
                return self.r < other.r

            if self.l == other.r:  # pragma: no branch
                return self.r < other.l

        if isinstance(self.l, int) and abs(self.l) < 10:
            return self.r < other

        return super().__lt__(other)


class Mul(Num):
    join = '*'

    op = operator.mul

    def __init__(self, l: Count, r: Num):
        if isinstance(l, Num) and l.depth > r.depth:
            l, r = r, l

        self.l = l
        self.r = r

    def __repr__(self) -> str:
        if self.l == -1:
            return f'-{self.r}'

        return super().__repr__()

    def estimate(self) -> int:
        return round(self.estimate_l() + self.estimate_r())

    def __neg__(self) -> Count:
        return -(self.l) * self.r

    def __mod__(self, other: int) -> int:
        if (l_mod := self.l % other) == 0:
            return 0

        if (r_mod := self.r % other) == 0:
            return 0

        return (l_mod * r_mod) % other

    def __mul__(self, other: Count) -> Count:
        if self.l == -1:
            return -1 * (self.r * other)

        return super().__mul__(other)

    def __rmul__(self, other: int) -> Count:
        if other == -1:
            return super().__rmul__(other)

        return (other * self.l) * self.r

    def __add__(self, other: Count) -> Count:
        if isinstance(other, Mul):
            if self.r == other.r:
                return (self.l + other.l) * self.r

            if self.l == other.l:
                return self.l * (self.r + other.r)

            if (isinstance(s_exp := self.r, Exp)
                    and isinstance(o_exp := other.r, Exp)
                    and s_exp.base == o_exp.base):
                try:
                    return _add_exponents(
                        (s_exp, self.l),
                        (o_exp, other.l),
                    )
                except NotImplementedError:
                    pass

        elif isinstance(other, Add):
            if isinstance(other.l, int):  # pragma: no branch
                return other.l + (other.r + self)

            if isinstance(other.l, Mul):
                if other.l.l == self.l:  # pragma: no branch
                    return (self + other.l) + other.r

        elif isinstance(other, Exp):
            return other + self

        if self.l == -1 and other == self.r:
            return 0

        return super().__add__(other)

    def __floordiv__(self, other: int) -> Count:
        l, r = self.l, self.r

        if l % other == 0:
            return (l // other) * r

        if r % other == 0:
            return l * (r // other)

        if isinstance(l, int) and other % l == 0:
            return r // (other // l)

        if isinstance(r, int) and other % r == 0:  # no-coverage
            return l // (other // r)

        return super().__floordiv__(other)  # no-coverage

    def __lt__(self, other: Count) -> bool:
        if self.l < 0:
            raise NotImplementedError

        if isinstance(other, Mul):
            if self.l == other.l:
                return self.r < other.r

            if self.r == other.r:  # pragma: no branch
                return self.l < other.l

        return super().__lt__(other)


class Div(Num):
    join = '//'

    op = operator.floordiv

    @property
    def num(self) -> Count:
        return self.l

    @property
    def den(self) -> int:
        assert isinstance(r := self.r, int)
        return r

    def __init__(self, l: Num, r: int):
        assert r > 0

        self.l = l
        self.r = r

    def __mod__(self, other: int) -> int:
        assert isinstance(den := self.den, int)

        try:  # pylint: disable = too-many-try-statements
            rem, div = divmod(
                self.num % (other * den),
                den)

            assert div == 0

            return rem % other
        except (NumException, AssertionError):
            pass

        try:
            inv = pow(den, -1, other)
        except ValueError as exc:
            raise NumException from exc

        return ((self.num % other) * (inv % other)) % other

    def estimate(self) -> int:
        return round(self.estimate_l() - self.estimate_r())

    def __add__(self, other: Count) -> Count:
        return (self.num + (other * self.den)) // self.den

    def __radd__(self, other: Count) -> Count:
        assert isinstance(other, int)

        return ((other * self.den) + self.num) // self.den

    def __rmul__(self, other: int) -> Count:
        if other == -1:
            return super().__rmul__(other)

        return (other * self.num) // self.den

    def __floordiv__(self, other: int) -> Count:
        assert isinstance(num := self.num, Num)
        return num // (other * self.den)

    def __lt__(self, other: Count) -> bool:
        if isinstance(other, Div):
            if self.den == other.den:  # pragma: no branch
                return self.num < other.num

        return super().__lt__(other)


class Exp(Num):
    join = '**'

    op = operator.pow

    @property
    def base(self) -> Count:
        return self.l

    @property
    def exp(self) -> Count:
        return self.r

    def __init__(self, l: Count, r: Count):
        while isinstance(l, int) and l > 1:  # pylint: disable = while-used
            if l == 8:
                l = 2
                r *= 3
                break

            if floor(root := sqrt(l)) != ceil(root):
                break

            r *= int(log(l, root))
            l = int(root)

        self.l = l
        self.r = r

    def estimate(self) -> int:
        return round(self.estimate_l() * 10 ** self.estimate_r())

    def __mod__(self, other: int) -> int:
        if other == 1 or other == self.base:
            return 0

        res = 1

        base, exp = self.base, self.exp

        if not isinstance(exp, int):
            if isinstance(base, int):
                return exp_mod_special_cases(other, base, exp)
            raise NumException

        while exp > 0 and res > 0:  # pylint: disable = while-used
            if (exp % 2) == 1:
                res = (res * base) % other
            exp //= 2
            base = (base ** 2) % other

        return res

    def __add__(self, other: Count) -> Count:
        if isinstance(other, Mul):
            base = self.base

            if isinstance(exp := other.r, Exp) and exp.base == base:
                try:
                    return _add_exponents((self, 1), (exp, other.l))
                except NotImplementedError:
                    pass

            if isinstance(exp := other.l, Exp) and exp.base == base:
                try:
                    return _add_exponents((self, 1), (exp, other.r))
                except NotImplementedError:
                    pass

        elif isinstance(other, Exp):
            if other.base == self.base:
                try:
                    return _add_exponents((self, 1), (other, 1))
                except NotImplementedError:
                    pass

        return super().__add__(other)

    def __mul__(self, other: Count) -> Count:
        if isinstance(other, Exp):
            if (base := self.base) == other.base:
                return Exp(base, self.exp + other.exp)

        elif isinstance(other, Add):
            if (not isinstance(other.l, Exp)
                    or not isinstance(other.r, Exp)):
                return (self * other.l) + (self * other.r)

        elif isinstance(other, Mul):
            if isinstance(other.l, int):
                return other.l * (self * other.r)

            if (isinstance(o_exp := other.l, Exp)
                    and o_exp.base == self.base):
                return (self * o_exp) * other.r

        return super().__mul__(other)

    def __rmul__(self, other: int) -> Count:
        if other == -1 or isinstance(base := self.base, Num):
            return super().__rmul__(other)

        if other < -1 and -other % base == 0:
            return -(-other * self)

        if other < 1 or other % base != 0:
            return super().__rmul__(other)

        exp = self.exp

        while other % base == 0:  # pylint: disable = while-used
            other //= base
            exp += 1

        return other * Exp(base, exp)

    def __floordiv__(self, other: int) -> Count:
        base, exp = self.base, self.exp

        if not isinstance(base, int):  # no-coverage
            return super().__floordiv__(other)

        while other % base == 0:  # pylint: disable = while-used
            other //= base
            exp -= 1

        return (
            base
            if exp == 1 else
            Exp(base, exp)
        )

    def __lt__(self, other: Count) -> bool:
        if isinstance(other, Exp):
            if self.base == other.base:
                return self.exp < other.exp

            if self.base < other.base and self.exp < other.exp:
                return True

            if self.base > other.base and self.exp > other.exp:
                return False

        return super().__lt__(other)

    def __pow__(self, other: Count) -> Exp:
        return Exp(self.base, self.exp * other)


def _add_exponents(
        l: tuple[Exp, Count],
        r: tuple[Exp, Count],
) -> Count:
    (l_exp, l_co), (r_exp, r_co) = l, r

    assert (base := l_exp.base) == r_exp.base

    if (l_pow := l_exp.exp) > (r_pow := r_exp.exp):
        return _add_exponents((r_exp, r_co), (l_exp, l_co))

    assert l_pow <= r_pow

    diff = r_pow - l_pow

    diff_exp = (
        base ** diff
        if not isinstance(base, int) or diff < 2 else
        Exp(base, diff)
    )

    return (l_co + (r_co * diff_exp)) * Exp(base, l_pow)


def gcd(den: int, num: Count) -> int:
    if isinstance(num, int):
        return pgcd(den, num)

    if isinstance(num, Add):
        return min(gcd(den, num.l), gcd(den, num.r))

    if isinstance(num, Mul):
        return max(gcd(den, num.l), gcd(den, num.r))

    if isinstance(num, Div):  # no-coverage
        return den

    if (isinstance(num, Exp)
            and isinstance(base := num.base, int)):  # pragma: no branch
        val, exp = 1, num.exp

        while den % base == 0:  # pylint: disable = while-used
            val *= base
            den //= base
            exp -= 1

        return val

    return 1  # no-coverage


def exp_mod_special_cases(mod: int, base: int, exp: Num) -> int:
    if base == 2:
        if mod == 3:
            return 1 if exp % 2 == 0 else 2

        if mod == 4:
            return 0

        if mod == 6:
            return 4 if exp % 2 == 0 else 2

        if mod == 12:
            return 4 if exp % 2 == 0 else 8

        if mod == 9:
            return {
                0: 1,
                1: 2,
                2: 4,
                3: 8,
                4: 7,
                5: 5,
            }[exp % 6]

        if mod == 18:
            return {
                0: 10,
                1: 2,
                2: 4,
                3: 8,
                4: 16,
                5: 14,
            }[exp % 6]

        if mod == 54:
            return {
                0: 28,
                1: 2,
                2: 4,
                3: 8,
                4: 16,
                5: 32,
                6: 10,
                7: 20,
                8: 40,
                9: 26,
                10: 52,
                11: 50,
                12: 46,
                13: 38,
                14: 22,
                15: 44,
                16: 34,
                17: 14,
            }[exp % 18]

        if mod == 162:
            return {
                0: 82,
                1: 2,
                2: 4,
                3: 8,
                4: 16,
                5: 32,
                6: 64,
                7: 128,
                8: 94,
                9: 26,
                10: 52,
                11: 104,
                12: 46,
                13: 92,
                14: 22,
                15: 44,
                16: 88,
                17: 14,
                18: 28,
                19: 56,
                20: 112,
                21: 62,
                22: 124,
                23: 86,
                24: 10,
                25: 20,
                26: 40,
                27: 80,
                28: 160,
                29: 158,
                30: 154,
                31: 146,
                32: 130,
                33: 98,
                34: 34,
                35: 68,
                36: 136,
                37: 110,
                38: 58,
                39: 116,
                40: 70,
                41: 140,
                42: 118,
                43: 74,
                44: 148,
                45: 134,
                46: 106,
                47: 50,
                48: 100,
                49: 38,
                50: 76,
                51: 152,
                52: 142,
                53: 122,
            }[exp % 54]

    if base == 3:
        if mod == 6:
            return 3

        if mod == 8:
            return 1 if exp % 2 == 0 else 3

        if mod == 16:
            return {
                0: 1,
                1: 3,
                2: 9,
                3: 11,
            }[exp % 4]

        if mod == 32:
            return {
                0: 1,
                1: 3,
                2: 9,
                3: 27,
                4: 17,
                5: 19,
                6: 25,
                7: 11,
            }[exp % 8]

        if mod == 64:
            return {
                0: 1,
                1: 3,
                2: 9,
                3: 27,
                4: 17,
                5: 51,
                6: 25,
                7: 11,
                8: 33,
                9: 35,
                10: 41,
                11: 59,
                12: 49,
                13: 19,
                14: 57,
                15: 43,
            }[exp % 16]

        if mod == 128:
            return {
                0: 1,
                1: 3,
                2: 9,
                3: 27,
                4: 81,
                5: 115,
                6: 89,
                7: 11,
                8: 33,
                9: 99,
                10: 41,
                11: 123,
                12: 113,
                13: 83,
                14: 121,
                15: 107,
                16: 65,
                17: 67,
                18: 73,
                19: 91,
                20: 17,
                21: 51,
                22: 25,
                23: 75,
                24: 97,
                25: 35,
                26: 105,
                27: 59,
                28: 49,
                29: 19,
                30: 57,
                31: 43,
            }[exp % 32]

        if mod == 256:
            return {
                0: 1,
                1: 3,
                2: 9,
                3: 27,
                4: 81,
                5: 243,
                6: 217,
                7: 139,
                8: 161,
                9: 227,
                10: 169,
                11: 251,
                12: 241,
                13: 211,
                14: 121,
                15: 107,
                16: 65,
                17: 195,
                18: 73,
                19: 219,
                20: 145,
                21: 179,
                22: 25,
                23: 75,
                24: 225,
                25: 163,
                26: 233,
                27: 187,
                28: 49,
                29: 147,
                30: 185,
                31: 43,
                32: 129,
                33: 131,
                34: 137,
                35: 155,
                36: 209,
                37: 115,
                38: 89,
                39: 11,
                40: 33,
                41: 99,
                42: 41,
                43: 123,
                44: 113,
                45: 83,
                46: 249,
                47: 235,
                48: 193,
                49: 67,
                50: 201,
                51: 91,
                52: 17,
                53: 51,
                54: 153,
                55: 203,
                56: 97,
                57: 35,
                58: 105,
                59: 59,
                60: 177,
                61: 19,
                62: 57,
                63: 171,
            }[exp % 64]

        return {
            0: 1,
            1: 3,
            2: 9,
            3: 27,
            4: 81,
            5: 243,
            6: 217,
            7: 139,
            8: 417,
            9: 227,
            10: 169,
            11: 507,
            12: 497,
            13: 467,
            14: 377,
            15: 107,
            16: 321,
            17: 451,
            18: 329,
            19: 475,
            20: 401,
            21: 179,
            22: 25,
            23: 75,
            24: 225,
            25: 163,
            26: 489,
            27: 443,
            28: 305,
            29: 403,
            30: 185,
            31: 43,
            32: 129,
            33: 387,
            34: 137,
            35: 411,
            36: 209,
            37: 115,
            38: 345,
            39: 11,
            40: 33,
            41: 99,
            42: 297,
            43: 379,
            44: 113,
            45: 339,
            46: 505,
            47: 491,
            48: 449,
            49: 323,
            50: 457,
            51: 347,
            52: 17,
            53: 51,
            54: 153,
            55: 459,
            56: 353,
            57: 35,
            58: 105,
            59: 315,
            60: 433,
            61: 275,
            62: 313,
            63: 427,
            64: 257,
            65: 259,
            66: 265,
            67: 283,
            68: 337,
            69: 499,
            70: 473,
            71: 395,
            72: 161,
            73: 483,
            74: 425,
            75: 251,
            76: 241,
            77: 211,
            78: 121,
            79: 363,
            80: 65,
            81: 195,
            82: 73,
            83: 219,
            84: 145,
            85: 435,
            86: 281,
            87: 331,
            88: 481,
            89: 419,
            90: 233,
            91: 187,
            92: 49,
            93: 147,
            94: 441,
            95: 299,
            96: 385,
            97: 131,
            98: 393,
            99: 155,
            100: 465,
            101: 371,
            102: 89,
            103: 267,
            104: 289,
            105: 355,
            106: 41,
            107: 123,
            108: 369,
            109: 83,
            110: 249,
            111: 235,
            112: 193,
            113: 67,
            114: 201,
            115: 91,
            116: 273,
            117: 307,
            118: 409,
            119: 203,
            120: 97,
            121: 291,
            122: 361,
            123: 59,
            124: 177,
            125: 19,
            126: 57,
            127: 171,
        }[exp % 128]

    raise NumException

########################################

Count = int | Num


TRUNCATE_COUNT = 10 ** 12

MAX_DEPTH = 120

def show_number(num: Count) -> str:
    if isinstance(num, int):
        if num >= TRUNCATE_COUNT:
            return f"(~10^{log10(num):.0f})"

    elif num.depth > MAX_DEPTH:  # no-coverage
        return "(???)"

    return str(num)
