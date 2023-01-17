from unittest import TestCase

# pylint: disable = wildcard-import, unused-wildcard-import
from test.prog_data import *

from tm.program import Program
from tm.machine import Machine


class BackwardReasoning(TestCase):
    def assert_could_halt(self, prog: str):
        self.assertFalse(
            Program(prog).cant_halt,
            f'halt false positive: {prog}')

    def assert_cant_halt(self, prog: str):
        self.assertTrue(
            Program(prog).cant_halt,
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
        if prog in CANT_SPIN_OUT_SLOW:
            return

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
