from __future__ import annotations

import operator
from abc import abstractmethod
from collections.abc import Callable

from tm.show import show_number as show


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
        return f'({show(self.l)} {self.join} {show(self.r)})'

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

    @abstractmethod
    def __neg__(self) -> Count: ...

    def __eq__(self, other: object) -> bool:
        return (
            isinstance(other, type(self))
            and self.l == other.l
            and self.r == other.r
        )

    def __lt__(self, _: int) -> bool:
        return False  # no-coverage

    def __le__(self, _: int) -> bool:
        return False

    def __gt__(self, _: int) -> bool:
        return True

    def __ge__(self, _: int) -> bool:
        return True

    def __add__(self, other: Count) -> Count:
        copy = self.copy()

        return (
            Add(other, copy)
            if isinstance(other, int) else
            Add(copy, other.copy())
        )

    def __radd__(self, other: Count) -> Count:
        return Add(other, self.copy())

    def __sub__(self, other: Count) -> Count:
        if other == 0:  # no-coverage
            return self.copy()

        return self + -other

    def __rsub__(self, other: Count) -> Count:
        return other + -self  # no-coverage

    def __mul__(self, other: Count) -> Count:
        return other * self  # no-coverage

    def __rmul__(self, other: Count) -> Count:
        if other == 1:  # no-coverage
            return self.copy()

        return Mul(
            other.copy() if isinstance(other, Num) else other,
            self.copy(),
        )

    @abstractmethod
    def __mod__(self, other: int) -> int: ...

    def __divmod__(self, other: int) -> tuple[Count, int]:
        mod = self % other

        return (self - mod) // other, mod

    def __floordiv__(self, other: Count) -> Count:
        if other == 1:
            return self.copy()

        return Div(self.copy(), other)

    def __pow__(self, other: Count) -> Count:
        return Exp(self.copy(), other)  # no-coverage


class Add(Num):
    join = '+'

    op = operator.add

    def __init__(self, l: Count, r: Num):
        super().__init__(l, r)

    def rcopy(self) -> Num:
        assert isinstance(self.r, Num)
        return self.r.copy()

    def __mod__(self, other: int) -> int:
        return ((self.l % other) + (self.r % other)) % other

    def __neg__(self) -> Count:
        return -(self.l) + -(self.r)

    def __add__(self, other: Count) -> Add:
        copy = self.copy()

        assert isinstance(copy, Add)

        copy.l += other

        return copy

    def __radd__(self, other: Count) -> Add:
        return self + other

    def __iadd__(self, other: Count) -> Count:
        if isinstance(other, Num):
            return Add(self.copy(), other.copy())

        if isinstance(self.l, int):
            l = other + self.l
            r = self.rcopy()

            return r if l == 0 else l + r

        self.l += other

        return self

    def __sub__(self, other: Count) -> Count:
        if isinstance(other, Add) and self.r == other.r:
            return self.l - other.l

        return self + -other

    def __isub__(self, other: Count) -> Count:
        if isinstance(self.l, int):
            l = self.l - other
            r = self.rcopy()

            return r if l == 0 else l + r

        self.l -= other

        return self

    def __rmul__(self, other: Count) -> Count:
        if isinstance(other, Num):  # no-coverage
            return super().__rmul__(other)

        return (other * self.l) + (other * self.r)


class Mul(Num):
    join = '*'

    op = operator.mul

    def __init__(self, l: Count, r: Num):
        super().__init__(l, r)

    def __neg__(self) -> Count:
        return -(self.l) * self.r

    def __mod__(self, other: int) -> int:
        return ((self.l % other) * (self.r % other)) % other

    def __rmul__(self, other: Count) -> Count:
        if (isinstance(other, Num)
                or isinstance(self.l, Num)):  # no-coverage
            return super().__rmul__(other)

        return (self.l * other) * self.r

    def __add__(self, other: Count) -> Count:
        if isinstance(other, Mul) and self.r == other.r:  # no-coverage
            return (self.l + other.l) * self.r

        return super().__add__(other)


class Div(Num):
    join = '//'

    op = operator.floordiv

    def __neg__(self) -> Count:
        raise NotImplementedError

    def __mod__(self, other: int) -> int:
        assert isinstance(self.r, int)

        if other == self.r:
            return 0

        try:
            inv = pow(self.r, -1, other)
        except ValueError as exc:
            raise NumException from exc

        return ((self.l % other) * (inv % other)) % other

    def __rmul__(self, other: Count) -> Count:
        if (isinstance(other, Num)
                or isinstance(self.r, Num)
                or other % self.r != 0):
            return super().__rmul__(other)

        return (other // self.r) * self.l

    def __floordiv__(self, other: Count) -> Div:
        return Div(
            self.lcopy(),
            other * self.r,
        )


class Exp(Num):
    join = '**'

    op = operator.pow

    def __neg__(self) -> Count:
        raise NotImplementedError

    def __mod__(self, other: int) -> int:
        if other == 1:
            return 0

        res = 1

        base, exp = self.l, self.r

        while exp > 0 and res > 0:  # pylint: disable = while-used
            if (exp % 2) == 1:
                res = (res * base) % other
            exp //= 2
            base = (base ** 2) % other

        return res

########################################

Count = int | Num
