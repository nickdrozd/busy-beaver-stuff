from unittest import skip

from test.test_turing import TuringTest
from test.prog_data import (
    HALT_SLOW,
    RECUR_SLOW,
    SPINOUT_SLOW,
    SPINOUT_BLANK_SLOW,
)

class Slow(TuringTest):
    @skip('')
    def test_halt(self):
        self._test_halt(HALT_SLOW)

    def test_spinout(self):
        self._test_spinout(SPINOUT_SLOW)
        self._test_spinout(SPINOUT_BLANK_SLOW, blank = True)

    def test_recur(self):
        self._test_recur(RECUR_SLOW, quick = False)
