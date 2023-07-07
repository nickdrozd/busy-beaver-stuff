from unittest import TestCase

from test.prog_data import (
    KERNEL,
    SPAGHETTI,
    CANT_HALT_FALSE_NEGATIVES,
    CANT_BLANK_FALSE_NEGATIVES,
    CANT_SPIN_OUT_FALSE_NEGATIVES,
)

from tm.machine import Machine
from tm.program import Program


class BackwardReasoning(TestCase):
    def assert_could_halt(self, prog: str):
        self.assertFalse(
            Program(prog).cant_halt,
            f'halt false positive: {prog}')

    def assert_cant_halt(self, prog: str):
        self.assertTrue(
            Program(prog).cant_halt
                or prog in CANT_HALT_FALSE_NEGATIVES,
            f'halt false negative: "{prog}"')

    def assert_could_blank(self, prog: str):
        self.assertFalse(
            Program(prog).cant_blank,
            f'blank false positive: "{prog}"')

    def assert_cant_blank(self, prog: str):
        try:
            self.assertTrue(
                Program(prog).cant_blank)
        except AssertionError:
            self.assertTrue(
                prog in CANT_BLANK_FALSE_NEGATIVES
                or Machine(prog).run(sim_lim = 10).blanks,
                f'blank false negative: "{prog}"')

    def assert_could_spin_out(self, prog: str):
        self.assertFalse(
            Program(prog).cant_spin_out,
            f'spin out false positive: "{prog}"')

    def assert_cant_spin_out(self, prog: str):
        try:
            self.assertTrue(
                Program(prog).cant_spin_out)
        except AssertionError:
            self.assertIn(
                prog,
                CANT_SPIN_OUT_FALSE_NEGATIVES,
                f'spin out false negative: "{prog}"')

    def assert_simple(self, prog: str):
        self.assertTrue(
            Program(prog).graph.is_simple
            or prog in SPAGHETTI
            or prog in KERNEL
        )
