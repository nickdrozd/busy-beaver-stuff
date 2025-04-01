# ruff: noqa: E501, SIM102, PLR0911
# pylint: disable = too-complex, too-many-try-statements
# pylint: disable = confusing-consecutive-elif, used-before-assignment

from __future__ import annotations

import itertools
from abc import abstractmethod
from collections import defaultdict
from functools import cache
from math import ceil, floor, log, log2, log10, sqrt
from math import gcd as pgcd
from typing import Final


class ExpModLimit(Exception):
    pass

class ModDepthLimit(Exception):
    pass

class PeriodLimit(Exception):
    pass


type Count = int | Num


ADDS: dict[Count, dict[Num, Add]] = defaultdict(dict)
MULS: dict[Count, dict[Num, Mul]] = defaultdict(dict)
DIVS: dict[Num, dict[int, Div]] = defaultdict(dict)
EXPS: dict[int, dict[Count, Exp]] = defaultdict(dict)


class Num:
    depth: int

    leaves: int

    tower_est: int

    @abstractmethod
    def __int__(self) -> int: ...

    def __contains__(self, other: Num) -> bool:
        return False

    @abstractmethod
    def __hash__(self) -> int: ...

    def estimate(self) -> Count:
        est: Exp | Tet

        if (tower := self.tower_est) > 3:
            est = Tet(10, tower)
        else:
            try:
                digits = self.digits()
            except OverflowError:
                est = Tet(10, tower)
            else:
                est = make_exp(10, digits)

        return -est if self < 0 else est

    @abstractmethod
    def digits(self) -> int: ...

    @abstractmethod
    def __neg__(self) -> Count: ...

    def __eq__(self, other: object) -> bool:
        return other is self

    def __lt__(self, other: Count) -> bool:
        if isinstance(other, Add | Mul):
            l, r = other.l, other.r

            try:
                if self <= r and l > 0:
                    return True
            except NotImplementedError:
                pass

            try:
                if self <= l and r > 0:  # no-branch
                    return True  # no-cover
            except NotImplementedError:  # no-cover
                pass

            if self == r:
                return 0 < l

            if self == l:  # no-cover
                return 0 < r

        if isinstance(other, Add):
            if isinstance(l, int) and abs(l) < 10:
                return self < r

        raise NotImplementedError(self, other)

    def __le__(self, other: Count) -> bool:
        return self == other or self < other

    def __gt__(self, other: Count) -> bool:
        return self != other and not self < other

    def __ge__(self, other: Count) -> bool:
        return self == other or self > other

    @abstractmethod
    def __add__(self, other: Count) -> Count: ...

    @abstractmethod
    def __radd__(self, other: int) -> Count: ...

    @abstractmethod
    def __sub__(self, other: Count) -> Count: ...

    def __rsub__(self, other: Count) -> Count:
        return other + -self

    @abstractmethod
    def __mul__(self, other: Count) -> Count: ...

    @abstractmethod
    def __rmul__(self, other: int) -> Count: ...

    @abstractmethod
    def __mod__(self, mod: int) -> int: ...

    def __divmod__(self, other: int) -> tuple[Count, int]:
        if other == 1:
            return self, 0

        mod = self % other

        return (self - mod) // other, mod

    @abstractmethod
    def __floordiv__(self, other: Count) -> Count: ...

    def __rpow__(self, other: int) -> Exp | Tet:
        return make_exp(other, self)


def make_add(l: Count, r: Num) -> Add:
    if isinstance(l, Num) and l.depth > r.depth:  # no-cover
        l, r = r, l

    adds = ADDS[l]

    try:
        return adds[r]
    except KeyError:
        adds[r] = (add := Add(l, r))
        return add

