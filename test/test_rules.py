from unittest import TestCase

from tm.rules import apply_mult


def apply_loop(count: int, times: int, mul: int, add: int) -> int:
    val = count

    for _ in range(times):
        val = add + (mul * val)

    return val


VALUES = {
    ( 1,  4, 2, 3): 61,
    (45, 99, 2, 7): 32958915605933964438914283339769,
}


class TestApply(TestCase):
    def test_apply_mult(self):
        for (count, times, mul, add), val in VALUES.items():
            self.assertEqual(
                int(apply_mult(count, times, mul, add)),
                val)

            self.assertEqual(
                apply_loop(count, times, mul, add),
                val)
