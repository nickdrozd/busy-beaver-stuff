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
