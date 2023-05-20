from __future__ import annotations

from math import log10
from abc import abstractmethod
from typing import TYPE_CHECKING

from tm.rust_stuff import RuleLimit, UnknownRule, InfiniteRule


Count = int

Plus = int

if TYPE_CHECKING:
    Mult = tuple[Count, Count]

    Op = Plus | Mult

    Index = tuple[int, int]

    Rule = dict[Index, Op]

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
    def get_count(self, index: Index) -> int: ...

    @abstractmethod
    def set_count(self, index: Index, val: Count) -> None: ...

    def count_apps(self, rule: Rule) -> Count | None:
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
            if isinstance(diff, Plus):
                count = (
                    self.get_count(pos) + diff * times
                    if diff >= -1 else
                    mod
                    if (mod := self.get_count(pos) % -diff) > 0 else
                    mod + -diff
                )
            else:
                div, mod = diff

                count = (
                    self.get_count(pos)
                        * (term := div ** times)
                        + mod * (1 + ((term - div) // (div - 1)))
                    )

            self.set_count(pos, count)

        return times
