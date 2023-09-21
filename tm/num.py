# pylint: disable = useless-parent-delegation, confusing-consecutive-elif
# pylint: disable = too-many-lines, too-many-return-statements

from __future__ import annotations

import itertools
from abc import abstractmethod
from math import sqrt, floor, ceil, log, log2, log10, gcd as pgcd
from functools import cache, cached_property


class NumException(Exception):
    pass


PROFILE = False

NUM_COUNTS = {
    "adds": 0,
    "muls": 0,
    "divs": 0,
    "exps": 0,
}


class Num:
    join: str

    @property
    @abstractmethod
    def left(self) -> Count: ...

    @property
    @abstractmethod
    def right(self) -> Count: ...

    def __repr__(self) -> str:
        return '({} {} {})'.format(
            show_number(self.left),
            self.join,
            show_number(self.right),
        )

    @abstractmethod
    def __int__(self) -> int: ...

    @cached_property
    def depth(self) -> int:
        return 1 + max(
            0 if isinstance(l := self.left, int) else l.depth,
            0 if isinstance(r := self.right, int) else r.depth,
        )

    @abstractmethod
    def estimate(self) -> int: ...

    def estimate_l(self) -> float:
        return (
            l.estimate()
            if isinstance(l := self.left, Num) else
            0
            if l < 1 else
            log10(l)
        )

    def estimate_r(self) -> float:
        return (
            log10(r)
            if isinstance(r := self.right, int) else
            r.estimate()
        )

    def __neg__(self) -> Count:
        return -1 * self

    @abstractmethod
    def __eq__(self, other: object) -> bool: ...

    def __lt__(self, other: Count) -> bool:
        if isinstance(other, int):
            return False

        if isinstance(other, Add):
            l, r = other.l, other.r

            if self == r:
                return l > 0

            if self == l:
                return r > 0

            if isinstance(l, int) and abs(l) < 10:
                return self < r

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

    def __radd__(self, other: int) -> Count:
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
        assert other > 1

        return Div(self, other)

    def __rpow__(self, other: int) -> Exp:
        return Exp(other, self)


