from __future__ import annotations

from abc import abstractmethod
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

    Mult = tuple[int, int]

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

    assert type(cnt1) == type(cnt2) == type(cnt3) == type(cnt4)  # pylint: disable = unidiomatic-typecheck

    try:
        assert cnt1 < cnt2 < cnt3 < cnt4
    except NotImplementedError:
        pass

    if not isinstance(cnt1, int):
        raise RuleLimit('calculate_diff')

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

    if not div_1 == div_2 == div_3:  # no-cover
        raise UnknownRule(
            'different divs')

    if mod_1 == mod_2 == mod_3:
        return mult

    mdm1, mdm2 = divmod(mod_2, mod_1), divmod(mod_3, mod_2)

    if mdm1 != mdm2:
        raise UnknownRule(
            'different mdms')

    if div_1 == 1:
        return mdm1

    raise UnknownRule


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

            count, absdiff = self.get_count(pos), abs(diff)

            if isinstance(count, int) and absdiff >= count:
                return None

            try:
                div, rem = divmod(count, absdiff)
            except NumException as exc:
                raise RuleLimit('count_apps') from exc

            divs.append((
                div if rem > 0 else div - 1,
                pos,
            ))

        if (any(isinstance(div, int) for div, _ in divs)
                and not all(isinstance(div, int) for div, _ in divs)):
            divs = [
                divpos
                for divpos in divs
                if isinstance(divpos[0], int)
            ]

        return min(divs, key = lambda p: p[0])

    def apply_rule(self, rule: Rule) -> Count | None:
        if (apps := self.count_apps(rule)) is None:
            return None

        times, min_pos = apps

        for pos, diff in rule.items():
            count = self.get_count(pos)

            match diff:
                case (mul, add):
                    result = apply_mult(count, times, mul, add)
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


def apply_mult(count: Count, times: Count, mul: int, add: int) -> Count:
    if not isinstance(count, int) and count.depth > 20:  # no-cover
        raise RuleLimit('count-depth')

    if not isinstance(times, int) and times.depth > 200:  # no-cover
        raise RuleLimit('times-depth')

    exp = Exp(mul, times)

    result: Count = (
        count * exp
        + add * (1 + ((exp - mul) // (mul - 1)))
    )

    return result
