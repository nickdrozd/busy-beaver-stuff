# pylint: disable = line-too-long

from unittest import TestCase

from tm.machine import Machine, quick_term_or_rec
from tm.reason import (
    cant_halt,
    cant_blank,
    cant_spin_out,
)

from tools.instr_seq import instr_seq


class TestDisplay(TestCase):
    def test_display(self):
        machine = Machine(
            "1RB 2LA ...  1LB 1LA 0RA"
        ).run(watch_tape = True)

        print(machine)

        machine = Machine(
            "1RB 1RA  0RC 1LA  1LC 1LD  0RB 0RD"
        ).run(watch_tape = True)

        print(machine)

        machine = Machine(
            "1RB 2RA 3LA 0LB  1LB 1LA 0RB 1RB"
        ).run()

        self.assertEqual(
            str(machine.tape),
            "[0] 3^97 1^2")

        machine = Machine(
            "1LB 2LA 3RA 0RB  1RB 1RA 0LB 1LB"
        ).run()

        self.assertEqual(
            str(machine.tape),
            "1^2 3^97 [0]")

        self.assertEqual(
            str(Machine("1RB 1RB  1LA ...").run(watch_tape = True)),
            "1RB 1RB  1LA ... || CYCLES: 3 | MARKS: 2 | UNDFND: (3, (1, 1)) | TPCFGS: 4")


class TestFloss(TestCase):
    def test_macro(self):
        self.assertIsNotNone(
            Machine(
                "1RB 2LA 1RA 1RA  1LB 1LA 3RB ...",
                blocks = 2,
            ).run().simple_termination)

        self.assertIsNotNone(
            macro := Machine(
                "1RB 0RA 1LB  2LA 2RB 0LA",
                blocks = 3,
                backsym = 1,
                params = (2, 3),
            ).run().program)

        self.assertEqual(
            str(macro),
            "1RB 0RA 1LB  2LA 2RB 0LA (3-cell block macro) (1-cell backsymbol macro)")

        self.assertIsNone(
            Machine(
                "1RB 1LB  1LA ...",
                backsym = 1,
            ).run(100).xlimit)

        self.assertIsNone(
            Machine(
                "1RB 1LA  1LA ...",
                backsym = 1,
                params = (2, 2),
            ).run(100).xlimit)

        self.assertIsNotNone(
            (machine := Machine(
                "1RB 0LC  0RD 1RA  ... 0LD  1LE 1LA  0LF 1LA  0RE 1LF",
                opt_macro = 200,
            ).run(500)).infrul)

        print(machine)

    def test_machine(self):
        self.assertIsNotNone(
            Machine(
                "1RB 1LA  1RC 0LB  0LB ..."
            ).run().blanks)

        self.assertTrue(
            quick_term_or_rec(
                "1RB 1LA  0LB 1LB",
                50))

        self.assertTrue(
            quick_term_or_rec(
                "1RB 1RB  0LA ...",
                10))

        self.assertFalse(
            quick_term_or_rec(
                "1RB 1RA  1RC 0RD  1LE 0RA  ... 0RB  1LB 1LE",
                100_000_000))

        self.assertIsNotNone(
            Machine(
                "1RB ...  0LB 0RA"
            ).run())

        self.assertFalse(
            quick_term_or_rec(
                "1RB ...  0LB 1LA",
                10))

        self.assertFalse(
            quick_term_or_rec(
                "1RB 1LA ...  2LB 1RB 0LA",
                50))

    def test_prover(self):
        progs = (
            "1RB 0LD  1LB 0RC  0RE 1LD  1LA ...  0RB 0LC",
            "1RB 0RC  1LC 1RA  1RE 0LD  0LC 0LE  0RB 1LD",
            "1RB 0LC  1LC 1RA  1LD 0LD  0LE 0LC  1RE 0RB",
            "1RB 1LD  0RC 1LE  0RD 1RA  1LD 0LA  0LB 1LE",
            "1RB 3LA 4LB 0RB 1RA 3LA  2LA 2RA 4LA 1RA 5RB ...",
            "1RB 3RB 5RA 1LB 5LA 2LB  2LA 2RA 4RB ... 3LB 2LA",
            "1RB 1RA  1LC 0RF  0LE 0RD  0RE 1LB  1RA 0LC  ... 1RD",
            "1RB 1LC  1LA 0LD  1RB 0LA  ... 1LE  1RF 0LB  1RB 0RE",
            "1RB 1LD  0RC ...  1RD 0LA  1RE 1RF  1LC 1LE  1RA 0RD",
            "1LB ...  0LC 1LC  0LD 0LC  1LE 1RA  0LF 0LE  1LG 1RD  0LH 0LG  1LI 1RF  0LJ 0LI  1RJ 1RH",
        )

        for prog in progs:
            print(prog)

            self.assertIsNotNone(
                machine := Machine(
                    prog,
                    opt_macro = 500,
                ).run())

            self.assertIsNotNone(
                machine.marks)

        Machine(
            "1RB 2RA 1LB 2LA  2LA 3RB 1LB 2RA"
        ).run(
            sim_lim = 800
        )

    def test_reasoner(self):
        self.assertIsNone(
            cant_halt(
                "1RB 0RA  1LA ...",
                depth = 3))

        self.assertIsNone(
            cant_blank(
                "1RB 0RA  1LB 1LA",
                depth = 1))

        self.assertIsNotNone(
            cant_blank(
                "1RB 2LA 1LA  2LA 2RB 0RA",
                depth = 2))

        self.assertIsNone(
            cant_spin_out(
                "1RB 0RB 0LB  1LB 2RA 1LA",
                depth = 7))

        self.assertIsNotNone(
            cant_spin_out(
                "1RB 1LA  0LA 0RC  0LA 1RB",
                depth = 0))

        self.assertIsNone(
            cant_halt(
                "1RB ...  1LC 0RC  1RA 0LC",
                depth = 1_000))

        _ = list(instr_seq(
            "1RB 1LB  1LA ..."))

        _ = list(instr_seq(
            "1RB ...  0RC 0LA  1LC 1LD  0RB 0RD"))

    def test_mixed_divs(self):
        self.assertIsNotNone(
            Machine(
                "1RB 0RC  1LC 1RA  0RC 1RD  1LE 0RB  1LB 0LD",
                opt_macro = 50,
            ).run())

    def test_enum_rule(self):
        self.assertIsNotNone(
            Machine(
                "1RB 1LA  1RC 1RD  0LB ...  0LA 1RE  1RB 0RF  0RB 1LD"
            ).run(
                sim_lim = 4846
            ).xlimit)

    def test_sus_rule(self):
        self.assertEqual(
            Machine(
                "1RB 2LA 1RA 2LB 2RA  0LA 2RB 3RB 4RA ..."
            ).run().susrul,
            (5, 2))
