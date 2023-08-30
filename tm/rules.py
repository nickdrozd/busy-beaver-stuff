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

    Mult = tuple[int, int]

    Op = Plus | Mult

    Index = tuple[int, int]

    Rule = dict[Index, Op]


def calculate_diff(cnt1: Count, cnt2: Count, cnt3: Count) -> Op | None:
    if cnt1 == cnt2 == cnt3:
        return None

    try:
        plus, diff = cnt2 - cnt1, cnt3 - cnt2
    except NumException as exc:
        raise RuleLimit from exc

    if plus == diff:
        return int(plus)

    if (not isinstance(cnt1, int)
            or not isinstance(cnt2, int)
            or not isinstance(cnt3, int)):
        raise RuleLimit

    if (mult := divmod(cnt2, cnt1)) == divmod(cnt3, cnt2):
        return mult

    raise UnknownRule()


def make_rule(cnts1: Counts, cnts2: Counts, cnts3: Counts) -> Rule:
    rule = {
        (s, i): diff
        for s, spans in enumerate(zip(cnts1, cnts2, cnts3))
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

    def count_apps(self, rule: Rule) -> tuple[Count, Index] | None:
        divs: list[tuple[Count, Index]] = []

        for pos, diff in rule.items():
            if not isinstance(diff, Plus) or diff >= 0:
                continue

            if (absdiff := abs(diff)) >= (count := self.get_count(pos)):
                return None

            try:
                div, rem = divmod(count, absdiff)
            except NumException as exc:
                raise RuleLimit from exc

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
                case (div, mod):
                    result = apply_mult(count, times, div, mod)
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


def apply_mult(count: Count, times: Count, div: int, mod: int) -> Count:
    exp = (
        div ** times
        if isinstance(times, int) and log10(times) < 3 else
        Exp(div, times)
    )

    result: Count = (
        count * exp
        + mod * (1 + ((exp - div) // (div - 1)))
    )

    return result
