from __future__ import annotations

import operator
from abc import abstractmethod
from math import sqrt, floor, ceil, log, log10
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
        # return f'{type(self).__name__}({self.l}, {self.r})'

        return f'({show(self.l)} {self.join} {show(self.r)})'

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

    def __add__(self, other: Count) -> Num:
        return (
            self
            if other == 0 else
            Add(other, self)
            if isinstance(other, int) else
            Add(self, other)
        )

    def __radd__(self, other: Count) -> Num:
        assert isinstance(other, int)

        return self + other

    def __sub__(self, other: Count) -> Count:
        if other == 0:  # no-coverage
            return self

        return self + -other

    def __rsub__(self, other: Count) -> Count:
        return other + -self  # no-coverage

    def __mul__(self, other: Count) -> Count:
        if other == 1:  # no-coverage
            return self

        return Mul(other, self)

    def __rmul__(self, other: Count) -> Count:
        if other == 0:
            return 0

        if other == 1:  # no-coverage
            return self

        return Mul(other, self)

    @abstractmethod
    def __mod__(self, other: int) -> int: ...

    def __divmod__(self, other: int) -> tuple[Count, int]:
        mod = self % other

        return (self - mod) // other, mod

    def __floordiv__(self, other: Count) -> Count:
        if other == 1:
            return self

        return Div(self, other)

    def __pow__(self, other: Count) -> Exp:
        return Exp(self, other)  # no-coverage

    def __rpow__(self, other: Count) -> Exp:
        return Exp(other, self)  # no-coverage


class Add(Num):
    join = '+'

    op = operator.add

    def __init__(self, l: Count, r: Num):
        super().__init__(l, r)

    def __mod__(self, other: int) -> int:
        return ((self.l % other) + (self.r % other)) % other

    def __neg__(self) -> Count:
        return -(self.l) + -(self.r)

    def __add__(self, other: Count) -> Num:
        r = self.r

        assert isinstance(r, Num)

        return (
            r
            if (ladd := self.l + other) == 0 else
            Add(ladd, r)
        )

    def __sub__(self, other: Count) -> Count:
        if isinstance(other, Add) and self.r == other.r:
            return self.l - other.l

        return self + -other

    def __mul__(self, other: Count) -> Count:
        if isinstance(other, Num):
            return super().__mul__(other)

        return other * self

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

    def __add__(self, other: Count) -> Num:
        if isinstance(other, Mul) and self.r == other.r:
            val = (self.l + other.l) * self.r
            assert isinstance(val, Num)
            return val

        if isinstance(other, Add) and isinstance(other.l, int):
            return other.l + (other.r + self)

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

    def __neg__(self) -> Count:
        return (
            Exp(-(self.l), self.r)
            if self.r % 2 == 1 else
            self.l * -Exp(self.l, self.r - 1)
        )

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

    def __add__(self, other: Count) -> Num:
        if isinstance(other, Mul) and other.r == self:
            result = (1 + other.l) * self
            assert isinstance(result, Num)
            return result

        return super().__add__(other)

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

        assert isinstance(l, int)

        while other % l == 0:  # pylint: disable = while-used
            other //= l
            r += 1

        return other * Exp(l, r)

########################################

Count = int | Num
