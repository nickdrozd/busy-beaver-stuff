from unittest import TestCase

from tm.rules import apply_mult


def apply_loop(
        count: int,
        times: int,
        bef: int,
        mul: int,
        aft: int,
) -> int:
    val = count

    for _ in range(times):
        val = aft + (mul * (bef + val))

    return val


VALUES = {
    ( 1,  4, 0, 2, 3): 61,
    ( 1,  4, 5, 2, 3): 211,
    (45, 99, 0, 2, 7): 32958915605933964438914283339769,
    (45, 99, 5, 2, 7): 39297168607075111446397799366639,
}


class TestApply(TestCase):
    def test_apply_mult(self):
        for (count, times, bef, mul, aft), val in VALUES.items():
            self.assertEqual(
                apply_mult(count, times, bef, mul, aft),
                val)

            self.assertEqual(
                apply_loop(count, times, bef, mul, aft),
                val)
