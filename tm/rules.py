from abc import abstractmethod

Plus = int
Mult = tuple[int, int]

Op = Plus | Mult

Rule = dict[tuple[int, int], Op]

Counts = tuple[tuple[int, ...], tuple[int, ...]]


class ImplausibleRule(Exception):
    pass


class InfiniteRule(Exception):
    pass


def calculate_diff(
        rec_rule: int,
        cnt1: int,
        cnt2: int,
        cnt3: int,
) -> Op | None:
    if cnt1 == cnt2:
        assert cnt3 == cnt1
        return None

    if (plus := cnt2 - cnt1) == cnt3 - cnt2:
        return plus

    if (mult := divmod(cnt2, cnt1)) == divmod(cnt3, cnt2):
        return mult

    if rec_rule:
        raise ImplausibleRule()

    return None


def make_rule(
        rec_rule: int,
        counts0: Counts,
        counts1: Counts,
        counts2: Counts,
) -> Rule:
    rule = {
        (s, i): diff
        for s, spans in enumerate(
                zip(counts0, counts1, counts2))
        for i, counts in enumerate(zip(*spans))
        if (diff := calculate_diff(
                rec_rule, *counts)) is not None
    }

    if all(diff >= 0
           for diff in rule.values()
           if isinstance(diff, Plus)):
        raise InfiniteRule()

    return rule


class ApplyRule:
    @abstractmethod
    def __getitem__(self, index: tuple[int, int]) -> int: ...

    @abstractmethod
    def __setitem__(self, index: tuple[int, int], val: int) -> None: ...

    def apply_rule(self, rule: Rule) -> int | None:
        divs: list[int] = []

        for pos, diff in rule.items():
            if not isinstance(diff, Plus) or diff >= 0:
                continue

            if (abs_diff := abs(diff)) >= (count := self[pos]):
                return None

            div, rem = divmod(count, abs_diff)
            divs.append(div if rem > 0 else div - 1)

        times: int = min(divs)

        for pos, diff in rule.items():
            if isinstance(diff, Plus):
                self[pos] += diff * times
                continue

            div, mod = diff

            self[pos] *= (term := div ** times)
            self[pos] += mod * (1 + ((term - div) // (div-1)))

        return times
