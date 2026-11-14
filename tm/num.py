# ruff:file-ignore[collapsible-if, too-many-return-statements]

import itertools
from abc import abstractmethod
from collections import defaultdict
from functools import cache, cached_property
from math import ceil, floor, log, log2, log10, sqrt
from math import gcd as pgcd
from typing import ClassVar, Final, Never, Self

from tm.show import show_number

MAX_LEAVES: Final[int] = 120

########################################

type Count = int | Num

########################################

class NumError(Exception):
    pass

class PeriodLimit(Exception):
    def __init__(self, base: int, mod: int):
        super().__init__(
            f'{base} ** ... % {mod}')

def raise_lt_not_implemented(l: Count, r: Count) -> Never:
    raise NotImplementedError(
        f'{type(l).__name__}.__lt__: {l} < {r}')

########################################

class Num:
    depth: int

    leaves: int

    @abstractmethod
    def __int__(self) -> int: ...

    def __contains__(self, other: Num) -> bool:
        return False

    @abstractmethod
    def __hash__(self) -> int: ...

    def estimate(self) -> Arrow:
        return estimate_count(self)

    @abstractmethod
    def digits(self) -> int: ...

    @cached_property
    def pos(self) -> bool:
        return 0 < self

    @cached_property
    def neg(self) -> bool:
        return self < 0

    @abstractmethod
    def __neg__(self) -> Count: ...

    def __abs__(self) -> Count:
        return -self if self.neg else self

    def __eq__(self, other: object) -> bool:
        return other is self

    def __lt__(self, other: Count) -> bool:
        assert not isinstance(other, int)
        assert self.pos
        assert other.pos

        if isinstance(other, Add | Mul):
            l, r = other.l, other.r

            if self <= r and l > 0:
                return True

            if self == r:
                return 0 < l

            assert self != l

            if (isinstance(other, Add)
                    and isinstance(l, int)
                    and abs(l) < 10):
                return self < r

        raise_lt_not_implemented(self, other)

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

    def __rpow__(self, other: int) -> Exp | Arrow:
        return Exp.make(other, self)


def estimate_digits(num: Num) -> Arrow | None:
    try:
        digits = num.digits()
    except (OverflowError, ValueError):
        return None

    return Arrow(10, 1, max(1, digits))


def _estimate_arrow(base: int, arrows: int, height: Count) -> Arrow:
    if not isinstance(height, Arrow) or height.base != base:
        return Arrow(base, arrows, height)

    match arrows, height.arrows, height.height:
        case _, level, h if level == arrows + 1:
            return Arrow(base, level, h + 1)
        case 1, 1, int() as h if h >= base:
            return Arrow(base, 2, 4 if h >= base ** 6 else 3)
        case 2, 1, int() as h if h >= base:
            return Arrow(base, 3, 3)
        case 2, 1, Num():  # no-cover
            return Arrow(base, 3, 3)
        case 2, 2, int() as h if h >= 3:
            return Arrow(base, 3, 3)

    n = 1
    tail: Count = height
    while (n < 4 and isinstance(tail, Arrow)
           and tail.base == base and tail.arrows == arrows):
        n += 1
        tail = tail.height

    if n < 4:
        return Arrow(base, arrows, height)

    match tail:
        case Arrow() as next_ if next_.base == base and next_.arrows == arrows + 1:  # no-cover
            return Arrow(base, arrows + 1, next_.height + n)
        case Arrow() as next_ if next_.base == base and next_.arrows > arrows + 1:  # no-cover
            return next_
        case _:
            return Arrow(base, arrows + 1, n + 1)


def _estimate_exp(exponent: Count, done: dict[Num, Arrow]) -> Arrow:
    match exponent:
        case int() as n if n <= 0:  # no-cover
            return Arrow(10, 1, 1)
        case int() as n:  # no-cover
            return _estimate_arrow(10, 1, Arrow(10, 1, max(1, round(log10(n)))))
        case Num() as exp:  # no-branch
            est = done[exp]
            match est.arrows:
                case 1:
                    result = _estimate_arrow(10, 1, est)
                    return Arrow(10, 2, 3) if result.arrows == 2 and result.height == 4 else result
                case 2:
                    return _estimate_arrow(10, 2, est.height + 1)
                case _:
                    return est


