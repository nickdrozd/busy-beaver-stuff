from unittest import TestCase

from test.prog_data import EXPAND, NORMALIZE, TNF
from tools.normalize import Normalizer, compact, expand, normalize
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

    def test_tnf_0rb(self):
        prog = "0RB 1RE  1LC ...  0LD 0LC  1RD 1LA  1LB 0RE"

        self.assertEqual(
            tree_norm(normalize(prog)),
            "1RB ...  0RC 0RB  1LC 1RD  0LA 1LE  1RA 0LE")

    def test_equiv(self):
        prog_153 = "1RB 0LB 0RC  2LC 2LA 1RA  1RA 1LC ..."

        prog_758 = "1RB 2LC 1RC  2LC ... 2RB  2LA 0LB 0RA"

        normaliz = Normalizer(prog_153)

        self.assertEqual(
            "1RC 0LC 0RB  1RA 1LB ...  2LB 2LA 1RA",
            str(normaliz.swap_states(1, 2)))

        self.assertEqual(
            "2RC 0RB 0LC  2RA ... 2LB  1LB 2RA 1LA",
            str(normaliz.swap_colors(1, 2)))

        self.assertEqual(
            "1LB 2RC 1LC  2RC ... 2LB  2RA 0RB 0LA",
            str(normaliz.swap_states(0, 2)))

        self.assertEqual(
            prog_758,
            str(normaliz.swap_shifts()))
