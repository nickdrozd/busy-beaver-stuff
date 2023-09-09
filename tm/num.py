# pylint: disable = useless-parent-delegation, confusing-consecutive-elif

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

    def __init__(self, l: Count, r: Count):
        self.l = l
        self.r = r

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

    def __rmul__(self, other: Count) -> Count:
        return (
            0 if other == 0 else
            self if other == 1 else
            Mul(other, self)
        )

    @abstractmethod
    def __mod__(self, other: int) -> int: ...

    def __divmod__(self, other: int) -> tuple[Count, int]:
        mod = self % other

        return (self - mod) // other, mod

    def __floordiv__(self, other: int) -> Count:
        return (
            self
            if other == 1 else
            Div(self, other)
        )

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

        super().__init__(l, r)

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

    def __rmul__(self, other: Count) -> Count:
        return (
            (other * self.l) + (other * self.r)
            if isinstance(other, int) and other != -1 else
            super().__rmul__(other)
        )

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
            if self.l == other.l:
                return self.r < other.r

            if self.r == other.r:
                return self.l < other.l

            if (isinstance(self.l, int)
                    and isinstance(other.l, int)
                    and abs(self.l - other.l) < 10):
                return self.r < other.r

        if isinstance(self.l, int) and abs(self.l) < 10:
            return self.r < other

        return super().__lt__(other)


class Mul(Num):
    join = '*'

    op = operator.mul

    def __init__(self, l: Count, r: Num):
        if isinstance(l, Num) and l.depth > r.depth:
            l, r = r, l

        super().__init__(l, r)

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
        return (
            -1 * (self.r * other)
            if self.l == -1 else
            super().__mul__(other)
        )

    def __rmul__(self, other: Count) -> Count:
        if ((isinstance(other, int) and other != -1)
                or (isinstance(s_exp := self.l, Exp)
                   and other == s_exp.base)):
            return (other * self.l) * self.r

        return super().__rmul__(other)  # no-coverage

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
        super().__init__(l, r)

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

    def __rmul__(self, other: Count) -> Count:
        return (
            (other * self.num) // self.den
            if other != -1 else
            super().__rmul__(other)
        )

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

        super().__init__(l, r)

    def estimate(self) -> int:
        return round(self.estimate_l() * 10 ** self.estimate_r())

    def __mod__(self, other: int) -> int:
        if other == 1 or other == self.base:
            return 0

        res = 1

        base, exp = self.base, self.exp

        if not isinstance(exp, int):
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

    def __rmul__(self, other: Count) -> Count:
        if isinstance(other, Num) or isinstance(self.base, Num):
            return super().__rmul__(other)

        assert isinstance(other, int)

        base = self.base

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
        if not isinstance(base, int) or diff < 1_000 else
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
