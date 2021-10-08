# pylint: disable = attribute-defined-outside-init

from unittest import TestCase

from generate.program import Program

PROGS = {
    "1RB ...  1LB 0RC  1LC 1LA": (
        {'A', 'B', 'C'},
        {0, 1},
        {'A', 'B', 'C'},
        {0, 1},
        'A1',
    ),
    "1RB ...  1RC ...  ... ...  ... ...  ... ...": (
        {'B', 'C'},
        {1},
        {'A', 'B', 'C', 'D'},
        {0, 1},
        None,
    ),
    "1RB ... ... ...  1LB 1LA ... ...": (
        {'A', 'B'},
        {1},
        {'A', 'B'},
        {0, 1, 2},
        None,
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

    def assert_last_slot(self, slot):
        self.assertEqual(
            slot,
            self.prog.last_slot)

    def test_used_available(self):
        # pylint: disable = line-too-long
        for prog, (used_st, used_co, avail_st, avail_co, last) in PROGS.items():
            self.prog = Program(prog)

            self.assert_used_states(used_st)
            self.assert_used_colors(used_co)
            self.assert_available_states(avail_st)
            self.assert_available_colors(avail_co)
            self.assert_last_slot(last)

    def test_branch(self):
        for (prog, loc), extensions in BRANCH.items():
            self.assertEqual(
                set(Program(prog).branch(loc)),
                extensions)


BRANCH = {
    ("1RB 1LB  1LB 1LA", 'A1'): {
        '1RB 0LA  1LB 1LA',
        '1RB 0LB  1LB 1LA',
        '1RB 0RA  1LB 1LA',
        '1RB 0RB  1LB 1LA',
        '1RB 1LA  1LB 1LA',
        '1RB 1LB  1LB 1LA',
        '1RB 1RA  1LB 1LA',
        '1RB 1RB  1LB 1LA',
    },

    ("1RB ...  1LC ...  ... ...  ... ...", 'D0'): {
        '1RB ...  1LC ...  ... ...  0LA ...',
        '1RB ...  1LC ...  ... ...  0LB ...',
        '1RB ...  1LC ...  ... ...  0LC ...',
        '1RB ...  1LC ...  ... ...  0LD ...',
        '1RB ...  1LC ...  ... ...  0RA ...',
        '1RB ...  1LC ...  ... ...  0RB ...',
        '1RB ...  1LC ...  ... ...  0RC ...',
        '1RB ...  1LC ...  ... ...  0RD ...',
        '1RB ...  1LC ...  ... ...  1LA ...',
        '1RB ...  1LC ...  ... ...  1LB ...',
        '1RB ...  1LC ...  ... ...  1LC ...',
        '1RB ...  1LC ...  ... ...  1LD ...',
        '1RB ...  1LC ...  ... ...  1RA ...',
        '1RB ...  1LC ...  ... ...  1RB ...',
        '1RB ...  1LC ...  ... ...  1RC ...',
        '1RB ...  1LC ...  ... ...  1RD ...',
    },

    ("1RB ... ... ...  1LB 1LA ... ...", 'A1'): {
        '1RB 0LA ... ...  1LB 1LA ... ...',
        '1RB 0LB ... ...  1LB 1LA ... ...',
        '1RB 0RA ... ...  1LB 1LA ... ...',
        '1RB 0RB ... ...  1LB 1LA ... ...',
        '1RB 1LA ... ...  1LB 1LA ... ...',
        '1RB 1LB ... ...  1LB 1LA ... ...',
        '1RB 1RA ... ...  1LB 1LA ... ...',
        '1RB 1RB ... ...  1LB 1LA ... ...',
        '1RB 2LA ... ...  1LB 1LA ... ...',
        '1RB 2LB ... ...  1LB 1LA ... ...',
        '1RB 2RA ... ...  1LB 1LA ... ...',
        '1RB 2RB ... ...  1LB 1LA ... ...',
    },

    ("1RB ... ... ...  ... ... ... ...", 'B0'): {
        '1RB ... ... ...  0LA ... ... ...',
        '1RB ... ... ...  0LB ... ... ...',
        '1RB ... ... ...  0RA ... ... ...',
        '1RB ... ... ...  0RB ... ... ...',
        '1RB ... ... ...  1LA ... ... ...',
        '1RB ... ... ...  1LB ... ... ...',
        '1RB ... ... ...  1RA ... ... ...',
        '1RB ... ... ...  1RB ... ... ...',
        '1RB ... ... ...  2LA ... ... ...',
        '1RB ... ... ...  2LB ... ... ...',
        '1RB ... ... ...  2RA ... ... ...',
        '1RB ... ... ...  2RB ... ... ...',
    },
}
