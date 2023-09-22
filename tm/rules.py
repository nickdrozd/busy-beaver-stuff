from __future__ import annotations

from abc import abstractmethod
from math import log10
from typing import TYPE_CHECKING

from tm.num import Exp, NumException
from tm.rust_stuff import RuleLimit, UnknownRule, InfiniteRule


Plus = int

if TYPE_CHECKING:
    from tm.num import Count

    Counts = tuple[
        list[Count],
        list[Count],
    ]

    Mult = tuple[int, int, int]

    Op = Plus | Mult

    Index = tuple[int, int]

    Rule = dict[Index, Op]

    Apps = tuple[Count, Index]


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

    if (not isinstance(cnt1, int)
            or not isinstance(cnt2, int)
            or not isinstance(cnt3, int)):
        raise RuleLimit('calculate_diff')

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
        return 0, *mult

    mdm1, mdm2 = divmod(mod_2, mod_1), divmod(mod_3, mod_2)

    if mdm1 != mdm2:  # no-cover
        raise UnknownRule(
            'different mdms')

    if div_1 == 1:  # no-branch
        return 0, *mdm1

    raise UnknownRule  # no-cover


def make_rule(
        cnts1: Counts,
        cnts2: Counts,
        cnts3: Counts,
        cnts4: Counts,
) -> Rule:
    rule = {
        (s, i): diff
        for s, spans in enumerate(zip(cnts1, cnts2, cnts3, cnts4))
        for i, counts in enumerate(zip(*spans))
        if (diff := calculate_diff(*counts)) is not None
    }

    if all(diff >= 0
           for diff in rule.values()
           if isinstance(diff, Plus)):
        raise InfiniteRule()

    return rule


class ApplyRule:
    @abstractmethod
    def get_count(self, index: Index) -> Count: ...

    @abstractmethod
    def set_count(self, index: Index, val: Count) -> None: ...

    def count_apps(self, rule: Rule) -> Apps | None:
        divs: list[Apps] = []

        for pos, diff in rule.items():
            if not isinstance(diff, Plus) or diff >= 0:
                continue

            if (absdiff := abs(diff)) >= (count := self.get_count(pos)):
                return None

            try:
                div, rem = divmod(count, absdiff)
            except NumException as exc:
                raise RuleLimit('count_apps') from exc

            divs.append((
                div if rem > 0 else div - 1,
                pos,
            ))

        return min(divs, key = lambda p: p[0])

    def apply_rule(self, rule: Rule) -> Count | None:
        if (apps := self.count_apps(rule)) is None:
            return None

        times, min_pos = apps

        for pos, diff in rule.items():
            count = self.get_count(pos)

            match diff:
                case (bef, mul, aft):
                    result = apply_mult(count, times, bef, mul, aft)
                case _:
                    assert isinstance(diff, Plus)

                    if pos != min_pos:
                        result = apply_plus(count, times, diff)
                    else:
                        assert diff < 0

                        result = (
                            mod
                            if (mod := count % -diff) > 0 else
                            -diff
                        )

            self.set_count(pos, result)

        return times


def apply_plus(count: Count, times: Count, diff: Plus) -> Count:
    return count + diff * times


def apply_mult(
        count: Count,
        times: Count,
        bef: int,
        mul: int,
        aft: int,
) -> Count:
    if not isinstance(count, int) and count.depth > 20:  # no-cover
        raise RuleLimit('count-depth')

    if not isinstance(times, int) and times.depth > 200:  # no-cover
        raise RuleLimit('times-depth')

    exp = (
        mul ** times
        if isinstance(times, int) and log10(times) < 3 else
        Exp(mul, times)
    )

    result: Count = (
        count * exp
        + (aft + bef * mul) * (1 + ((exp - mul) // (mul - 1)))
    )

    return result
