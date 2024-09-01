from unittest import TestCase

from test.prog_data import NORMALIZE, EXPAND, TNF

from tools.normalize import normalize, expand
from tools.tree_norm import tree_norm


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

    def test_tnf(self):
        for lex, tnf in TNF.items():
            self.assertEqual(
                tnf,
                tree_norm(lex))
