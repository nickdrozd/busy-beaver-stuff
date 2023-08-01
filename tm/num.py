from __future__ import annotations

import operator
from dataclasses import dataclass
from collections.abc import Callable

from tm.show import show_number


@dataclass
class Num:
    l: Count
    r: Count

    join: str
    op: Callable[[int, int], int]

    def __init__(self, l: Count, r: Count):
        self.l = l
        self.r = r

    def __repr__(self) -> str:
        return f'({self.lstr} {self.join} {self.rstr})'

    @property
    def lstr(self) -> str:
        return (
            show_number(self.l)
            if isinstance(self.l, int)
            else
            str(self.l)
        )

    @property
    def rstr(self) -> str:
        return (
            show_number(self.r)
            if isinstance(self.r, int)
            else
            str(self.r)
        )

    def lcopy(self) -> Count:
        return self.l.copy() if isinstance(self.l, Num) else self.l

    def rcopy(self) -> Count:
        return self.r.copy() if isinstance(self.r, Num) else self.r

    def copy(self) -> Num:
        return type(self)(
            self.lcopy(),
            self.rcopy(),
        )

    def __int__(self) -> int:
        return self.op(
            int(self.l),
            int(self.r),
        )

    def __neg__(self) -> Count:
        raise NotImplementedError

    def __lt__(self, other: Count) -> bool:
        if not isinstance(other, int):
            raise NotImplementedError

        return False

    def __le__(self, other: Count) -> bool:
        if not isinstance(other, int):
            raise NotImplementedError

        return False

    def __gt__(self, other: Count) -> bool:
        if not isinstance(other, int):
            raise NotImplementedError

        return True

    def __ge__(self, other: Count) -> bool:
        if not isinstance(other, int):
            raise NotImplementedError

        return True

    def __add__(self, other: Count) -> Count:
        return Add(
            self.copy(),
            other.copy() if isinstance(other, Num) else other,
        )

    def __radd__(self, other: Count) -> Count:
        return Add(
            other.copy() if isinstance(other, Num) else other,
            self.copy(),
        )

    def __sub__(self, other: Count) -> Count:
        if other == 0:
            return self.copy()

        return Add(
            self.copy(),
            -(other.copy()) if isinstance(other, Num) else -other,
        )

    def __rsub__(self, other: Count) -> Count:
        return Add(other, -(self.copy()))

    def __mul__(self, other: Count) -> Count:
        return Mul(
            self.copy(),
            other.copy() if isinstance(other, Num) else other,
        )

    def __rmul__(self, other: Count) -> Count:
        if other == 1:
            return self.copy()

        return Mul(
            other.copy() if isinstance(other, Num) else other,
            self.copy(),
        )

    def __mod__(self, other: int) -> int:
        raise NotImplementedError

    def __rmod__(self, other: Count) -> int:
        if isinstance(other, int):
            return other

        raise NotImplementedError

    def __divmod__(self, other: int) -> tuple[Count, int]:
        mod = self % other

        return (self - mod) // other, mod

    def __rdivmod__(self, other: Count) -> tuple[Count, int]:
        if isinstance(other, int):
            return 0, other

        raise NotImplementedError

    def __floordiv__(self, other: Count) -> Count:
        if other == 1:
            return self.copy()

        return Div(self.copy(), other)

    def __pow__(self, other: Count) -> Count:
        return Exp(self.copy(), other)


class Add(Num):
    join = '+'

    op = operator.add

    def __mod__(self, other: int) -> int:
        return ((self.l % other) + (self.r % other)) % other

    def __neg__(self) -> Add:
        return Add(-(self.lcopy()), -(self.rcopy()))

    def __iadd__(self, other: Count) -> Count:
        if isinstance(other, Num):
            return Add(self.copy(), other.copy())

        l: Count
        r: Count

        if isinstance(self.l, int):
            l = other + self.l
            r = self.rcopy()

            return self.r if l == 0 else Add(l, r)

        if isinstance(self.r, int):
            r = other + self.r
            l = self.lcopy()

            return self.l if r == 0 else Add(l, r)

        self.l += other

        return self

    def __isub__(self, other: Count) -> Count:
        if isinstance(other, Num):
            return Add(-(self.copy()), -(other.copy()))

        l: Count
        r: Count

        if isinstance(self.l, int):
            l = self.l - other
            r = self.rcopy()

            return self.r if l == 0 else Add(l, r)

        if isinstance(self.r, int):
            r = self.r - other
            l = self.lcopy()

            return self.l if r == 0 else Add(l, r)

        self.l -= other

        return self


class Mul(Num):
    join = '*'

    op = operator.mul

    def __neg__(self) -> Mul:
        return Mul(
            -(self.lcopy()),
            self.rcopy(),
        )

    def __mod__(self, other: int) -> int:
        return ((self.l % other) * (self.r % other)) % other


class Div(Num):
    join = '//'

    op = operator.floordiv

    def __neg__(self) -> Div:
        return Div(
            -(self.lcopy()),
            self.rcopy(),
        )

    def __mod__(self, other: int) -> int:
        if not isinstance(self.r, int):
            raise NotImplementedError

        if other == self.r:
            return 0

        try:
            inv = pow(self.r, -1, other)
        except ValueError:
            return int(self) % other

        return ((self.l % other) * (inv % other)) % other


class Exp(Num):
    join = '**'

    op = operator.pow

    def __neg__(self) -> Count:
        raise NotImplementedError

    def __mod__(self, other: int) -> int:
        if other == 1:
            return 0

        if not isinstance(self.r, int):
            raise NotImplementedError

        res = 1

        base, exp = self.l, self.r

        while exp > 0:  # pylint: disable = while-used
            if (exp % 2) == 1:
                res = (res * base) % other
            exp >>= 1
            base = (base ** 2) % other

        return res

########################################

Count = int | Num
