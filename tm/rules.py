from math import log10
from abc import abstractmethod

from tm.rust_stuff import RuleLimit

Plus = int
Mult = tuple[int, int]

Op = Plus | Mult

Index = tuple[int, int]

Rule = dict[Index, Op]

Counts = tuple[tuple[int, ...], tuple[int, ...]]


class ApplyRule:
    @abstractmethod
    def __getitem__(self, index: Index) -> int: ...

    @abstractmethod
    def __setitem__(self, index: Index, val: int) -> None: ...

    def count_apps(self, rule: Rule) -> int | None:
        divs: list[int] = []

        for pos, diff in rule.items():
            if not isinstance(diff, Plus) or diff >= 0:
                continue

            if (abs_diff := abs(diff)) >= (count := self[pos]):
                return None

            div, rem = divmod(count, abs_diff)
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
                self[pos] += diff * times
                continue

            div, mod = diff

            self[pos] *= (term := div ** times)
            self[pos] += mod * (1 + ((term - div) // (div-1)))

        return times
