from unittest import TestCase

from test.utils import read_holdouts

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

LR_FALSE_POSITIVES = {
    "py": {
        "1RB 0RB  0LB 1RC  1LD 0RC  1LB 1LA",
        "1RB 0RB  0LC 1RD  1LC 1LA  1LB 0RD",
        "1RB 1LD  1LC 1RB  0RC 1LA  0RD 0LC",
    },
    "rs": {
        "1RB ...  1RC 0RA  0LD 0LB  0RB 1LC",
        "1RB 0LD  1LC 1RA  ... 1LD  0RD 1LA",
        "1RB 0RC  0LB 1LA  ... 1RD  1LB 0RD",
        "1RB 0RD  0LC 0LA  1LA 1LB  1RA ...",
        "1RB 1LA  0LC 0LD  ... 1LD  0RD 1RA",
        "1RB 1LC  0LA 0RD  1RC 1LB  ... 1RA",
        "1RB 1LC  0LA 0RD  1RD 1LB  ... 1RA",
    }
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
            if any(prog in progs
                   for progs in LR_FALSE_POSITIVES.values()):
                continue

            self.assertFalse(
                quick_term_or_rec(prog, STEPS),
                prog)