def _estimate_opseq(times: Count, ops: tuple[tuple[str, int], ...], done: dict[Num, Arrow]) -> Arrow:
    levels = sum(1 if op == '**' else -1 if op == '~' else 0 for op, _ in ops)
    height = times if isinstance(times, int) else done[times]

    if levels == 0:  # no-cover
        if any(op == '**' for op, _ in ops):
            return _estimate_arrow(10, 2, height)

        if any(op == '*' and val > 1 for op, val in ops):
            return _estimate_arrow(10, 1, max(1, height) if isinstance(height, int) else height)

        if not isinstance(times, int):
            return done[times]

        return Arrow(10, 1, max(1, round(log10(max(1, times)))))

    if isinstance(times, int):
        height = max(1, levels * times)

    return _estimate_arrow(10, 2, height)



def _exp_chain_height(num: Count) -> int | None:
    levels = 0

    while True:
        match num:
            case Exp(exp=exp):
                levels += 1
                num = exp
            case Add(l=int(), r=child) | Mul(l=int(), r=child) | Div(num=child):
                num = child
            case int() as n:
                return levels + (n >= 10)
            case _:
                return None


def _estimate_resolved(num: Num, done: dict[Num, Arrow]) -> Arrow:
    from tm.rules import OpSeqResult  # ruff: ignore[import-outside-top-level]

    match num:
        case Add(l=int(), r=int_rhs) | Mul(l=int(), r=int_rhs):
            return done[int_rhs]
        case Add(l=Num() as num_lhs, r=num_rhs) | Mul(l=Num() as num_lhs, r=num_rhs):
            try:
                return max(done[num_rhs], done[num_lhs])
            except NotImplementedError:
                return done[num_rhs]
        case Div(num=div_num):
            return done[div_num]
        case Exp(exp=exponent) as exp_num:
            result = _estimate_exp(exponent, done)
            if (result.arrows == 2 and isinstance(exponent, Num)
                    and done[exponent].arrows == 1
                    and (chain_height := _exp_chain_height(exp_num)) is not None):
                assert isinstance(result.height, int)
                return Arrow(10, 2, max(result.height, chain_height))
            return result
        case Arrow(base=arrow_base, arrows=arrow_arrows, height=arrow_height):
            estimated = arrow_height if isinstance(arrow_height, int) else done[arrow_height]
            result = _estimate_arrow(arrow_base, arrow_arrows, estimated)
            return num if (result.base == arrow_base and result.arrows == arrow_arrows
                           and estimated is arrow_height) else result
        case OpSeqResult(times=seq_times, opseq=seq_ops):  # no-branch
            return _estimate_opseq(seq_times, seq_ops, done)
        case _:  # no-cover
            raise AssertionError(type(num).__name__)


def estimate_count(root: Num) -> Arrow:
    from tm.rules import OpSeqResult  # ruff: ignore[import-outside-top-level]

    done: dict[Num, Arrow] = {}
    visiting: set[Num] = set()
    stack: list[tuple[Num, bool]] = [(root, False)]

    while stack:
        num, ready = stack.pop()
        if num in done:  # no-cover
            continue

        if ready:
            visiting.remove(num)
            done[num] = _estimate_resolved(num, done)
            continue

        if (isinstance(num, Add | Mul | Div | Exp)
                and num.depth < 64
                and (quick := estimate_digits(num)) is not None):
            done[num] = quick
            continue

        assert num not in visiting

        visiting.add(num)
        stack.append((num, True))

        children: tuple[Count, ...]
        match num:
            case Add(l=l, r=r) | Mul(l=l, r=r):
                children = (l, r)
            case Div(num=child) | Exp(exp=child) | Arrow(height=child) | OpSeqResult(times=child):  # no-branch
                children = (child,)
            case _:  # no-cover
                raise NotImplementedError(type(num).__name__)

        stack.extend(
            (child, False) for child in reversed(children)
            if isinstance(child, Num) and child not in done
        )

    result = done[root]

    return (
        Arrow(result.base, 2, 2)
        if (result.arrows == 1
            and isinstance(height := result.height, Arrow)
            and height.base == result.base
            and height.arrows == 1
            and height.height == 1)
        else result
    )


