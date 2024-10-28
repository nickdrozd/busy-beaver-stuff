from unittest import TestCase

from test.utils import read_holdouts

from tm.machine import quick_term_or_rec


STEPS = 1_000

HOLDOUT_COUNTS = {
    '32q': 3,
    '23q': 9,
    '42h': 8,
    '42q': 106,
    '24h': 759,
}

TOTAL_HOLDOUTS = 885


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
                quick_term_or_rec(prog, STEPS),
                prog)
