# pylint: disable = useless-parent-delegation, confusing-consecutive-elif

from __future__ import annotations

import operator
from abc import abstractmethod
from math import sqrt, floor, ceil, log, log10
from typing import TYPE_CHECKING

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

    def depth(self) -> int:
        l_depth = 0 if isinstance(self.l, int) else self.l.depth()
        r_depth = 0 if isinstance(self.r, int) else self.r.depth()

        return 1 + max(l_depth, r_depth)

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

        raise NotImplementedError

    def __le__(self, other: Count) -> bool:
        return self == other or self < other

    def __gt__(self, other: Count) -> bool:
        return self != other and not self < other

    def __ge__(self, other: Count) -> bool:
        return self == other or self > other

    def __add__(self, other: Count) -> Count:
        return (
            self if other == 0
            else
            Add(other, self) if isinstance(other, int)
            else
            2 * self if other == self
            else
            other.l + (self + other.r)
                if (isinstance(other, Add)
                    and isinstance(other.l, int))
            else
            Add(self, other)
        )

    def __radd__(self, other: Count) -> Count:
        assert isinstance(other, int)

        return self + other

    def __sub__(self, other: Count) -> Count:
        return self + -other

    def __rsub__(self, other: Count) -> Count:
        return other + -self

    def __mul__(self, other: Count) -> Count:
        return (
            other * self
            if isinstance(other, int) else
            Mul(self, other)
        )

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

    def __floordiv__(self, other: Count) -> Count:
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
            if isinstance(other, int) else
            super().__rmul__(other)
        )

    def __floordiv__(self, other: Count) -> Count:
        if (isinstance(other, int)
                and self.l % other == 0
                and self.r % other == 0):
            return (self.l // other) + (self.r // other)

        return super().__floordiv__(other)

    def __lt__(self, other: Count) -> bool:
        if isinstance(other, Add):
            if self.l == other.l:
                return self.r < other.r

            if self.r == other.r:
                return self.l < other.l

        return super().__lt__(other)


class Mul(Num):
    join = '*'

    op = operator.mul

    def __init__(self, l: Count, r: Num):
        if isinstance(l, Num) and l.depth() > r.depth():
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
        return ((self.l % other) * (self.r % other)) % other

    def __mul__(self, other: Count) -> Count:
        return super().__mul__(other)

    def __rmul__(self, other: Count) -> Count:
        if (isinstance(other, int)
                or (isinstance(s_exp := self.l, Exp)
                   and other == s_exp.base)):
            return (other * self.l) * self.r

        return super().__rmul__(other)

    def __add__(self, other: Count) -> Count:
        if isinstance(other, Mul):
            if self.r == other.r:
                return (self.l + other.l) * self.r

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
            if isinstance(other.l, int):
                return other.l + (other.r + self)

        elif isinstance(other, Exp):
            return other + self

        return super().__add__(other)

    def __floordiv__(self, other: Count) -> Count:
        if isinstance(other, int):
            if self.l % other == 0:
                return (self.l // other) * self.r

            if self.r % other == 0:  # pragma: no branch
                return self.l * (self.r // other)

        return super().__floordiv__(other)


class Div(Num):
    join = '//'

    op = operator.floordiv

    @property
    def num(self) -> Count:
        return self.l

    @property
    def den(self) -> Count:
        return self.r

    def __init__(self, l: Num, r: Count):
        super().__init__(l, r)

    def __mod__(self, other: int) -> int:
        assert isinstance(den := self.den, int)

        if other == den:
            return 0

        try:  # pylint: disable = too-many-try-statements
            rem, div = divmod(
                self.num % (other * den),
                den
            )

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

    def __mul__(self, other: Count) -> Count:
        return other * self

    def __rmul__(self, other: Count) -> Count:
        if (isinstance(other, Num)
                or isinstance(self.den, Num)
                or other % self.den != 0):
            return super().__rmul__(other)

        return (other // self.den) * self.num

    def __floordiv__(self, other: Count) -> Div:
        return Div(self, other * self.den)


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
        while (isinstance(l, int)  # pylint: disable = while-used
                   and l > 1
                   and floor(root := sqrt(l)) == ceil(root)):
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

    def __floordiv__(self, other: Count) -> Count:
        if other == self.l:
            return Exp(self.l, self.r - 1)

        return super().__floordiv__(other)

    def __lt__(self, other: Count) -> bool:
        if isinstance(other, Exp):
            if self.base == other.base:
                return self.exp < other.exp

            if self.base < other.base and self.exp < other.exp:
                return True

            if self.base > other.base and self.exp > other.exp:
                return False

        return super().__lt__(other)


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

########################################

Count = int | Num


TRUNCATE_COUNT = 10 ** 12

def show_number(num: Count) -> str:
    return (
        str(num)
        if not isinstance(num, int) or num < TRUNCATE_COUNT else
        f"(~10^{log10(num):.0f})"
    )
