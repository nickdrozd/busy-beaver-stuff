# pylint: disable = attribute-defined-outside-init

from unittest import TestCase

from tree_gen import Program

PROGS = {
    "1RB ... 1LB 0RC 1LC 1LA": (
        {'A', 'B', 'C'},
        {0, 1},
        {'A', 'B', 'C'},
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
        # pylint: disable = line-too-long
        for prog, (used_st, used_co, avail_st, avail_co, last) in PROGS.items():
            self.prog = Program(prog)

            self.assert_used_states(used_st)
            self.assert_used_colors(used_co)
            self.assert_available_states(avail_st)
            self.assert_available_colors(avail_co)
            (self.assertTrue if last else self.assertFalse)(last)

    def test_branch_1(self):
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

    def test_branch_2(self):
        prog = Program("1RB ... ... ... 1LB 1LA ... 3..")

        self.assertEqual(
            set(prog.branch('A1')),
            {
                '1RB 0LA ... ... 1LB 1LA ... 3..',
                '1RB 0LB ... ... 1LB 1LA ... 3..',
                '1RB 0RA ... ... 1LB 1LA ... 3..',
                '1RB 0RB ... ... 1LB 1LA ... 3..',
                '1RB 1LA ... ... 1LB 1LA ... 3..',
                '1RB 1LB ... ... 1LB 1LA ... 3..',
                '1RB 1RA ... ... 1LB 1LA ... 3..',
                '1RB 1RB ... ... 1LB 1LA ... 3..',
                '1RB 2LA ... ... 1LB 1LA ... 3..',
                '1RB 2LB ... ... 1LB 1LA ... 3..',
                '1RB 2RA ... ... 1LB 1LA ... 3..',
                '1RB 2RB ... ... 1LB 1LA ... 3..',
            }
        )

    def test_branch_3(self):
        prog = Program("1RB ... ... ... ... ... ... ..3")

        self.assertEqual(
            set(prog.branch('B0')),
            {
                '1RB ... ... ... 0LA ... ... ..3',
                '1RB ... ... ... 0LB ... ... ..3',
                '1RB ... ... ... 0RA ... ... ..3',
                '1RB ... ... ... 0RB ... ... ..3',
                '1RB ... ... ... 1LA ... ... ..3',
                '1RB ... ... ... 1LB ... ... ..3',
                '1RB ... ... ... 1RA ... ... ..3',
                '1RB ... ... ... 1RB ... ... ..3',
                '1RB ... ... ... 2LA ... ... ..3',
                '1RB ... ... ... 2LB ... ... ..3',
                '1RB ... ... ... 2RA ... ... ..3',
                '1RB ... ... ... 2RB ... ... ..3',
            }
        )
