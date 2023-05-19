from __future__ import annotations

from math import log10
from abc import abstractmethod
from typing import TYPE_CHECKING

from tm.rust_stuff import RuleLimit


Count = int

Plus = int

if TYPE_CHECKING:
    Mult = tuple[Count, Count]

    Op = Plus | Mult

    Index = tuple[int, int]

    Rule = dict[Index, Op]

    Counts = tuple[list[Count], list[Count]]


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
