from __future__ import annotations

from math import log10
from abc import abstractmethod
from typing import TYPE_CHECKING

from tm.rust_stuff import RuleLimit, UnknownRule, InfiniteRule


Plus = int

if TYPE_CHECKING:
    Mult = tuple[int, int]

    Op = Plus | Mult

    Index = tuple[int, int]

    Rule = dict[Index, Op]

    Count = int

    Counts = tuple[list[Count], list[Count]]


def calculate_diff(cnt1: int, cnt2: int, cnt3: int) -> Op | None:
    if cnt1 == cnt2 == cnt3:
        return None

    if (plus := cnt2 - cnt1) == cnt3 - cnt2:
        return plus

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

    def count_apps(self, rule: Rule) -> int | None:
        divs: list[int] = []

        for pos, diff in rule.items():
            if not isinstance(diff, Plus) or diff >= 0:
                continue

            if (absdiff := abs(diff)) >= (count := self.get_count(pos)):
                return None

            div, rem = divmod(count, absdiff)
            divs.append(div if rem > 0 else div - 1)

        return min(divs)

    def apply_rule(self, rule: Rule) -> int | None:
        if (times := self.count_apps(rule)) is None:
            return None

        if (any(not isinstance(op, Plus) for op in rule.values())
                and log10(times) > 10):
            raise RuleLimit()

        for pos, diff in rule.items():
            count = self.get_count(pos)

            match diff:
                case (div, mod):
                    result = apply_mult(count, times, div, mod)
                case _:
                    assert isinstance(diff, Plus)
                    result = apply_plus(count, times, diff)

            self.set_count(pos, result)

        return times


def apply_plus(count: Count, times: int, diff: Plus) -> Count:
    return (
        count + diff * times
        if diff >= -1 else
        mod
        if (mod := count % -diff) > 0 else
        mod + -diff
    )


def apply_mult(count: Count, times: int, div: int, mod: int) -> Count:
    result: Count = (
        count
        * (term := div ** times)
        + mod * (1 + ((term - div) // (div - 1)))
    )

    return result
