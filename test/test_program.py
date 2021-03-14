# pylint: disable = attribute-defined-outside-init

from unittest import TestCase

from tree_gen import Program

PROGS = {
    "1RB ... 1LB 0RC 1LC 1LA": (
        {'A', 'B', 'C'},
        {0, 1},
        {'A', 'B', 'C', 'H'},
        {0, 1},
        True,
    ),
    "1RB ... 1RC ... ... ... ... ... ... ...": (
        {'B', 'C'},
        {1},
        {'A', 'B', 'C', 'D'},
        {0, 1},
        False,
    ),
    "1RB ... ... ... 1LB 1LA ... 3..": (
        {'A', 'B'},
        {1},
        {'A', 'B'},
        {0, 1, 2},
        False,
    )
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

    def assert_used_colors(self, colors):
        self.assertEqual(
            colors,
            set(map(int, self.prog.used_colors)))

    def assert_available_colors(self, colors):
        self.assertEqual(
            colors,
            set(map(int, self.prog.available_colors)))

    def test_used_available(self):
        for prog, (used_st, used_co, avail_st, avail_co, last) in PROGS.items():
            self.prog = Program(prog)

            self.assert_used_states(used_st)
            self.assert_used_colors(used_co)
            self.assert_available_states(avail_st)
            self.assert_available_colors(avail_co)
            (self.assertTrue if last else self.assertFalse)(last)

    def test_branch(self):
        prog = Program("1RB ... 1LC ... ... ... ... ...")

        self.assertEqual(
            set(prog.branch('D0')),
            {
                '1RB ... 1LC ... ... ... 0LA ...',
                '1RB ... 1LC ... ... ... 0LB ...',
                '1RB ... 1LC ... ... ... 0LC ...',
                '1RB ... 1LC ... ... ... 0LD ...',
                '1RB ... 1LC ... ... ... 0RA ...',
                '1RB ... 1LC ... ... ... 0RB ...',
                '1RB ... 1LC ... ... ... 0RC ...',
                '1RB ... 1LC ... ... ... 0RD ...',
                '1RB ... 1LC ... ... ... 1LA ...',
                '1RB ... 1LC ... ... ... 1LB ...',
                '1RB ... 1LC ... ... ... 1LC ...',
                '1RB ... 1LC ... ... ... 1LD ...',
                '1RB ... 1LC ... ... ... 1RA ...',
                '1RB ... 1LC ... ... ... 1RB ...',
                '1RB ... 1LC ... ... ... 1RC ...',
                '1RB ... 1LC ... ... ... 1RD ...',
            }
        )
