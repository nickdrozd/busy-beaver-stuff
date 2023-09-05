# pylint: disable = useless-parent-delegation

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

    @abstractmethod
    def __neg__(self) -> Count: ...

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

    def estimate(self) -> int:
        return round(self.estimate_l() + self.estimate_r())

    def __neg__(self) -> Count:
        return -(self.l) * self.r

    def __mod__(self, other: int) -> int:
        return ((self.l % other) * (self.r % other)) % other

    def __mul__(self, other: Count) -> Count:
        return super().__mul__(other)

    def __rmul__(self, other: Count) -> Count:
        if (isinstance(other, Num)
                or isinstance(self.l, Num)):
            return super().__rmul__(other)

        return (self.l * other) * self.r

    def __add__(self, other: Count) -> Count:
        if isinstance(other, Mul) and self.r == other.r:
            return (self.l + other.l) * self.r

        if isinstance(other, Add) and isinstance(other.l, int):
            return other.l + (other.r + self)

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

    def __init__(self, l: Num, r: Count):
        super().__init__(l, r)

    def __neg__(self) -> Count:
        raise NotImplementedError

    def __mod__(self, other: int) -> int:
        assert isinstance(self.r, int)

        if other == self.r:
            return 0

        try:  # pylint: disable = too-many-try-statements
            rem, div = divmod(
                self.l % (other * self.r),
                self.r
            )

            assert div == 0

            return rem % other
        except (NumException, AssertionError):
            pass

        try:
            inv = pow(self.r, -1, other)
        except ValueError as exc:
            raise NumException from exc

        return ((self.l % other) * (inv % other)) % other

    def estimate(self) -> int:
        return round(self.estimate_l() - self.estimate_r())

    def __rmul__(self, other: Count) -> Count:
        if (isinstance(other, Num)
                or isinstance(self.r, Num)
                or other % self.r != 0):
            return super().__rmul__(other)

        return (other // self.r) * self.l

    def __floordiv__(self, other: Count) -> Div:
        return Div(self, other * self.r)


class Exp(Num):
    join = '**'

    op = operator.pow

    def __init__(self, l: Count, r: Count):
        while (isinstance(l, int)  # pylint: disable = while-used
                   and l > 1
                   and floor(root := sqrt(l)) == ceil(root)):
            r *= int(log(l, root))
            l = int(root)

        super().__init__(l, r)

    def estimate(self) -> int:
        return round(self.estimate_l() * 10 ** self.estimate_r())

    def __neg__(self) -> Count:
        return (
            Exp(-(self.l), self.r)
            if self.r % 2 == 1 else
            -(self.l) * Exp(self.l, self.r - 1)
        )

    def __mod__(self, other: int) -> int:
        if other == 1 or other == self.l:
            return 0

        res = 1

        base, exp = self.l, self.r

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
            if isinstance(exp := other.r, Exp) and exp.l == self.l:
                try:
                    return _add_exponents((self, 1), (exp, other.l))
                except NotImplementedError:
                    pass

        if (isinstance(other, Exp)
                and other.l == self.l
                and isinstance(self.r, int)
                and isinstance(other.r, int)):
            return _add_exponents((self, 1), (other, 1))

        return super().__add__(other)

    def __mul__(self, other: Count) -> Count:
        if isinstance(other, Exp) and (base := self.l) == other.l:
            return Exp(base, self.r + other.r)

        if isinstance(other, Add):
            return (self * other.l) + (self * other.r)

        if isinstance(other, Mul):
            if isinstance(other.l, int):
                return other.l * (self * other.r)

        return super().__mul__(other)

    def __rmul__(self, other: Count) -> Count:
        if (isinstance(other, Num)
                or other < 1
                or isinstance(self.l, Num)
                or log10(other) > 20
                or other % self.l != 0
            ):
            return super().__rmul__(other)

        r = self.r
        l = self.l

        while other % l == 0:  # pylint: disable = while-used
            other //= l
            r += 1

        return other * Exp(l, r)

    def __floordiv__(self, other: Count) -> Count:
        if other == self.l:
            return Exp(self.l, self.r - 1)

        return super().__floordiv__(other)

    def __lt__(self, other: Count) -> bool:
        if isinstance(other, Exp):
            if self.l == other.l:
                return self.r < other.r

            if self.l < other.l and self.r < other.r:
                return True

            if self.l > other.l and self.r > other.r:
                return False

        return super().__lt__(other)


def _add_exponents(
        l: tuple[Exp, Count],
        r: tuple[Exp, Count],
) -> Count:
    (l, l_co), (r, r_co) = l, r

    assert (base := l.l) == r.l

    if (l_pow := l.r) > (r_pow := r.r):
        return _add_exponents((r, r_co), (l, l_co))

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