class Add(Num):
    join = '+'

    l: Count
    r: Num

    def __init__(self, l: Count, r: Num):
        if PROFILE:
            NUM_COUNTS["adds"] += 1

        if isinstance(l, Num) and l.depth > r.depth:
            l, r = r, l

        self.l = l
        self.r = r

    @property
    def left(self) -> Count:
        return self.l

    @property
    def right(self) -> Num:
        return self.r

    def __int__(self) -> int:
        return int(self.l) + int(self.r)

    def estimate(self) -> int:
        return round(max(self.estimate_l(), self.estimate_r()))

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, Add):
            return False

        return (
            (self.l == other.l and self.r == other.r)
            or (self.l == other.r and self.r == other.l)
        )

    def __mod__(self, other: int) -> int:
        return ((self.l % other) + (self.r % other)) % other

    def __neg__(self) -> Count:
        return -(self.l) + -(self.r)

    def __add__(self, other: Count) -> Count:
        l, r = self.l, self.r

        if isinstance(other, int):
            if isinstance(l, int):
                return (l + other) + r

            return super().__add__(other)

        if isinstance(l, int):
            return l + (other + r)

        return super().__add__(other)

    def __sub__(self, other: Count) -> Count:
        if isinstance(other, Add):
            if self.r == other.r:
                return self.l - other.l

            if self.l == other.l:
                return self.r - other.r

        return self + -other

    def __mul__(self, other: Count) -> Count:
        if isinstance(other, int):
            return other * self

        return super().__mul__(other)

    def __rmul__(self, other: int) -> Count:
        return (other * self.l) + (other * self.r)

    def __floordiv__(self, other: int) -> Count:
        if other == 1:
            return self

        div = gcd(
            gcd(other, l := self.l),
            gcd(other, r := self.r))

        if div == 1:
            return super().__floordiv__(other)

        return ((l // div) + (r // div)) // (other // div)

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

            if self.l == other.r:  # no-branch
                return self.r < other.l

            if (isinstance(self.l, int)  # no-branch
                    and isinstance(other.l, int)):
                return self.r < other.r

        if isinstance(self.l, int) and abs(self.l) < 10:
            return self.r < other

        return super().__lt__(other)


class Mul(Num):
    join = '*'

    l: Count
    r: Num

    def __init__(self, l: Count, r: Num):
        if PROFILE:
            NUM_COUNTS["muls"] += 1

        if isinstance(l, Num) and l.depth > r.depth:
            l, r = r, l

        self.l = l
        self.r = r

    @property
    def left(self) -> Count:
        return self.l

    @property
    def right(self) -> Num:
        return self.r

    def __repr__(self) -> str:
        if self.l == -1:
            return f'-{self.r}'

        return super().__repr__()

    def __int__(self) -> int:
        return int(self.l) * int(self.r)

    def estimate(self) -> int:
        return round(self.estimate_l() + self.estimate_r())

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, Mul):
            return False

        return (
            (self.l == other.l and self.r == other.r)
            or (self.l == other.r and self.r == other.l)
        )

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

        if isinstance(other, Exp):
            if other.multiplies_with(self.r):  # no-branch
                return self.l * (self.r * other)

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

            if self.l == other.r:
                return self.l * (self.r + other.l)

            if self.r == other.l:
                return (self.l + other.r) * self.r

            if (isinstance(s_exp := self.r, Exp)
                    and isinstance(o_exp := other.r, Exp)
                    and s_exp.base == o_exp.base):
                try:
                    return add_exponents(
                        (s_exp, self.l),
                        (o_exp, other.l),
                    )
                except NotImplementedError:
                    pass

        elif isinstance(other, Add):
            if isinstance(other.l, int):  # no-branch
                return other.l + (other.r + self)

            if isinstance(other.l, Mul):
                if other.l.l == self.l:  # no-branch
                    return (self + other.l) + other.r

        elif isinstance(other, Exp):
            return other + self

        if self.l == -1 and other == self.r:
            return 0

        return super().__add__(other)

    def __floordiv__(self, other: int) -> Count:
        if other == 1:
            return self

        l, r = self.l, self.r

        if (lgcd := gcd(other, l)) > 1:
            return ((l // lgcd) * r) // (other // lgcd)

        if (rgcd := gcd(other, r)) > 1:
            return (l * (r // rgcd)) // (other // rgcd)

        return super().__floordiv__(other)  # no-cover

    def __lt__(self, other: Count) -> bool:
        if isinstance(other, Mul):
            if self.l == other.l:
                return self.r < other.r

            if self.r == other.r:
                return self.l < other.l

            if self.l == other.r:
                return self.r < other.l

            if self.r == other.l:  # no-branch
                return self.l < other.r

        if self.l < 0:  # no-cover
            raise NotImplementedError

        return super().__lt__(other)


class Div(Num):
    join = '//'

    num: Num
    den: int

    def __init__(self, num: Num, den: int):
        if PROFILE:
            NUM_COUNTS["divs"] += 1

        assert den > 0

        self.num = num
        self.den = den

    @property
    def left(self) -> Num:
        return self.num

    @property
    def right(self) -> int:
        return self.den

    def __int__(self) -> int:
        return int(self.num) // self.den

    def __eq__(self, other: object) -> bool:
        return (
            isinstance(other, Div)
            and self.den == other.den
            and self.num == other.num
        )

    def __neg__(self) -> Count:
        return -(self.num) // self.den

    def __mod__(self, other: int) -> int:
        if other == 1:
            return 0

        div, rem = divmod(
            self.num % (other * self.den),
            self.den)

        assert rem == 0

        return div % other

    def estimate(self) -> int:
        return round(self.estimate_l() - self.estimate_r())

    def __add__(self, other: Count) -> Count:
        return (self.num + (other * self.den)) // self.den

    def __radd__(self, other: int) -> Count:
        return ((other * self.den) + self.num) // self.den

    def __mul__(self, other: Count) -> Count:
        return (self.num * other) // self.den

    def __rmul__(self, other: int) -> Count:
        return (other * self.num) // self.den

    def __floordiv__(self, other: int) -> Count:
        if other == 1:
            return self

        return self.num // (other * self.den)

    def __lt__(self, other: Count) -> bool:
        if isinstance(other, Div):
            if self.den == other.den:  # no-branch
                return self.num < other.num

        return super().__lt__(other)


class Exp(Num):
    join = '**'

    base: int
    exp: Count

    def __init__(self, base: int, exp: Count):
        if PROFILE:
            NUM_COUNTS["exps"] += 1

        for _ in itertools.count():
            if not isinstance(base, int) or base <= 1:
                break

            if base == 8:
                base = 2
                exp *= 3
                break

            if floor(root := sqrt(base)) != ceil(root):
                break

            exp *= int(log(base, root))
            base = int(root)

        self.base = base
        self.exp = exp

    @property
    def left(self) -> int:
        return self.base

    @property
    def right(self) -> Count:
        return self.exp

    def __int__(self) -> int:
        return self.base ** int(self.exp)  # type: ignore[no-any-return]

    def __eq__(self, other: object) -> bool:
        return (
            isinstance(other, Exp)
            and self.base == other.base
            and self.exp == other.exp
        )

    def estimate(self) -> int:
        return round(self.estimate_l() * 10 ** self.estimate_r())

    def __mod__(self, other: int) -> int:
        if other == 1:
            return 0

        base = self.base

        if other == 1 or other == base:
            return 0

        res = 1

        exp = self.exp

        if (period := find_period(base, other)) > 0:
            exp %= period

        if not isinstance(exp, int):
            return exp_mod_special_cases(other, base, exp)

        for _ in itertools.count():
            if exp <= 0 or res <= 0:
                break

            if (exp % 2) == 1:
                res = (res * base) % other

            exp //= 2

            base = (base ** 2) % other

        return res

    def __add__(self, other: Count) -> Count:
        if isinstance(other, Mul):
            l, r = other.l, other.r

            base = self.base

            if isinstance(r, Exp) and r.base == base:
                try:
                    return add_exponents((self, 1), (r, l))
                except NotImplementedError:
                    pass

            if isinstance(l, Exp) and l.base == base:
                try:
                    return add_exponents((self, 1), (l, r))
                except NotImplementedError:
                    pass

        elif isinstance(other, Exp):
            if other.base == self.base:
                try:
                    return add_exponents((self, 1), (other, 1))
                except NotImplementedError:
                    pass

        return super().__add__(other)

    def multiplies_with(self, other: Count) -> bool:
        if isinstance(other, Exp):
            return other.base == self.base

        if isinstance(other, Mul):
            return (
                self.multiplies_with(other.l)
                or self.multiplies_with(other.r)
            )

        return False

    def __mul__(self, other: Count) -> Count:
        if isinstance(other, Exp):
            if (base := self.base) == other.base:
                return Exp(base, self.exp + other.exp)

        elif isinstance(other, Add):
            l, r = other.l, other.r

            if not isinstance(l, Exp) or not isinstance(r, Exp):
                return (self * l) + (self * r)

        elif isinstance(other, Mul):
            l, r = other.l, other.r

            if isinstance(l, int):
                return l * (self * r)

            if self.multiplies_with(l):
                return (self * l) * r

            if self.multiplies_with(r):  # no-branch
                return l * (self * r)

        return super().__mul__(other)

    def __rmul__(self, other: int) -> Count:
        if other == -1 or isinstance(base := self.base, Num):
            return super().__rmul__(other)

        if other < -1 and -other % base == 0:
            return -(-other * self)

        if other < 1 or other % base != 0:
            return super().__rmul__(other)

        exp = self.exp

        for i in itertools.count():
            if other % base != 0:
                exp += i
                break

            other //= base

        return other * Exp(base, exp)

    def __floordiv__(self, other: int) -> Count:
        if other == 1:
            return self

        base, exp = self.base, self.exp

        if not isinstance(base, int):  # no-cover
            return super().__floordiv__(other)

        for i in itertools.count():
            if other % base != 0:
                exp -= i
                break

            other //= base

        assert exp > 0

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


def add_exponents(
        l: tuple[Exp, Count],
        r: tuple[Exp, Count],
) -> Count:
    (l_exp, l_co), (r_exp, r_co) = l, r

    assert (base := l_exp.base) == r_exp.base

    if l_exp.exp > r_exp.exp:
        (l_exp, l_co), (r_exp, r_co) = (r_exp, r_co), (l_exp, l_co)

    assert (l_pow := l_exp.exp) <= (r_pow := r_exp.exp)

    diff_exp = (
        base ** diff
        if (diff := r_pow - l_pow) < 2 else
        Exp(base, diff)
    )

    return (l_co + (r_co * diff_exp)) * Exp(base, l_pow)


def gcd(l: int, r: Count) -> int:
    if l == 1 or r == 1:
        return 1

    if isinstance(r, int):
        return pgcd(l, r)

    if isinstance(r, Add):
        return min(gcd(l, r.l), gcd(l, r.r))

    if isinstance(r, Mul):
        return max(gcd(l, r.l), gcd(l, r.r))

    assert isinstance(r, Exp), (l, r)

    if not isinstance(base := r.base, int):  # no-cover
        return 1

    blog = int(log(l, base))

    over = base ** blog

    for _ in itertools.count():
        if l % over == 0 :
            break

        blog -= 1
        over //= base

    return base ** blog  # type: ignore[no-any-return]


@cache
def find_period(base: int, mod: int) -> int:
    if base % mod == 0:  # no-cover
        return 0

    if base == 3 and int(exp := log2(mod)) == exp:
        return 2 ** (int(exp) - 2)  # type: ignore[no-any-return]

    val = 1
    for period in range(1, mod):
        val *= base
        val %= mod

        if val == 1:
            return period

    return 0


def exp_mod_special_cases(mod: int, base: int, exp: Num) -> int:
    assert isinstance(exp, Div)

    if base == 3:
        if mod == 6:  # no-branch
            return 3

    if base != 2:  # no-cover
        raise NumException

    if mod == 12:
        return 4 if exp % 2 == 0 else 8

    if 2 * (3 ** round(log(mod / 2, 3))) != mod:  # no-cover
        raise NumException

    period = exp % (mod // 3)

    if mod == 6:
        return {
            0: 4,
            1: 2,
        }[period]

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
        }[period]

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
        }[period]

    if mod == 486:
        return {
            0: 244,
            1: 2,
            2: 4,
            3: 8,
            4: 16,
            5: 32,
            6: 64,
            7: 128,
            8: 256,
            9: 26,
            10: 52,
            11: 104,
            12: 208,
            13: 416,
            14: 346,
            15: 206,
            16: 412,
            17: 338,
            18: 190,
            19: 380,
            20: 274,
            21: 62,
            22: 124,
            23: 248,
            24: 10,
            25: 20,
            26: 40,
            27: 80,
            28: 160,
            29: 320,
            30: 154,
            31: 308,
            32: 130,
            33: 260,
            34: 34,
            35: 68,
            36: 136,
            37: 272,
            38: 58,
            39: 116,
            40: 232,
            41: 464,
            42: 442,
            43: 398,
            44: 310,
            45: 134,
            46: 268,
            47: 50,
            48: 100,
            49: 200,
            50: 400,
            51: 314,
            52: 142,
            53: 284,
            54: 82,
            55: 164,
            56: 328,
            57: 170,
            58: 340,
            59: 194,
            60: 388,
            61: 290,
            62: 94,
            63: 188,
            64: 376,
            65: 266,
            66: 46,
            67: 92,
            68: 184,
            69: 368,
            70: 250,
            71: 14,
            72: 28,
            73: 56,
            74: 112,
            75: 224,
            76: 448,
            77: 410,
            78: 334,
            79: 182,
            80: 364,
            81: 242,
            82: 484,
            83: 482,
            84: 478,
            85: 470,
            86: 454,
            87: 422,
            88: 358,
            89: 230,
            90: 460,
            91: 434,
            92: 382,
            93: 278,
            94: 70,
            95: 140,
            96: 280,
            97: 74,
            98: 148,
            99: 296,
            100: 106,
            101: 212,
            102: 424,
            103: 362,
            104: 238,
            105: 476,
            106: 466,
            107: 446,
            108: 406,
            109: 326,
            110: 166,
            111: 332,
            112: 178,
            113: 356,
            114: 226,
            115: 452,
            116: 418,
            117: 350,
            118: 214,
            119: 428,
            120: 370,
            121: 254,
            122: 22,
            123: 44,
            124: 88,
            125: 176,
            126: 352,
            127: 218,
            128: 436,
            129: 386,
            130: 286,
            131: 86,
            132: 172,
            133: 344,
            134: 202,
            135: 404,
            136: 322,
            137: 158,
            138: 316,
            139: 146,
            140: 292,
            141: 98,
            142: 196,
            143: 392,
            144: 298,
            145: 110,
            146: 220,
            147: 440,
            148: 394,
            149: 302,
            150: 118,
            151: 236,
            152: 472,
            153: 458,
            154: 430,
            155: 374,
            156: 262,
            157: 38,
            158: 76,
            159: 152,
            160: 304,
            161: 122,
        }[period]

    if mod == 1458:
        return {
            0: 730,
            1: 2,
            2: 4,
            3: 8,
            4: 16,
            5: 32,
            6: 64,
            7: 128,
            8: 256,
            9: 512,
            10: 1024,
            11: 590,
            12: 1180,
            13: 902,
            14: 346,
            15: 692,
            16: 1384,
            17: 1310,
            18: 1162,
            19: 866,
            20: 274,
            21: 548,
            22: 1096,
            23: 734,
            24: 10,
            25: 20,
            26: 40,
            27: 80,
            28: 160,
            29: 320,
            30: 640,
            31: 1280,
            32: 1102,
            33: 746,
            34: 34,
            35: 68,
            36: 136,
            37: 272,
            38: 544,
            39: 1088,
            40: 718,
            41: 1436,
            42: 1414,
            43: 1370,
            44: 1282,
            45: 1106,
            46: 754,
            47: 50,
            48: 100,
            49: 200,
            50: 400,
            51: 800,
            52: 142,
            53: 284,
            54: 568,
            55: 1136,
            56: 814,
            57: 170,
            58: 340,
            59: 680,
            60: 1360,
            61: 1262,
            62: 1066,
            63: 674,
            64: 1348,
            65: 1238,
            66: 1018,
            67: 578,
            68: 1156,
            69: 854,
            70: 250,
            71: 500,
            72: 1000,
            73: 542,
            74: 1084,
            75: 710,
            76: 1420,
            77: 1382,
            78: 1306,
            79: 1154,
            80: 850,
            81: 242,
            82: 484,
            83: 968,
            84: 478,
            85: 956,
            86: 454,
            87: 908,
            88: 358,
            89: 716,
            90: 1432,
            91: 1406,
            92: 1354,
            93: 1250,
            94: 1042,
            95: 626,
            96: 1252,
            97: 1046,
            98: 634,
            99: 1268,
            100: 1078,
            101: 698,
            102: 1396,
            103: 1334,
            104: 1210,
            105: 962,
            106: 466,
            107: 932,
            108: 406,
            109: 812,
            110: 166,
            111: 332,
            112: 664,
            113: 1328,
            114: 1198,
            115: 938,
            116: 418,
            117: 836,
            118: 214,
            119: 428,
            120: 856,
            121: 254,
            122: 508,
            123: 1016,
            124: 574,
            125: 1148,
            126: 838,
            127: 218,
            128: 436,
            129: 872,
            130: 286,
            131: 572,
            132: 1144,
            133: 830,
            134: 202,
            135: 404,
            136: 808,
            137: 158,
            138: 316,
            139: 632,
            140: 1264,
            141: 1070,
            142: 682,
            143: 1364,
            144: 1270,
            145: 1082,
            146: 706,
            147: 1412,
            148: 1366,
            149: 1274,
            150: 1090,
            151: 722,
            152: 1444,
            153: 1430,
            154: 1402,
            155: 1346,
            156: 1234,
            157: 1010,
            158: 562,
            159: 1124,
            160: 790,
            161: 122,
            162: 244,
            163: 488,
            164: 976,
            165: 494,
            166: 988,
            167: 518,
            168: 1036,
            169: 614,
            170: 1228,
            171: 998,
            172: 538,
            173: 1076,
            174: 694,
            175: 1388,
            176: 1318,
            177: 1178,
            178: 898,
            179: 338,
            180: 676,
            181: 1352,
            182: 1246,
            183: 1034,
            184: 610,
            185: 1220,
            186: 982,
            187: 506,
            188: 1012,
            189: 566,
            190: 1132,
            191: 806,
            192: 154,
            193: 308,
            194: 616,
            195: 1232,
            196: 1006,
            197: 554,
            198: 1108,
            199: 758,
            200: 58,
            201: 116,
            202: 232,
            203: 464,
            204: 928,
            205: 398,
            206: 796,
            207: 134,
            208: 268,
            209: 536,
            210: 1072,
            211: 686,
            212: 1372,
            213: 1286,
            214: 1114,
            215: 770,
            216: 82,
            217: 164,
            218: 328,
            219: 656,
            220: 1312,
            221: 1166,
            222: 874,
            223: 290,
            224: 580,
            225: 1160,
            226: 862,
            227: 266,
            228: 532,
            229: 1064,
            230: 670,
            231: 1340,
            232: 1222,
            233: 986,
            234: 514,
            235: 1028,
            236: 598,
            237: 1196,
            238: 934,
            239: 410,
            240: 820,
            241: 182,
            242: 364,
            243: 728,
            244: 1456,
            245: 1454,
            246: 1450,
            247: 1442,
            248: 1426,
            249: 1394,
            250: 1330,
            251: 1202,
            252: 946,
            253: 434,
            254: 868,
            255: 278,
            256: 556,
            257: 1112,
            258: 766,
            259: 74,
            260: 148,
            261: 296,
            262: 592,
            263: 1184,
            264: 910,
            265: 362,
            266: 724,
            267: 1448,
            268: 1438,
            269: 1418,
            270: 1378,
            271: 1298,
            272: 1138,
            273: 818,
            274: 178,
            275: 356,
            276: 712,
            277: 1424,
            278: 1390,
            279: 1322,
            280: 1186,
            281: 914,
            282: 370,
            283: 740,
            284: 22,
            285: 44,
            286: 88,
            287: 176,
            288: 352,
            289: 704,
            290: 1408,
            291: 1358,
            292: 1258,
            293: 1058,
            294: 658,
            295: 1316,
            296: 1174,
            297: 890,
            298: 322,
            299: 644,
            300: 1288,
            301: 1118,
            302: 778,
            303: 98,
            304: 196,
            305: 392,
            306: 784,
            307: 110,
            308: 220,
            309: 440,
            310: 880,
            311: 302,
            312: 604,
            313: 1208,
            314: 958,
            315: 458,
            316: 916,
            317: 374,
            318: 748,
            319: 38,
            320: 76,
            321: 152,
            322: 304,
            323: 608,
            324: 1216,
            325: 974,
            326: 490,
            327: 980,
            328: 502,
            329: 1004,
            330: 550,
            331: 1100,
            332: 742,
            333: 26,
            334: 52,
            335: 104,
            336: 208,
            337: 416,
            338: 832,
            339: 206,
            340: 412,
            341: 824,
            342: 190,
            343: 380,
            344: 760,
            345: 62,
            346: 124,
            347: 248,
            348: 496,
            349: 992,
            350: 526,
            351: 1052,
            352: 646,
            353: 1292,
            354: 1126,
            355: 794,
            356: 130,
            357: 260,
            358: 520,
            359: 1040,
            360: 622,
            361: 1244,
            362: 1030,
            363: 602,
            364: 1204,
            365: 950,
            366: 442,
            367: 884,
            368: 310,
            369: 620,
            370: 1240,
            371: 1022,
            372: 586,
            373: 1172,
            374: 886,
            375: 314,
            376: 628,
            377: 1256,
            378: 1054,
            379: 650,
            380: 1300,
            381: 1142,
            382: 826,
            383: 194,
            384: 388,
            385: 776,
            386: 94,
            387: 188,
            388: 376,
            389: 752,
            390: 46,
            391: 92,
            392: 184,
            393: 368,
            394: 736,
            395: 14,
            396: 28,
            397: 56,
            398: 112,
            399: 224,
            400: 448,
            401: 896,
            402: 334,
            403: 668,
            404: 1336,
            405: 1214,
            406: 970,
            407: 482,
            408: 964,
            409: 470,
            410: 940,
            411: 422,
            412: 844,
            413: 230,
            414: 460,
            415: 920,
            416: 382,
            417: 764,
            418: 70,
            419: 140,
            420: 280,
            421: 560,
            422: 1120,
            423: 782,
            424: 106,
            425: 212,
            426: 424,
            427: 848,
            428: 238,
            429: 476,
            430: 952,
            431: 446,
            432: 892,
            433: 326,
            434: 652,
            435: 1304,
            436: 1150,
            437: 842,
            438: 226,
            439: 452,
            440: 904,
            441: 350,
            442: 700,
            443: 1400,
            444: 1342,
            445: 1226,
            446: 994,
            447: 530,
            448: 1060,
            449: 662,
            450: 1324,
            451: 1190,
            452: 922,
            453: 386,
            454: 772,
            455: 86,
            456: 172,
            457: 344,
            458: 688,
            459: 1376,
            460: 1294,
            461: 1130,
            462: 802,
            463: 146,
            464: 292,
            465: 584,
            466: 1168,
            467: 878,
            468: 298,
            469: 596,
            470: 1192,
            471: 926,
            472: 394,
            473: 788,
            474: 118,
            475: 236,
            476: 472,
            477: 944,
            478: 430,
            479: 860,
            480: 262,
            481: 524,
            482: 1048,
            483: 638,
            484: 1276,
            485: 1094,
        }[period]

    raise NumException

########################################

Count = int | Num


TRUNCATE_COUNT = 10 ** 12

MAX_DEPTH = 120

def show_number(num: Count) -> str:
    if isinstance(num, int):
        if abs(num) >= TRUNCATE_COUNT:
            return "{}(~10^{:.0f})".format(
                '-' if num < 0 else '',
                log10(abs(num)),
            )

    elif num.depth > MAX_DEPTH:  # no-cover
        return "(???)"

    return str(num)