class Add(Num):
    l: Count
    r: Num

    instances: ClassVar[
        dict[
            Count,
            dict[Num, Self],
        ]
    ] = defaultdict(dict)

    @staticmethod
    def make(l: Count, r: Num) -> Add:
        if isinstance(l, Num) and l.depth > r.depth:
            l, r = r, l

        adds = Add.instances[l]

        try:
            return adds[r]
        except KeyError:
            adds[r] = (add := Add(l, r))
            return add

    def __init__(self, l: Count, r: Num):
        self.l = l
        self.r = r

        self.depth = 1 + r.depth

        self.leaves = (1 if isinstance(l, int) else l.leaves) + r.leaves

    def __repr__(self) -> str:
        l, r = self.l, self.r

        if isinstance(l, int):
            return f'({show_number(l)} + {r})'

        if MAX_LEAVES < self.leaves:
            return '(... + ...)'

        return f'({l} + {r})'

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
        assert mod != 1

        return ((self.l % mod) + (self.r % mod)) % mod

    def __neg__(self) -> Count:
        return -(self.l) + -(self.r)

    def __radd__(self, other: int) -> Count:
        if other == 0:
            return self

        if isinstance(l := self.l, int):
            return (l + other) + self.r

        return Add.make(other, self)

    def __add__(self, other: Count) -> Count:
        l, r = self.l, self.r

        if isinstance(other, int):
            if other == 0:
                return self

            if not isinstance(l, int):
                return Add.make(other, self)

            return (l + other) + r

        if isinstance(other, Add):
            lo, ro = other.l, other.r

            if isinstance(lo, int):
                return (lo + l) + (r + ro)

            if self == lo:
                return (2 * self) + ro

            if self == ro:  # no-cover
                return lo + (2 * self)

            if l == -lo:
                return r + ro

            assert r != -ro

        if isinstance(l, int):
            return l + (other + r)

        if r == other:
            return l + (2 * r)

        if l == other:
            return (2 * l) + r

        return Add.make(self, other)

    def __sub__(self, other: Count) -> Count:
        if other == 0:
            return self

        if self == other:  # no-cover
            return 0

        if isinstance(other, Add):
            l, lo = self.l, other.l

            if isinstance(l, int) and isinstance(lo, int):
                return (l - lo) + (self.r - other.r)

        return self + -other

    def __mul__(self, other: Count) -> Count:
        if isinstance(other, int):
            return other * self

        assert isinstance(self.l, int)
        assert isinstance(other, Exp)

        return Mul.make(self, other)

    def __rmul__(self, other: int) -> Count:
        match other:
            case 0:
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
            return Div.make(self, other)

        return ((l // div) + (r // div)) // (other // div)

    def __lt__(self, other: Count) -> bool:
        l, r = self.l, self.r

        if isinstance(other, int):
            if isinstance(l, int):
                return r.neg

            if l.neg and r.neg:
                return True

            if l.pos and r.pos:
                return False

            if other == 0:
                if l.neg and r.pos:
                    return r < -l

                if r.neg and l.pos:  # no-branch
                    return l < -r

        elif isinstance(other, Add):
            lo, ro = other.l, other.r

            if self == ro:
                return lo > 0

            if l == lo:
                return r < ro

            if r == ro:
                return l < lo

            if l < lo and r < ro:
                return True

        if other == r:
            return l < 0

        if isinstance(l, int):
            if abs(l) < 10:
                return r < other

        elif other == l:
            return r.neg

        raise_lt_not_implemented(self, other)


class Mul(Num):
    l: Count
    r: Num

    instances: ClassVar[
        dict[
            Count,
            dict[Num, Self],
        ]
    ] = defaultdict(dict)

    @staticmethod
    def make(l: Count, r: Num) -> Mul:
        if not isinstance(l, int) and l.depth > r.depth:
            l, r = r, l

            if r.neg:
                l, r = -l, -r  # type: ignore[assignment]  # ty: ignore[invalid-assignment]

        assert r.pos

        muls = Mul.instances[l]

        try:
            return muls[r]
        except KeyError:
            muls[r] = (mul := Mul(l, r))
            return mul

    def __init__(self, l: Count, r: Num):
        assert r.pos

        self.l = l
        self.r = r

        self.depth = 1 + r.depth

        self.leaves = (1 if isinstance(l, int) else l.leaves) + r.leaves

    def __repr__(self) -> str:
        l, r = self.l, self.r

        if isinstance(l, int):
            if l == -1:
                return f'-{self.r}'

            return f'({show_number(l)} * {r})'

        if MAX_LEAVES < self.leaves:  # no-cover
            return '(... * ...)'

        return f'({l} * {r})'

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

    def __neg__(self) -> Count:
        return -(self.l) * self.r

    def __mod__(self, mod: int) -> int:
        assert mod != 1

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
            assert l != -1

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

        return Add.make(other, self)

    def __add__(self, other: Count) -> Count:
        if isinstance(other, int):
            return self if other == 0 else Add.make(other, self)

        l, r = self.l, self.r

        if isinstance(other, Mul):
            lo, ro = other.l, other.r

            if r == ro:
                return (l + lo) * r

            if l != -1 and l == lo:
                if (not isinstance(nr := r + ro, Add)  # no-branch
                        or (nr.l != r and nr.l != ro)):  # ruff:ignore[repeated-equality-comparison]
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
                    return add_exponents((r, l), (lo, ro))

        elif isinstance(other, Add):
            lo, ro = other.l, other.r

            if isinstance(lo, int):
                return lo + (ro + self)

            if l != -1:
                if isinstance(lo, Mul):
                    if lo.l == l:
                        return (self + lo) + ro

                if isinstance(ro, Mul):
                    if ro.l == l:
                        return lo + (self + ro)

        elif isinstance(other, Exp):  # ruff:ignore[if-with-same-arms]
            return other + self

        elif isinstance(other, Div):
            return other + self

        if l == -1 and other == r:  # no-cover
            return 0

        return Add.make(self, other)

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
            if self == other.r:
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

        return Div.make(self, other)

    def __lt__(self, other: Count) -> bool:
        l, r = self.l, self.r

        if isinstance(other, int):
            assert r.pos
            return l < 0

        if isinstance(other, Mul):
            lo, ro = other.l, other.r

            if l == lo:
                return r < ro

            if r == ro:
                return l < lo

        if l < 0 and other.pos:
            return True

        if other <= l or (other <= r and 0 < l):
            return False

        if isinstance(other, Exp):
            if 0 < l < 10:
                return r < other

        return super().__lt__(other)


class Div(Num):
    num: Num
    den: int

    instances: ClassVar[
        dict[
            Num,
            dict[int, Self],
        ]
    ] = defaultdict(dict)

    @staticmethod
    def make(num: Num, den: int) -> Div:
        divs = Div.instances[num]

        try:
            return divs[den]
        except KeyError:
            divs[den] = (div := Div(num, den))
            return div

    def __init__(self, num: Num, den: int):
        assert den > 0

        self.num = num
        self.den = den

        self.depth = 1 + num.depth

        self.leaves = 1 + num.leaves

    def __repr__(self) -> str:
        num = (
            '...'
            if MAX_LEAVES < self.leaves else
            str(self.num)
        )

        return f'({num} // {self.den})'

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

        num, den = self.num, self.den

        if (inv := inv_mod(den, mod)) is not None:
            return (inv * (num % mod)) % mod

        div, rem = divmod(num % (mod * den), den)

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
        assert other != 0

        match other:
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
            return Div.make(num, other * den)

        return (num // cden) // ((other // cden) * den)

    def __lt__(self, other: Count) -> bool:
        if isinstance(other, int):
            return self.num.neg

        if not isinstance(other, Div):
            raise_lt_not_implemented(self, other)

        den, deno = self.den, other.den

        if den > deno:
            div, mod = divmod(den, deno)

            assert mod == 0

            return self < div * other

        if den < deno:
            div, mod = divmod(deno, den)

            assert mod == 0

            return div * self < other

        assert self.den == other.den

        return self.num < other.num


class Exp(Num):
    base: int
    exp: Count

    instances: ClassVar [
        dict[
            int,
            dict[Count, Self],
        ]
    ] = defaultdict(dict)

    @staticmethod
    def make(base: int, exp: Count) -> Exp:
        for _ in itertools.count():
            if not isinstance(base, int) or base <= 1:
                break

            if base == 8:
                base = 2
                exp *= 3
                break

            if base == 27:
                base = 3
                exp *= 3
                break

            if floor(root := sqrt(base)) != ceil(root):
                break

            exp *= int(log(base, root))
            base = int(root)

        exps = Exp.instances[base]

        try:
            return exps[exp]
        except KeyError:
            exps[exp] = (exp_expr := Exp(base, exp))
            return exp_expr

    def __init__(self, base: int, exp: Count):
        self.base = base
        self.exp = exp

        self.depth = 1 if isinstance(exp, int) else 1 + exp.depth
        self.leaves = 2 if isinstance(exp, int) else 1 + exp.leaves

    def __repr__(self) -> str:
        exp = (
            '...'
            if MAX_LEAVES < self.leaves else
            show_number(self.exp)
        )

        return f'({self.base} ** {exp})'

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

    def __mod__(self, mod: int) -> int:
        if mod == 1:
            return 0

        base = self.base

        if base % mod == 0:
            return 0

        if mod == 2:
            return base % 2

        exp = self.exp

        match base:
            case 2:
                match mod:
                    case 4:
                        return 0

                    case 6:
                        return 4 if exp % 2 == 0 else 2

                    case 12:
                        return 4 if exp % 2 == 0 else 8

            case 3:
                if mod == 6:
                    return 3

                if int(log_mod := log2(mod)) == log_mod:
                    exp %= 2 ** (int(log_mod) - 2)

                    if exp == 0:
                        return 1

            case 6:
                if mod == 10:  # no-branch
                    return 6

            case 7:
                if mod == 12:  # no-branch
                    return 1 if exp % 2 == 0 else 7

        kp, k0 = carmichael(mod)

        if k0 < exp:
            exp = k0 + (exp - k0) % kp

        try:
            if (period := find_period(base, mod, exp)) > 0:
                exp %= period
        except PeriodLimit:
            if (crt_res := crt(base, exp, mod)) is not None:
                return crt_res

            raise

        assert isinstance(exp, int)

        return pow(base, exp, mod)

    def __radd__(self, other: int) -> Count:
        if other == 0:
            return self

        return Add.make(other, self)

    def __add__(self, other: Count) -> Count:
        if isinstance(other, int):
            return self if other == 0 else Add.make(other, self)

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
                except NotImplementedError:
                    pass

        elif isinstance(other, Exp):
            assert (base := self.base) == other.base

            if base == 2:
                sexp, oexp = self.exp, other.exp

                if sexp == oexp:
                    return Exp.make(2, 1 + sexp)

            try:
                return add_exponents((self, 1), (other, 1))
            except NotImplementedError:
                pass

        elif isinstance(other, Div):
            return other + self

        elif isinstance(other, Add):  # no-branch
            if isinstance(other.l, int):
                return other.l + (self + other.r)

        return Add.make(self, other)

    def __sub__(self, other: Count) -> Count:
        if other == 0:
            return self

        if self == other:
            return 0

        if isinstance(other, Exp):
            assert self.base == other.base

            return add_exponents((self, 1), (other, -1))

        if isinstance(other, int):
            return Add.make(-other, self)

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

            return Exp.make(base, self.exp + other.exp)

        if isinstance(other, Add):
            return (self * other.l) + (self * other.r)

        assert isinstance(other, Mul), (self, other)

        l, r = other.l, other.r

        if isinstance(l, int):
            return l * (self * r)

        if self.multiplies_with(l):
            return (self * l) * r

        if self.multiplies_with(r):
            return l * (self * r)

        return Mul.make(self, other) # no-cover

    def __rmul__(self, other: int) -> Count:
        if other == 0:
            return 0

        if other == 1:
            return self

        if other == -1:
            return Mul.make(-1, self)

        base = self.base

        if other < -1 and -other % base == 0:
            return -(-other * self)

        if other % base != 0:
            return Mul.make(other, self)

        exp = self.exp

        for i in itertools.count():
            if other % base != 0:
                exp += i
                break

            other //= base

        return other * Exp.make(base, exp)

    def __floordiv__(self, other: Count) -> Count:
        if other == 1:
            return self

        base, exp = self.base, self.exp

        if not isinstance(other, int):
            assert isinstance(other, Exp)
            assert other.base == base
            assert other.exp <= exp, (self, other)

            match (diff := exp - other.exp):
                case 0:  # no-cover
                    return 1
                case 1:
                    return base
                case _:
                    return Exp.make(base, diff)

        for i in itertools.count():
            if other % base != 0:
                exp -= i
                break

            other //= base

        if other > 1:
            assert base > other

            if base % other != 0:
                raise NumError(f'bad div: {self} // {other}')

            return (base // other) * Exp.make(base, exp - 1)

        return (
            1 if exp == 0 else
            base if exp == 1 else
            Exp.make(base, exp)
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

            if self == l:
                return r.pos

            return self < r

        if isinstance(other, Arrow):
            if other.arrows == 1 and other.base == base:
                return exp < other.height
            return other > self

        if not isinstance(other, Mul):  # no-cover
            raise_lt_not_implemented(self, other)

        l, r = other.l, other.r

        if (r.pos and self <= l) or (0 < l and self <= r):
            return True

        if isinstance(l, Exp):
            assert l.base == base
            assert l.exp <= exp

            return (self // l) < r

        assert isinstance(r, Exp)

        assert r.base == base
        assert r.exp <= exp

        return (self // r) < l

    def __pow__(self, other: Count) -> Exp:
        return Exp.make(self.base, self.exp * other)


class Arrow(Num):
    base: int
    arrows: int
    height: Count

    def __init__(self, base: int, arrows: int, height: Count):
        assert base >= 2
        assert arrows >= 1
        assert not isinstance(height, int) or height >= 1

        self.base = base
        self.arrows = arrows
        self.height = height

        self.depth = 1
        self.leaves = 2

    def __repr__(self) -> str:
        ops = '↑' * self.arrows if self.arrows <= 5 else f'↑^{self.arrows}'
        return f'({self.base} {ops} {show_number(self.height)})'

    def __hash__(self) -> int:
        return id(self)

    def digits(self) -> int:  # ruff: ignore[no-self-use]
        raise OverflowError

    def __neg__(self) -> Count:
        return Mul.make(-1, self)

    def __int__(self) -> int:
        raise NotImplementedError

    def __mod__(self, mod: int) -> int:
        raise NotImplementedError

    def __eq__(self, other: object) -> bool:
        return (
            isinstance(other, Arrow)
            and self.base == other.base
            and self.arrows == other.arrows
            and self.height == other.height
        )

    def __rpow__(self, other: int) -> Exp | Arrow:
        if self.arrows != 2 or other != self.base:
            return Exp.make(other, self)

        return Arrow(self.base, 2, self.height + 1)

    def __lt__(self, other: Count) -> bool:
        if isinstance(other, int):
            return False

        if isinstance(other, Arrow):
            if self.base != other.base:
                raise_lt_not_implemented(self, other)

            if self.arrows == other.arrows:
                return self.height < other.height

            if self.height == other.height and self.height > 2:  # no-cover
                return self.arrows < other.arrows

            raise_lt_not_implemented(self, other)

        if isinstance(other, Exp) and self.base == other.base:
            if self.arrows == 1:  # no-cover
                return self.height < other.exp

            if self.height == 1:  # no-cover
                if other.exp == 1:
                    raise_lt_not_implemented(self, other)
                return other.exp > 1

            if self.height == 2:  # no-cover
                if other.exp == self.base:
                    raise_lt_not_implemented(self, other)
                return other.exp > self.base

            if (isinstance(other.exp, int)  # no-branch
                    and other.exp < self.base ** self.base
                    and self.height > 2):
                return False

        raise_lt_not_implemented(self, other)

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
        Exp.make(base, diff)
    )

    return (l_co + (r_co * diff_exp)) * l_exp


def gcd(l: int, r: Count) -> int:
    assert l != 1

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

    if not isinstance(r, Exp):
        return 1

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


def inv_mod(a: int, m: int) -> int | None:
    def egcd(a: int, b: int) -> tuple[int, int, int]:
        if b == 0:
            return (a, 1, 0)

        g, x1, y1 = egcd(b, a % b)

        return g, y1, x1 - (a // b) * y1

    g, x, _ = egcd(a % m, m)

    return None if g != 1 else x % m


MOD_PERIOD_LIMIT = 2 ** 24

@cache
def find_period(base: int, mod: int, exp: int) -> int:
    if base == 2 and mod == 2 * (3 ** round(log(mod / 2, 3))):
        return 0

    if mod >= MOD_PERIOD_LIMIT:
        raise PeriodLimit(base, mod)

    base_mod = base % mod

    if pgcd(base_mod, mod) != 1:
        return 0

    lam, _ = carmichael(mod)

    order = lam

    for p, k in prime_factors(lam):
        for _ in range(k):
            if pow(base_mod, cand := order // p, mod) != 1:
                break

            order = cand

    assert order < mod

    return (
        order
        if 0 < order < exp else
        0
    )


def carmichael(mod: int) -> tuple[int, int]:
    def lcm(a : int, b : int) -> int:
        return a * b // pgcd(a, b)

    res, max_k = 1, 1

    for p, k in prime_factors(mod):
        lam_pk = (
            2 ** (k - 2)
            if p == 2 and k >= 3 else
            (p - 1) * p ** (k - 1)
        )

        res = lcm(res, lam_pk)

        if k > max_k:  # ruff:ignore[if-stmt-min-max]
            max_k = k

    return res, max_k


def crt(base: int, exp: Count, mod: int) -> int | None:
    assert mod != 1

    moduli = [p ** k for (p, k) in prime_factors(mod)]

    if len(moduli) == 1:
        return None

    n = Exp.make(base, exp)

    m0, *rest = moduli

    r: int = n % m0
    m: int = m0

    for mi in rest:
        ai = n % mi
        inv = inv_mod(m % mi, mi)
        t = ((ai - r) % mi) * inv % mi
        r = (r + m * t) % (m * mi)
        m *= mi

    return r


PRIMES = [
    2, 3, 5, 7, 11, 13, 17, 19,
    23, 29, 31, 37, 41, 43, 47,
    53, 59, 61, 67, 71, 73, 79,
    83, 89, 97, 127,
]


def prime_factors(n: int) -> list[tuple[int, int]]:
    assert n > 0

    res = []

    for p in PRIMES:  # no-branch
        k = 0

        while n % p == 0:
            k += 1
            n //= p

        if k:
            res.append((p, k))

        if n == 1:
            return res

    raise NotImplementedError  # no-cover
