from typing import TYPE_CHECKING
from unittest import TestCase

from test.test_turing import (
    BACKWARD_REASONERS,
    REASON_LIMIT,
)

if TYPE_CHECKING:
    from test.prog_data import Goal
    from test.test_turing import BR


def get_reason(goal: Goal) -> tuple[BR, set[str]]:
    with open(f'test/data/holdouts/reason/{goal}.prog') as progs:  # noqa: PTH123
        return BACKWARD_REASONERS[goal], {prog.strip() for prog in progs}


HOLDOUTS: dict[Goal, int] = {
    'halt': 2790,
    'blank': 4007,
    'spinout': 3600,
}


class Reason(TestCase):
    def test_holdouts(self):
        unexpected: dict[Goal, set[str]] = {goal: set() for goal in HOLDOUTS}

        for goal, expected in HOLDOUTS.items():
            print(goal)

            reasoner, progs = get_reason(goal)

            self.assertEqual(len(progs), expected)

            for prog in progs:
                if not reasoner(prog, REASON_LIMIT).is_refuted():
                    continue

                unexpected[goal].add(prog)

        if all(not found for found in unexpected.values()):
            return

        for goal, found in unexpected.items():
            print(goal)

            for prog in found:
                print(prog)

        raise AssertionError
