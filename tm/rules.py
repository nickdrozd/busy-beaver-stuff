from __future__ import annotations

from abc import abstractmethod
from typing import TYPE_CHECKING

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

    Apps = tuple[Count, Index]


RULE_DESCENT: int = 20


def calculate_diff(
        cnt1: Count,
        cnt2: Count,
        cnt3: Count,
        cnt4: Count,
) -> Op | None:
    if cnt1 == cnt2 == cnt3 == cnt4:
        return None

    plus, diff1, diff2 = cnt2 - cnt1, cnt3 - cnt2, cnt4 - cnt3

    if plus == diff1 == diff2:
        assert isinstance(plus, int)
        return plus

    assert type(cnt1) == type(cnt2) == type(cnt3) == type(cnt4)  # pylint: disable = unidiomatic-typecheck

    try:
        ascending = cnt1 < cnt2 < cnt3 < cnt4
    except NotImplementedError:
        pass
    else:  # no-cover
        if not ascending:
            raise UnknownRule(
                'non-increasing')

    if not isinstance(cnt1, int):
        assert (
            not isinstance(cnt2, int)
            and not isinstance(cnt3, int)
            and not isinstance(cnt4, int)
        )

        return calculate_op_seq(cnt1, cnt2, cnt3, cnt4)

    assert (
        isinstance(cnt1, int)
        and isinstance(cnt2, int)
        and isinstance(cnt3, int)
        and isinstance(cnt4, int)
    )

    (div_1, mod_1), (div_2, mod_2), (div_3, mod_3) = \
        mult, _, _ = (
            divmod(cnt2, cnt1),
            divmod(cnt3, cnt2),
            divmod(cnt4, cnt3),
        )

    if not div_1 == div_2 == div_3:
        raise UnknownRule(
            'different divs')

    if mod_1 == mod_2 == mod_3:
        return mult

    if ((div_diff := cnt2 - (cnt1 * (1 + div_1)))
            == (cnt3 - (cnt2 * (1 + div_2)))
            == (cnt4 - (cnt3 * (1 + div_3)))):
        return 1 + div_1, div_diff

    raise UnknownRule


def calculate_op_seq(
        cnt1: Num,
        cnt2: Num,
        cnt3: Num,
        cnt4: Num,
) -> OpSeq:
    sub, descent = cnt1, []

    for _ in range(RULE_DESCENT):  # no-branch
        if sub in cnt2:
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

    sup, ascent = cnt2, []

    for _ in range(RULE_DESCENT):  # no-branch
        if sup == sub:
            break

        match sup:
            case Add():
                if not isinstance(l := sup.l, int):  # no-cover
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

    if (apply_ops(cnt2, 1, ops) != cnt3
            or apply_ops(cnt3, 1, ops) != cnt4):  # no-cover
        raise UnknownRule

    return ops


def make_rule(
        cnts1: Counts,
        cnts2: Counts,
        cnts3: Counts,
        cnts4: Counts,
) -> Rule | None:
    try:
        rule = {
            (s, i): diff
            for s, spans in enumerate(zip(cnts1, cnts2, cnts3, cnts4))
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
                raise RuleLimit('count_apps') from exc

            times = div if rem > 0 else div - 1

            if apps is None or times < apps[0]:
                apps = times, pos

        return apps

    def apply_rule(self, rule: Rule) -> Count | None:
        if (apps := self.count_apps(rule)) is None:
            return None

        times, min_pos = apps

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

                        result = (count % -diff) or -diff
                case ops:
                    result = apply_ops(count, times, ops) # type: ignore

            self.set_count(pos, result)

        return times


def apply_mult(count: Count, times: Count, mul: int, add: int) -> Count:
    if not isinstance(count, int) and count.depth > 20:  # no-cover
        raise RuleLimit('count-depth')

    if not isinstance(times, int) and times.depth > 200:  # no-cover
        raise RuleLimit('times-depth')

    exp = make_exp(mul, times)

    return (
        count * exp
        + add * (1 + ((exp - mul) // (mul - 1)))
    )


def apply_ops(count: Count, times: Count, ops: OpSeq) -> Count:
    if not isinstance(times, int):  # no-cover
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
