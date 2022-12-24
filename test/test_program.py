from tm import Program

# pylint: disable = wrong-import-order
from test.test_turing import BackwardReasoning

from test.prog_data import (
    BLANKERS,
    CANT_BLANK_FALSE_NEGATIVES,
    CANT_SPIN_OUT_FALSE_NEGATIVES,
    DONT_BLANK,
    DONT_SPIN_OUT,
    DO_HALT,
    DO_SPIN_OUT,
    HALT_SLOW,
    RECUR_TOO_SLOW,
    SPINOUT,
    SPINOUT_BLANK,
    SPINOUT_BLANK_SLOW,
    SPINOUT_SLOW,
    UNDEFINED,
)

PROGS: dict[
    str,
    tuple[
        set[str],
        set[int],
        set[str],
        set[int],
        str | None,
        tuple[str, ...],
    ],
] = {
    "1RB ...  1LB 0RC  1LC 1LA": (
        {'A', 'B', 'C'},
        {0, 1},
        {'A', 'B', 'C'},
        {0, 1},
        'A1',
        ('A0', 'A1', 'B0', 'B1', 'C0', 'C1'),
    ),
    "1RB ...  1RC ...  ... ...  ... ...  ... ...": (
        {'B', 'C'},
        {1},
        {'A', 'B', 'C', 'D'},
        {0, 1},
        None,
        ('A0', 'A1', 'B0', 'B1', 'C0', 'C1', 'D0', 'D1', 'E0', 'E1'),
    ),
    "1RB ... ... ...  1LB 1LA ... ...": (
        {'A', 'B'},
        {1},
        {'A', 'B'},
        {0, 1, 2},
        None,
        ('A0', 'A1', 'A2', 'A3', 'B0', 'B1', 'B2', 'B3')
    )
}

BRANCH = {
    ("1RB 1LB  1LB 1LA", 'A1'): {
        '1RB 0LA  1LB 1LA',
        '1RB 0LB  1LB 1LA',
        '1RB 0RA  1LB 1LA',
        '1RB 0RB  1LB 1LA',
        '1RB 1LA  1LB 1LA',
        # '1RB 1LB  1LB 1LA',
        # '1RB 1RA  1LB 1LA',
        # '1RB 1RB  1LB 1LA',
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

NORMALIZE = {
    '1RB 2LA 1RA 1LA  3LA 1R_ 2RB 2RA': {
        '1RB 3LA 1LA 1RA  2LA 1R_ 3RA 3RB',
        '2RB 2RA 1LA 2LA  3LA 1RB 2R_ 1RA',
        '1LB 2RA 1LA 1RA  3RA 1L_ 2LB 2LA',
        '1LB 3RA 1RA 1LA  2RA 1L_ 3LA 3LB',
        '2LB 2LA 1RA 2RA  3RA 1LB 2L_ 1LA',
    },
    '1RB 1LC  1RD 1RB  0RE 1RE  1LD 1LA  0LF 1LF  0RD 0RC': {
        '1RB 1LE  1RD 1RB  0RD 0RE  1LD 1LA  0RF 1RF  0LC 1LC',
        '1RB 1LD  1RE 1RB  0RE 0RD  0RF 1RF  1LE 1LA  0LC 1LC',
        '1LB 1RC  1LD 1LB  0LE 1LE  1RD 1RA  0RF 1RF  0LD 0LC',
        '1LB 1RE  1LD 1LB  0LD 0LE  1RD 1RA  0LF 1LF  0RC 1RC',
        '1LB 1RD  1LE 1LB  0LE 0LD  0LF 1LF  1RE 1RA  0RC 1RC',
    },
}

class TestProgram(BackwardReasoning):
    prog: Program

    def assert_used_states(self, states: set[str]):
        self.assertEqual(
            states,
            set(self.prog.used_states))

    def assert_available_states(self, states: set[str]):
        self.assertEqual(
            states,
            set(self.prog.available_states))

    def assert_used_colors(self, colors: set[int]):
        self.assertEqual(
            colors,
            set(map(int, self.prog.used_colors)))

    def assert_available_colors(self, colors: set[int]):
        self.assertEqual(
            colors,
            set(map(int, self.prog.available_colors)))

    def assert_last_slot(self, slot: str | None):
        self.assertEqual(
            (slot[0], int(slot[1])) if slot is not None else None,
            self.prog.last_slot)

    def assert_slots(self, slots: tuple[str, ...]):
        self.assertEqual(
            tuple((slot[0], int(slot[1])) for slot in slots),
            self.prog.slots)

    def test_used_available(self):
        # pylint: disable = line-too-long
        for prog, (used_st, used_co, avail_st, avail_co, last, slots) in PROGS.items():
            self.prog = Program(prog)

            self.assert_used_states(used_st)
            self.assert_used_colors(used_co)
            self.assert_available_states(avail_st)
            self.assert_available_colors(avail_co)
            self.assert_last_slot(last)
            self.assert_slots(slots)

    def test_branch(self):
        for (prog, loc), extensions in BRANCH.items():
            self.assertEqual(
                set(Program(prog).branch((loc[0], int(loc[1])))),
                extensions)

        self.assertFalse(
            tuple(
                Program(
                    "1RB 1LB  1LA 0LC  ... 1LD  1RD 0RA").branch(
                        ('C', 0),
                        halt = True)))

    def test_normalize(self):
        for norm, devs in NORMALIZE.items():
            for dev in devs:
                self.assertEqual(
                    norm,
                    Program(dev).normalize())

    def test_undefined(self):
        for prog, sequence in UNDEFINED.items():
            self.assertEqual(
                sequence,
                {
                    partial: (step, state + str(color))
                    for partial, step, (state, color) in
                    Program(prog).instr_seq
                },
            )

    def test_mother_of_giants(self):
        mother = "1RB 1LE  0LC 0LB  0LD 1LC  1RD 1RA  ... 0LA"

        for prog in Program(mother).branch(('E', 0)):
            self.assert_could_blank(prog)
            self.assert_could_spin_out(prog)

    def test_blank(self):
        for prog in DONT_BLANK:
            self.assert_cant_blank(prog)

        for prog in BLANKERS:
            self.assert_simple(prog)
            self.assert_could_blank(prog)

    def test_false_negatives(self):
        for prog in CANT_BLANK_FALSE_NEGATIVES:
            self.assertNotIn(prog, BLANKERS)
            self.assert_could_blank(prog)

        for prog in CANT_SPIN_OUT_FALSE_NEGATIVES:
            self.assertNotIn(
                prog,
                SPINOUT
                 | SPINOUT_SLOW
                 | SPINOUT_BLANK
                 | SPINOUT_BLANK_SLOW)

            self.assert_could_spin_out(prog)

    def test_halt(self):
        for prog in DO_HALT | set(HALT_SLOW):
            self.assert_could_halt(prog)

    def test_spinout(self):
        for prog in DO_SPIN_OUT | set(SPINOUT_SLOW):
            self.assert_simple(prog)
            self.assert_could_spin_out(prog)

        for prog in DONT_SPIN_OUT:
            self.assert_cant_spin_out(prog)

    def test_recur(self):
        for prog in RECUR_TOO_SLOW:
            self.assert_cant_halt(prog)
            self.assert_cant_blank(prog)
            self.assert_cant_spin_out(prog)
