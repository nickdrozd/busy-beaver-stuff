from __future__ import annotations

from abc import abstractmethod
from typing import TYPE_CHECKING
from itertools import pairwise

from tm.num import Add, Mul, Div, Exp, NumException, make_exp
from tm.rust_stuff import RuleLimit, UnknownRule, InfiniteRule


Plus = int

if TYPE_CHECKING:
    from tm.num import Num, Count

    Counts = tuple[
        list[Count],
        list[Count],
    ]

    Mult = tuple[int, int]

    OpSeq = tuple[tuple[str, int], ...]

    Op = Plus | Mult | OpSeq

    Index = tuple[int, int]

    Rule = dict[Index, Op]

    Apps = tuple[Count, Index, int]


RULE_DESCENT: int = 50


class SuspectedRule(Exception):
    pass


POSSIBLE_RULE_PAIRS = (3, 2), (5, 3), (5, 2), (5, 4), (4, 3)


def calculate_diff(*counts: Count) -> Op | None:
    count_1, *rest = counts

    if all(count == count_1 for count in rest):
        return None

    count_2, *_ = rest

    plus = count_2 - count_1

    if all(aft - bef == plus for bef, aft in pairwise(rest)):
        assert isinstance(plus, int)
        return plus

    try:
        ascending = all(bef < aft for bef, aft in pairwise(counts))
    except NotImplementedError:
        pass
    else:  # no-cover
        if not ascending:
            raise UnknownRule(
                'non-increasing')

    _, cnt3, cnt4 = rest

    if not isinstance(count_1, int):
        assert (
            not isinstance(count_2, int)
            and not isinstance(cnt3, int)
            and not isinstance(cnt4, int)
        )

        return calculate_op_seq(count_1, count_2, cnt3, cnt4)

    assert (
        isinstance(count_1, int)
        and isinstance(count_2, int)
        and isinstance(cnt3, int)
        and isinstance(cnt4, int)
    )

    div, mod = divmod(count_2, count_1)

    divmods = tuple(
        (bef, aft, *divmod(aft, bef))  # type: ignore[operator]
        for bef, aft in pairwise(rest)
    )

    if not all(div_next == div for _, _, div_next, _ in divmods):
        raise UnknownRule(
            'different divs')

    if all(mod_next == mod for _, _, _, mod_next in divmods):
        return div, mod

    div_diff = count_2 - (count_1 * (1 + div))

    if all(aft - (bef * (1 + divv)) == div_diff
           for bef, aft, divv, _ in divmods):
        return 1 + div, div_diff

    for add, sub in POSSIBLE_RULE_PAIRS:
        if ((count_2 - count_1 * add // sub)
                == (cnt3 - count_2 * add // sub)
                == (cnt4 - cnt3 * add // sub)):
            raise SuspectedRule(add, sub)

    raise UnknownRule


def calculate_op_seq(*counts: Num) -> OpSeq:
    sub, sup, *_ = counts

    descent = []

    for _ in range(RULE_DESCENT):  # no-branch
        if sub in sup:
            break

        match sub:
            case Add():
                if not isinstance(l := sub.l, int):
                    raise RuleLimit('sub_add')

                descent.append(
                    ('+', -l))

                sub = sub.r

            case Mul():
                if not isinstance(l := sub.l, int):
                    raise RuleLimit('sub_mul')

                descent.append(
                    ('//', l))

                sub = sub.r

            case Div():
                descent.append(
                    ('*', sub.den))

                sub = sub.num

            case Exp():  # no-branch
                if isinstance(exp := sub.exp, int):
                    raise RuleLimit('sub_exp')

                descent.append(
                    ('~', sub.base))

                sub = exp

    else:
        raise RuleLimit(  # no-cover
            'subexpression descent')

    ascent = []

    for _ in range(RULE_DESCENT):  # no-branch
        if sup == sub:
            break

        match sup:
            case Add():
                if not isinstance(l := sup.l, int):
                    raise RuleLimit('sup_add')

                ascent.append(
                    ('+', l))

                sup = sup.r

            case Mul():
                if not isinstance(l := sup.l, int):
                    raise RuleLimit('sup_mul')

                ascent.append(
                    ('*', l))

                sup = sup.r

            case Div():
                ascent.append(
                    ('//', sup.den))

                sup = sup.num

            case Exp():
                if isinstance(exp := sup.exp, int):  # no-cover
                    raise RuleLimit('sup_exp')

                ascent.append(
                    ('**', sup.base))

                sup = exp

    else:
        raise RuleLimit(  # no-cover
            'superexpression descent')

    ops = tuple(descent) + tuple(reversed(ascent))

    if any(apply_ops(bef, 1, ops) != aft
           for bef, aft in pairwise(counts[1:])):  # no-cover
        raise UnknownRule

    return ops


def make_rule(*countses: Counts) -> Rule | None:
    try:
        rule = {
            (s, i): diff
            for s, spans in enumerate(zip(*countses))
            for i, counts in enumerate(zip(*spans))
            if (diff := calculate_diff(*counts)) is not None
        }
    except UnknownRule:
        return None

    if all(diff >= 0
           for diff in rule.values()
           if isinstance(diff, Plus)):

        # print(f'inf: {rule}')

        raise InfiniteRule()

    return rule


class ApplyRule:
    @abstractmethod
    def get_count(self, index: Index) -> Count: ...

    @abstractmethod
    def set_count(self, index: Index, val: Count) -> None: ...

    def count_apps(self, rule: Rule) -> Apps | None:
        apps: Apps | None = None

        for pos, diff in rule.items():
            if not isinstance(diff, Plus) or diff >= 0:
                continue

            count, absdiff = self.get_count(pos), abs(diff)

            if isinstance(count, int) and absdiff >= count:
                return None

            try:
                div, rem = divmod(count, absdiff)
            except NumException as exc:
                raise RuleLimit(f'count_apps: {exc}') from exc

            times, min_res = (
                (div, rem)
                if rem > 0 else
                (div - 1, absdiff)
            )

            if apps is None or times < apps[0]:
                apps = times, pos, min_res

        return apps

    def apply_rule(self, rule: Rule) -> Count | None:
        if (apps := self.count_apps(rule)) is None:
            return None

        times, min_pos, min_res = apps

        for pos, diff in rule.items():
            count = self.get_count(pos)

            match diff:
                case (int(mul), int(add)):
                    result = apply_mult(count, times, mul, add)
                case Plus():  # no-branch
                    if pos != min_pos:
                        result = count + diff * times
                    else:
                        assert diff < 0
                        result = min_res
                case ops:
                    result = apply_ops(count, times, ops) # type: ignore

            self.set_count(pos, result)

        return times


def apply_mult(count: Count, times: Count, mul: int, add: int) -> Count:
    if not isinstance(count, int) and count.depth > 20:  # no-cover
        raise RuleLimit('count-depth')

    if not isinstance(times, int) and times.depth > 200:  # no-cover
        raise RuleLimit('times-depth')

    exp: int | Exp = (
        mul
        if times == 1 else
        make_exp(mul, times)
    )

    return (
        count * exp
        + add * (1 + ((exp - mul) // (mul - 1)))
    )


def apply_ops(count: Count, times: Count, ops: OpSeq) -> Count:
    if not isinstance(times, int):
        raise RuleLimit('ops_times')

    result = count

    for _ in range(times):
        for op, val in ops:
            match op:
                case '+':
                    result += val

                case '*':
                    result *= val

                case '//':
                    result //= val

                case '**':
                    result = val ** result

                case '~':  # no-branch
                    if not isinstance(result, Exp):
                        raise RuleLimit('inapplicable_op')

                    result = result.exp

    return result
