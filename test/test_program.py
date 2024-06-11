from unittest import TestCase

from test.prog_data import NORMALIZE, EXPAND

from tools.normalize import normalize, expand


class TestProgram(TestCase):
    def test_normalize(self):
        for norm, devs in NORMALIZE.items():
            for dev in devs:
                self.assertEqual(
                    norm,
                    normalize(dev))

    def test_expand(self):
        for compact, expanded in EXPAND.items():
            self.assertEqual(
                expand(compact),
                expanded,
            )
