from unittest import TestCase

from test.prog_data import EXPAND, NORMALIZE, TNF
from tools.normalize import compact, expand, normalize
from tools.tree_norm import tree_norm


class TestProgram(TestCase):
    def test_normalize(self):
        for norm, devs in NORMALIZE.items():
            for dev in devs:
                self.assertEqual(
                    norm,
                    normalize(dev))

    def test_expand(self):
        for compacted, expanded in EXPAND.items():
            self.assertEqual(
                expand(compacted),
                expanded)

            self.assertEqual(
                compact(expanded),
                compacted.replace('1RZ', '---'))

            self.assertEqual(
                expanded,
                expand(expanded))

            self.assertEqual(
                compacted,
                compact(compacted))

    def test_tnf(self):
        for lex, tnf in TNF.items():
            self.assertEqual(
                tnf,
                tree_norm(lex))