class Add(Num):
    l: Count
    r: Num

    def __init__(self, l: Count, r: Num):
        self.l = l
        self.r = r

        self.depth = 1 + r.depth

        if isinstance(l, int):
            self.leaves = 1 + r.leaves
            self.tower_est = r.tower_est

        else:
            self.leaves = l.leaves + r.leaves
            self.tower_est = max(l.tower_est, r.tower_est)

    def __repr__(self) -> str:
        return f'({show_number(self.l)} + {self.r})'

    def __hash__(self) -> int:
        return id(self)

    def __contains__(self, other: Num) -> bool:
        return other == self or other in self.r

    def __int__(self) -> int:
        return int(self.l) + int(self.r)

    def digits(self) -> int:
        r_dig = self.r.digits()

        return (
            r_dig
            if isinstance(l := self.l, int) else
            max(l.digits(), r_dig)
        )

    def __mod__(self, mod: int) -> int:
        if mod == 1:  # no-cover
            return 0

        return ((self.l % mod) + (self.r % mod)) % mod

    def __neg__(self) -> Count:
        return -(self.l) + -(self.r)

    def __radd__(self, other: int) -> Count:
        if other == 0:
            return self

        if isinstance(l := self.l, int):
            return (l + other) + self.r

        return make_add(other, self)

    def __add__(self, other: Count) -> Count:
        l, r = self.l, self.r

        if isinstance(other, int):
            if other == 0:
                return self

            if not isinstance(l, int):  # no-cover
                return make_add(other, self)

            return (l + other) + r

        if isinstance(other, Add):
            lo, ro = other.l, other.r

            if isinstance(lo, int):
                return (lo + l) + (r + ro)

            if self == lo:  # no-cover
                return (2 * self) + ro

        if isinstance(l, int):
            return l + (other + r)

        if other.depth < l.depth <= 8:
            return (other + l) + r

        return make_add(self, other)

    def __sub__(self, other: Count) -> Count:
        if other == 0:
            return self

        if self == other:
            return 0

        if isinstance(other, Add):
            l, lo = self.l, other.l

            if isinstance(l, int) and isinstance(lo, int):
                return (l - lo) + (self.r - other.r)

        return self + -other

    def __mul__(self, other: Count) -> Count:
        if isinstance(other, int):
            return other * self

        return make_mul(self, other)

    def __rmul__(self, other: int) -> Count:
        match other:
            case 0:  # no-cover
                return 0

            case 1:
                return self

            case -1:
                return -self

            case _:
                return (other * self.l) + (other * self.r)

    def __floordiv__(self, other: Count) -> Count:
        if other == 1:
            return self

        assert isinstance(other, int)

        if (((lgcd := gcd(other, l := self.l)) == 1
                or (rgcd := gcd(other, r := self.r))) == 1
                or (div := gcd(lgcd, rgcd)) == 1):
            return make_div(self, other)

        return ((l // div) + (r // div)) // (other // div)

    def __lt__(self, other: Count) -> bool:
        l, r = self.l, self.r

        if isinstance(other, int):
            if isinstance(l, int):
                return r < 0

            if l < 0 and r < 0:  # no-cover
                return True

            if 0 < l and 0 < r:  # no-branch
                return False

        elif isinstance(other, Add):
            lo, ro = other.l, other.r

            if self == ro:  # no-cover
                return lo > 0

            if l == lo:
                return r < ro

            if r == ro:
                return l < lo

            if l == ro:  # no-cover
                return r < lo

            if isinstance(l, int) and isinstance(lo, int):
                return r < ro

            if l < lo and r < lo:  # no-branch
                return True

        if other == r:
            return l < 0

        if isinstance(l, int):
            if abs(l) < 10:
                return r < other

        elif other == l:
            return r < 0

        raise NotImplementedError(self, other)


def make_mul(l: Count, r: Num) -> Mul:
    if isinstance(l, Num) and l.depth > r.depth:
        l, r = r, l

    muls = MULS[l]

    try:
        return muls[r]
    except KeyError:
        muls[r] = (mul := Mul(l, r))
        return mul


class Mul(Num):
    l: Count
    r: Num

    def __init__(self, l: Count, r: Num):
        if l < 0:
            assert r > 0

        if r < 0:
            assert l > 0
            assert isinstance(l, Num)

        self.l = l
        self.r = r

        self.depth = 1 + r.depth

        if isinstance(l, int):
            self.leaves = 1 + r.leaves
            self.tower_est = r.tower_est

            assert isinstance(r, Exp), (l, r)

        else:
            self.leaves = l.leaves + r.leaves
            self.tower_est = max(l.tower_est, r.tower_est)

    def __repr__(self) -> str:
        if self.l == -1:
            return f'-{self.r}'

        return f'({show_number(self.l)} * {self.r})'

    def __hash__(self) -> int:
        return id(self)

    def __contains__(self, other: Num) -> bool:
        return other == self or other in self.r

    def __int__(self) -> int:
        return int(self.l) * int(self.r)

    def digits(self) -> int:
        r_dig = self.r.digits()

        return r_dig + (
            l.digits()
            if not isinstance(l := self.l, int) else
            round(log10(l))
            if l > 0 else
            -round(log10(-l))
        )

    @property
    def is_exp_coef(self) -> bool:
        return isinstance(self.r, Exp) and isinstance(self.l, int)

    def __neg__(self) -> Count:
        return -(self.l) * self.r

    def __mod__(self, mod: int) -> int:
        if mod == 1:  # no-cover
            return 0

        if (l_mod := self.l % mod) == 0:
            return 0

        if (r_mod := self.r % mod) == 0:
            return 0

        return (l_mod * r_mod) % mod

    def __mul__(self, other: Count) -> Count:
        if isinstance(other, int):
            return other * self

        l, r = self.l, self.r

        if l == -1:
            return -1 * (r * other)

        if isinstance(other, Exp):
            if other.multiplies_with(r):  # no-branch
                return l * (r * other)

        elif isinstance(other, Div):
            return (self * other.num) // other.den

        return (self.l * other) * self.r

    def __rmul__(self, other: int) -> Count:
        l, r = self.l, self.r

        if other == -1:
            if l == -1:  # no-cover
                return r

            if isinstance(l, int):
                return -l * r

            assert isinstance(l, Exp), self
            assert isinstance(r, Add), self

            return l * -r

        if other == 1:
            return self

        return (other * l) * r

    def __radd__(self, other: int) -> Count:
        if other == 0:
            return self

        return make_add(other, self)

    def __add__(self, other: Count) -> Count:
        if isinstance(other, int):
            return self if other == 0 else make_add(other, self)

        l, r = self.l, self.r

        if isinstance(other, Mul):
            lo, ro = other.l, other.r

            if r == ro:
                return (l + lo) * r

            if l != -1 and l == lo:
                if (not isinstance(nr := r + ro, Add)  # no-branch
                        or (nr.l != r and nr.l != ro)):  # noqa: PLR1714
                    return l * nr

            if l == ro:
                return l * (r + lo)

            if r == lo:
                return (l + ro) * r

            if isinstance(r, Exp):
                if isinstance(ro, Exp):
                    assert r.base == ro.base

                    try:
                        return add_exponents((r, l), (ro, lo))
                    except NotImplementedError:
                        pass

                if isinstance(lo, Exp):
                    assert r.base == lo.base

                    try:
                        return add_exponents((r, l), (lo, ro))
                    except NotImplementedError:  # no-cover
                        pass

        elif isinstance(other, Add):
            lo, ro = other.l, other.r

            if isinstance(lo, int):
                return lo + (ro + self)

            if l != -1:  # no-branch
                if isinstance(lo, Mul):  # no-branch
                    if lo.l == l:
                        return (self + lo) + ro

                if isinstance(ro, Mul):
                    if ro.l == l:
                        return lo + (self + ro)

            if lo.depth < self.depth:
                return lo + (self + ro)

        elif isinstance(other, Exp):  # noqa: SIM114
            return other + self

        elif isinstance(other, Div):
            return other + self

        if l == -1 and other == r:  # no-cover
            return 0

        return make_add(self, other)

    def __sub__(self, other: Count) -> Count:
        l, r = self.l, self.r

        if isinstance(other, Mul):
            lo, ro = other.l, other.r

            if self == other:
                return 0

            if l == lo:
                return l * (r - ro)

            if r == ro:
                return (l - lo) * r

        elif isinstance(other, int):
            if other == 0:
                return self

            return -other + self

        elif isinstance(other, Add):
            if self == other.r:  # no-branch
                return -(other.l)

        if other == l:
            return l * (r - 1)

        return self + -other

    def __floordiv__(self, other: Count) -> Count:
        if other == 1:
            return self

        assert isinstance(other, int)

        l, r = self.l, self.r

        if (lgcd := gcd(other, l)) > 1:
            return ((l // lgcd) * r) // (other // lgcd)

        if (rgcd := gcd(other, r)) > 1:
            return (l * (r // rgcd)) // (other // rgcd)

        return make_div(self, other)

    def __lt__(self, other: Count) -> bool:
        l, r = self.l, self.r

        if isinstance(other, int):
            return l < 0

        if isinstance(other, Mul):
            lo, ro = other.l, other.r

            if l == lo:
                return r < ro

            if r == ro:
                return l < lo

            if l == ro:  # no-cover
                return r < lo

            if r == lo:  # no-cover
                return l < ro

            if self.is_exp_coef and other.is_exp_coef:
                # pylint: disable = no-else-return
                assert isinstance(l, int)
                assert isinstance(lo, int)
                assert isinstance(r, Exp)
                assert isinstance(ro, Exp)

                assert (base := r.base) == ro.base

                if (rexp := r.exp) == (roexp := ro.exp):  # no-ranch
                    return l < lo  # no-cover

                if rexp < roexp:
                    if not isinstance(diff := roexp - rexp, int):  # no-branch
                        return True  # no-cover

                    return ceil(l / lo) <= (base ** diff)  # type: ignore[no-any-return]

                else:  # noqa: RET505
                    if not isinstance(diff := rexp - roexp, int):  # no-branch
                        return False  # no-cover

                    return ceil(lo / l) > (base ** diff)  # type: ignore[no-any-return]

        if l < 0:
            if other < 0:  # no-cover
                raise NotImplementedError(self, other)

            return True

        try:
            if (other <= l and 0 < r) or (other <= r and 0 < l):
                return False
        except NotImplementedError:
            pass

        if (l < other and r < 0) or (r < other and l < 0):
            return True

        if isinstance(other, Exp):
            if 0 < l < 10:
                return r < other

        if 0 < l and isinstance(r, Add) and r.l == -1 and r.r == other:
            return False

        return super().__lt__(other)


def make_div(num: Num, den: int) -> Div:
    divs = DIVS[num]

    try:
        return divs[den]
    except KeyError:
        divs[den] = (div := Div(num, den))
        return div


class Div(Num):
    num: Num
    den: int

    def __init__(self, num: Num, den: int):
        assert den > 0

        self.num = num
        self.den = den

        self.depth = 1 + num.depth

        self.leaves = 1 + num.leaves

        self.tower_est = num.tower_est

    def __repr__(self) -> str:
        return f'({self.num} // {self.den})'

    def __hash__(self) -> int:
        return id(self)

    def __contains__(self, other: Num) -> bool:
        return other == self or other in self.num

    def __int__(self) -> int:
        return int(self.num) // self.den

    def __neg__(self) -> Count:
        return -(self.num) // self.den

    def __mod__(self, mod: int) -> int:
        if mod == 1:
            return 0

        if self.num.depth > 200:
            raise ModDepthLimit(
                f'{self} % {mod}')

        div, rem = divmod(
            self.num % (mod * self.den),
            self.den)

        assert rem == 0

        return div % mod

    def digits(self) -> int:
        return self.num.digits() - round(log10(self.den))

    def __add__(self, other: Count) -> Count:
        if other == 0:
            return self

        num, den = self.num, self.den

        if not isinstance(other, Div):
            return (num + (other * den)) // den

        if den == (oden := other.den):
            return (num + other.num) // den

        if den < oden:
            return other + self

        if pgcd(den, oden) > 1:
            return (num + ((den // oden) * other.num)) // den

        return ((oden * num) + (den * other.num)) // (den * oden)

    def __radd__(self, other: int) -> Count:
        if other == 0:
            return self

        return ((other * self.den) + self.num) // self.den

    def __sub__(self, other: Count) -> Count:
        if other == 0:
            return self

        if self == other:
            return 0

        if isinstance(other, Div) and (den := self.den) == other.den:
            return (self.num - other.num) // den

        assert isinstance(other, int)

        return -other + self

    def __mul__(self, other: Count) -> Count:
        num, den = self.num, self.den

        if isinstance(other, int):
            if other == den:
                return num

            if (cden := pgcd(den, other)) > 1:
                return ((other // cden) * num) // (den // cden)

        return (num * other) // den

    def __rmul__(self, other: int) -> Count:
        match other:
            case 0:  # no-cover
                return 0

            case 1:
                return self

            case _ if 0 < other:
                return self * other

            case _:
                return -(self * -other)

    def __floordiv__(self, other: Count) -> Count:
        if other == 1:
            return self

        assert isinstance(other, int)

        num, den = self.num, self.den

        if (cden := gcd(other, num)) == 1:
            return make_div(num, other * den)

        return (num // cden) // ((other // cden) * den)

    def __lt__(self, other: Count) -> bool:
        if isinstance(other, int):
            return self.num < 0

        if not isinstance(other, Div):
            raise NotImplementedError(self, other)

        assert self.den == other.den, (self, other)

        return self.num < other.num


def make_exp(base: int, exp: Count) -> Exp:
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

    exps = EXPS[base]

    try:
        return exps[exp]
    except KeyError:
        exps[exp] = (exp_expr := Exp(base, exp))
        return exp_expr


class Exp(Num):
    base: int
    exp: Count

    def __init__(self, base: int, exp: Count):
        self.base = base
        self.exp = exp

        if isinstance(exp, int):
            self.depth = 1
            self.leaves = 2
            self.tower_est = 2 if log10(exp) >= 1 else 1

        else:
            self.depth = 1 + exp.depth
            self.leaves = 1 + exp.leaves
            self.tower_est = 1 + exp.tower_est

    def __repr__(self) -> str:
        return f'({self.base} ** {show_number(self.exp)})'

    def __hash__(self) -> int:
        return id(self)

    def __contains__(self, other: Num) -> bool:
        return (
            other == self
            or (not isinstance(exp := self.exp, int)
                   and other in exp)
        )

    def __neg__(self) -> Count:
        return -1 * self

    def __int__(self) -> int:
        return self.base ** int(self.exp)  # type: ignore[no-any-return]

    def digits(self) -> int:
        if not isinstance(exp := self.exp, int):
            if exp.digits() >= 10:
                raise OverflowError

            exp = int(exp)

        return round(log10(self.base) * 10 ** log10(exp))

    def __mod__(self, mod: int) -> int:  # noqa: PLR0912
        if mod == 1:
            return 0

        base = self.base

        if mod == base:
            return 0

        if mod == 2:
            return base % 2

        assert base % mod != 0

        exp = self.exp

        assert 1 < exp

        match base:
            case 2:
                match mod:
                    case 4:
                        return 0

                    case 6:
                        return 4 if exp % 2 == 0 else 2

                    case 12:
                        return 4 if exp % 2 == 0 else 8

                    case 30:
                        match exp % 4:
                            case 3:
                                return 8
                            case 0:  # no-cover
                                return 15
                            case 1:  # no-cover
                                return 2
                            case 2:  # no-cover
                                return 4

            case 3:
                if mod == 6:
                    return 3

                if int(log_mod := log2(mod)) == log_mod:  # no-branch
                    exp %= int(2 ** (int(log_mod) - 2))

            case 6:
                if mod == 10:  # no-branch
                    return 6

            case 7:
                if mod == 12:  # no-branch
                    return 1 if exp % 2 == 0 else 7

        if (period := find_period(base, mod)) > 0:
            exp %= period

        if exp == 0:
            return 1

        if not isinstance(exp, int):
            return exp_mod_special_cases(mod, base, exp)

        res = 1

        for _ in itertools.count():
            if exp <= 0 or res <= 0:
                break

            if (exp % 2) == 1:
                res = (res * base) % mod

            exp //= 2

            base = (base ** 2) % mod

        return res

    def __radd__(self, other: int) -> Count:
        if other == 0:
            return self

        return make_add(other, self)

    def __add__(self, other: Count) -> Count:
        if isinstance(other, int):
            return self if other == 0 else make_add(other, self)

        if isinstance(other, Mul):
            l, r = other.l, other.r

            base = self.base

            if isinstance(r, Exp):
                assert r.base == base

                try:
                    return add_exponents((self, 1), (r, l))
                except NotImplementedError:
                    pass

            if isinstance(l, Exp):
                assert l.base == base

                try:
                    return add_exponents((self, 1), (l, r))
                except NotImplementedError:  # no-cover
                    pass

        elif isinstance(other, Exp):
            assert (base := self.base) == other.base

            if base == 2:
                sexp, oexp = self.exp, other.exp

                if sexp == oexp:
                    return make_exp(2, 1 + sexp)

            try:
                return add_exponents((self, 1), (other, 1))
            except NotImplementedError:
                pass

        elif isinstance(other, Div):
            return other + self

        elif isinstance(other, Add):  # no-branch
            if isinstance(other.l, int):
                return other.l + (self + other.r)

        return make_add(self, other)

    def __sub__(self, other: Count) -> Count:
        if other == 0:
            return self

        if self == other:
            return 0

        if isinstance(other, Exp):
            assert self.base == other.base

            return add_exponents((self, 1), (other, -1))

        if isinstance(other, int):
            return make_add(-other, self)

        return self + -other

    def multiplies_with(self, other: Count) -> bool:
        if isinstance(other, Exp):
            return other.base == self.base

        if isinstance(other, Mul):  # no-cover
            return (
                self.multiplies_with(other.l)
                or self.multiplies_with(other.r)
            )

        return False

    def __mul__(self, other: Count) -> Count:
        if isinstance(other, int):
            return other * self

        if isinstance(other, Exp):
            assert (base := self.base) == other.base

            return make_exp(base, self.exp + other.exp)

        if isinstance(other, Add):
            return (self * other.l) + (self * other.r)

        assert isinstance(other, Mul), (self, other)

        l, r = other.l, other.r

        if isinstance(l, int):
            return l * (self * r)

        if self.multiplies_with(l):
            return (self * l) * r

        if self.multiplies_with(r):  # no-branch
            return l * (self * r)

        return make_mul(self, other) # no-cover

    def __rmul__(self, other: int) -> Count:
        if other == 0:
            return 0

        if other == 1:
            return self

        if other == -1:
            return make_mul(-1, self)

        base = self.base

        if other < -1 and -other % base == 0:
            return -(-other * self)

        if other % base != 0:
            return make_mul(other, self)

        exp = self.exp

        for i in itertools.count():
            if other % base != 0:
                exp += i
                break

            other //= base

        return other * make_exp(base, exp)

    def __floordiv__(self, other: Count) -> Count:
        if other == 1:
            return self

        base, exp = self.base, self.exp

        if not isinstance(other, int):
            assert isinstance(other, Exp)
            assert other.base == base
            assert other.exp <= exp, (self, other)

            match (diff := exp - other.exp):
                case 0:
                    return 1
                case 1:
                    return base
                case _:
                    return make_exp(base, diff)

        for i in itertools.count():
            if other % base != 0:
                exp -= i
                break

            other //= base

        if other > 1:
            assert base > other
            assert base % other == 0

            return (base // other) * make_exp(base, exp - 1)

        return (
            1 if exp == 0 else
            base if exp == 1 else
            make_exp(base, exp)
        )

    def __lt__(self, other: Count) -> bool:
        if isinstance(other, int):
            return False

        base, exp = self.base, self.exp

        if isinstance(other, Exp):
            assert base == other.base
            return exp < other.exp

        if isinstance(other, Add):
            l, r = other.l, other.r

            if isinstance(l, int):  # no-branch
                return self < r

        elif isinstance(other, Mul):
            l, r = other.l, other.r

            if isinstance(l, Exp):
                assert l.base == base

                if l.exp <= exp:
                    return (self // l) < r

            if isinstance(r, Exp):
                assert r.base == base

                if r.exp <= exp:
                    return (self // r) < l

            try:
                if self <= r and l > 0:
                    return True
            except NotImplementedError:  # no-cover
                pass

            try:
                if self <= l and r > 0:  # no-branch
                    return True
            except NotImplementedError:  # no-cover
                pass

            if self == r:  # no-cover
                return 0 < l

            if self == l:  # no-cover
                return 0 < r

        elif isinstance(other, Tet):  # no-branch
            return other > self

        raise NotImplementedError(self, other)  # no-cover

    def __pow__(self, other: Count) -> Exp:
        return make_exp(self.base, self.exp * other)


class Tet(Num):
    base: int
    height: int

    def __init__(self, base: int, height: int):
        self.base = base
        self.height = height

        self.depth = 1
        self.leaves = 2
        self.tower_est = height

    def __repr__(self) -> str:
        return f'({self.base} ↑↑ {self.height})'

    def __hash__(self) -> int:
        return id(self)

    def digits(self) -> int:  # noqa: PLR6301
        raise OverflowError

    def estimate(self) -> Tet:
        return self

    def __neg__(self) -> Count:
        raise NotImplementedError

    def __int__(self) -> int:
        raise NotImplementedError

    def __mod__(self, mod: int) -> int:
        raise NotImplementedError

    def __eq__(self, other: object) -> bool:
        return (
            isinstance(other, Tet)
            and self.base == other.base
            and self.height == other.height
        )

    def __rpow__(self, other: int) -> Exp | Tet:
        if other != self.base:
            return make_exp(other, self)

        return Tet(self.base, 1 + self.height)

    def __lt__(self, other: Count) -> bool:
        if isinstance(other, int):
            return False

        if isinstance(other, Tet):
            if not self.base == other.base:
                raise NotImplementedError(self, other)

            return self.height < other.height

        assert isinstance(other, Exp)

        if isinstance(other.exp, int) and 2 < self.height:
            return False

        raise NotImplementedError(self, other)

    def __add__(self, other: Count) -> Count:
        return self

    def __sub__(self, other: Count) -> Count:
        if other == self:
            return 0

        return self

    def __radd__(self, other: int) -> Count:
        return self

    def __mul__(self, other: Count) -> Count:
        return self

    def __rmul__(self, other: int) -> Count:
        raise NotImplementedError

    def __floordiv__(self, other: Count) -> Count:
        raise NotImplementedError


def add_exponents(
        l: tuple[Exp, Count],
        r: tuple[Exp, Count],
) -> Count:
    (l_exp, l_co), (r_exp, r_co) = l, r

    if l_exp == r_exp:
        return (l_co + r_co) * l_exp

    assert (base := l_exp.base) == r_exp.base

    if l_exp.exp > r_exp.exp:
        (l_exp, l_co), (r_exp, r_co) = (r_exp, r_co), (l_exp, l_co)

    if not (l_pow := l_exp.exp) <= (r_pow := r_exp.exp):
        raise NotImplementedError(l_pow, r_pow)

    diff_exp = (
        base ** diff
        if (diff := r_pow - l_pow) < 1_000 else
        make_exp(base, diff)
    )

    return (l_co + (r_co * diff_exp)) * l_exp


def gcd(l: int, r: Count) -> int:
    if l == 1:  # no-cover
        return 1

    if isinstance(r, int):
        return pgcd(l, r)

    if isinstance(r, Add):
        if (lgcd := gcd(l, r.l)) == 1:
            return 1

        if (rgcd := gcd(l, r.r)) == 1:
            return 1

        return min(lgcd, rgcd)

    if isinstance(r, Mul):
        return max(gcd(l, r.l), gcd(l, r.r))

    assert isinstance(r, Exp), (l, r)

    if l == (base := r.base):
        return l

    if l % base != 0:
        return pgcd(l, base)

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
    if base == 2 and mod == 2 * (3 ** round(log(mod / 2, 3))):
        return 0

    val = 1

    if mod >= 2 ** 24:
        raise PeriodLimit(f'{base} ** ... % {mod}')

    for period in range(1, mod):
        val *= base
        val %= mod

        if val == 1:
            return period

    return 0


def exp_mod_special_cases(mod: int, base: int, exp: Num) -> int:
    if base != 2 or 2 * (3 ** round(log(mod / 2, 3))) != mod: # no-cover
        raise ExpModLimit(
            f'({base} ** {exp}) % {mod}')

    period = exp % (mod // 3)

    match mod:
        case 54:
            values = {
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
            }
        case 162:
            values = {
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
            }
        case 486:
            values = {
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
            }
        case 1458:
            values = {
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
            }
        case 4374:
            values = {
                267: 4364,
                296: 4090,
                655: 614,
                695: 1994,
                696: 3988,
                870: 1846,
                896: 2398,
                919: 326,
                989: 2768,
                1004: 2560,
                1231: 1532,
                1258: 3004,
                1307: 104,
            }
        case 13122:
            values = {
                296: 12838,
                919: 4700,
                1231: 5906,
                1258: 11752,
                2354: 6772,
                2447: 2768,
                2462: 2560,
                2765: 104,
                3183: 8738,
                3571: 4988,
                3611: 6368,
                3612: 12736,
            }
        case 39366:
            values = {
                919: 4700,
                2447: 2768,
                2765: 26348,
                3571: 4988,
                4670: 39082,
                5632: 11752,
                6836: 28804,
                7986: 25858,
                9979: 32150,
                11102: 19894,
                11931: 8738,
            }
        case 118098:
            values = {
                2447: 2768,
                3571: 44354,
                5632: 11752,
                6836: 68170,
                27163: 44066,
                29009: 26348,
                30914: 39082,
                34230: 25858,
                38175: 48104,
                36223: 110882,
            }
        case 354294:
            values = {
                6836: 68170,
                29009: 144446,
                30914: 157180,
                41813: 238964,
                42937: 162452,
                73596: 143956,
                75589: 110882,
                84364: 247948,
                105895: 162164,
            }
        case 1062882:
            values = {
                29009: 498740,
                84364: 956536,
                149012: 511474,
                223993: 516458,
                243032: 422464,
                278009: 238964,
                309792: 143956,
                311785: 110882,
            }
        case 3188646:
            values = {
                223993: 516458,
                309792: 2269720,
                597326: 422464,
                632303: 2364728,
                737597: 1561622,
                792952: 2019418,
                857600: 511474,
            }
        case 9565938:
            values = {
                 223993:  516458,
                 737597: 4750268,
                 857600: 3700120,
                1660208:  422464,
                1855834: 5208064,
                2435556: 5458366,
                2758067: 8742020,
            }
        case 28697814:
            values = {
                 857600:  3700120,
                1660208:  9988402,
                2758067:  8742020,
                7114889: 23882144,
                8233126:  5208064,
                8812848: 24590242,
            }
        case 86093442:
            values = {
                 2758067: 66137648,
                 7114889: 23882144,
                19989476: 61095748,
                20792084: 67384030,
                27944724: 81985870,
            }
        case 258280326:
            values = {
                7114889: 109975586,
                27944724: 168079312,
                77385104: 233282632,
                78187712: 153477472,
            }
        case 774840978:
            values = {
                7114889: 626536238,
                114038166: 684639964,
                250374596: 153477472,
            }
        case 2324522934:
            values = {
                265395215: 626536238,
                766935248: 153477472,
            }
        case 6973568802:  # no-branch
            values = {
                1541776226: 153477472,
            }
        case _:  # no-cover
            raise ExpModLimit(f'({base} ** {exp}) % {mod}')

    try:
        return values[period]
    except KeyError:
        raise ExpModLimit(f'({base} ** {exp}) % {mod}')  # noqa: B904

########################################

TRUNCATE_COUNT: Final[int] = 10 ** 12

MAX_LEAVES: Final[int] = 120

def show_number(num: Count) -> str:
    if isinstance(num, int):
        if abs(num) >= TRUNCATE_COUNT:
            return "{}(~10^{:.0f})".format(
                '-' if num < 0 else '',
                log10(abs(num)),
            )

    elif num.leaves > MAX_LEAVES:  # no-cover
        return "(???)"

    return str(num)
