from unittest import TestCase

from test.utils import read_holdouts
from tm.machine import term_or_rec

STEPS = 1_000

HOLDOUT_COUNTS = {
    '32q': 3,
    '23q': 9,
    '42h': 3,
    '42q': 43,
    '24h': 37,
}

TOTAL_HOLDOUTS = 95


class Holdouts(TestCase):
    def test_holdouts(self):
        holdouts: set[str] = set()

        for cat, count in HOLDOUT_COUNTS.items():
            progs = read_holdouts(cat)

            self.assertEqual(
                len(progs),
                count)

            holdouts |= progs

        self.assertEqual(
            len(holdouts),
            TOTAL_HOLDOUTS)

        for prog in holdouts:
            self.assertFalse(
                term_or_rec(prog, STEPS),
                prog)
