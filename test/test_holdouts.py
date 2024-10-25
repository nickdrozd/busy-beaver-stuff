from unittest import TestCase

from test.utils import read_holdouts
from test.lin_rec import run_loose_linrec_machine

from tm.machine import quick_term_or_rec


STEPS = 1_000

HOLDOUT_COUNTS = {
    '32q': 3,
    '23q': 9,
    '42h': 8,
    '42q': 104,
    '24h': 759,
}

TOTAL_HOLDOUTS = 883

LR_FALSE_POSIIVES = {
    "1RB 0RB  0LB 1RC  1LD 0RC  1LB 1LA",
    "1RB 0RB  0LC 1RD  1LC 1LA  1LB 0RD",
    "1RB 1LD  1LC 1RB  0RC 1LA  0RD 0LC",
}


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

            if prog in LR_FALSE_POSIIVES:
                self.assertIsNotNone(
                    run_loose_linrec_machine(prog, 300).infrul)

                continue

            self.assertIsNotNone(
                run_loose_linrec_machine(prog, STEPS).xlimit)
