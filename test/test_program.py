# pylint: disable = attribute-defined-outside-init

from unittest import TestCase

from tree_gen import Program

PROGS = {
    "1RB ... 1LB 0RC 1LC 1LA": (
        {'A', 'B', 'C'},
        {'A', 'B', 'C', 'H'},
        True,
    ),
    "1RB ... 1RC ... ... ... ... ... ... ...": (
        {'B', 'C'},
        {'A', 'B', 'C', 'D'},
        False,
    ),
}


class TestProgram(TestCase):
    def assert_used_states(self, states):
        self.assertEqual(
            states,
            self.prog.used_states)

    def assert_available_states(self, states):
        self.assertEqual(
            states,
            self.prog.available_states)

    def test_states(self):
        for prog, (used, avail, last) in PROGS.items():
            self.prog = Program(prog)

            self.assert_used_states(used)
            self.assert_available_states(avail)
            (self.assertTrue if last else self.assertFalse)(last)

    def test_branch(self):
        prog = Program("1RB ... 1LC ... ... ...")

        self.assertEqual(
            set(prog.branch('D0')),
            {
                '1RB ... 1LC ... ... ... 0LB',
                '1RB ... 1LC ... ... ... 0LA',
                '1RB ... 1LC ... ... ... 0LC',
                '1RB ... 1LC ... ... ... 0RB',
                '1RB ... 1LC ... ... ... 0RA',
                '1RB ... 1LC ... ... ... 0RC',
                '1RB ... 1LC ... ... ... 1LB',
                '1RB ... 1LC ... ... ... 1LA',
                '1RB ... 1LC ... ... ... 1LC',
                '1RB ... 1LC ... ... ... 1RB',
                '1RB ... 1LC ... ... ... 1RA',
                '1RB ... 1LC ... ... ... 1RC',
            }
        )
