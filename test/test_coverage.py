from unittest import TestCase

from tm.machine import Machine, LinRecMachine, run_variations
from tm.reason import (
    Program,
    instr_seq,
    cant_halt,
    cant_blank,
    cant_spin_out,
)


class TestDisplay(TestCase):
    def test_display(self):
        machine = Machine(
            "1RB 2LA 1R_  1LB 1LA 0RA"
        ).run(watch_tape = True)

        print(machine)

        machine = Machine(
            "1RB 1RA  0RC 0RB  1LC 1LD  1RA 1LB"
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

        print(Machine("1RB ...  ... ...").run(watch_tape = True))

        print(LinRecMachine("1RB 0LB  1LA 0RB").run(check_rec = 0))


class TestFloss(TestCase):
    def test_macro(self):
        self.assertIsNotNone(
            Machine(
                "1RB 2LA 1RA 1RA  1LB 1LA 3RB 1R_",
                blocks = 2,
            ).run())

        self.assertIsNotNone(
            macro := Machine(
                "1RB 0RA 1LB  2LA 2RB 0LA",
                blocks = [3, 3],
                backsym = [1, 1],
            ).run().comp)

        print(macro)

        self.assertEqual(len(macro), 1)

        self.assertEqual(
            Machine(
                program := Program(
                    "1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  1R_ 0LA"),
                blocks = 3,
            ).run().simple_termination,
            -1)

        self.assertEqual(
            len(program),
            10)

    def test_machine(self):
        self.assertIsNotNone(
            Machine(
                "1RB 1RA  1RC 0RD  1LE 0RA  0R_ 0RB  1LB 1LE"
            ).run().simple_termination)

        self.assertIsNotNone(
            Machine(
                "1RB 1LA  1RC 0LB  0LB ..."
            ).run().blanks)

        self.assertIsNotNone(
            LinRecMachine(
                "1RB 0LB  1LA 0RB"
            ).run(check_rec = 1))

        self.assertIsNotNone(
            LinRecMachine(
                "1RB 1LA  0RC ...  1LC 0LA"
            ).run(check_rec = 1))

        self.assertIsNotNone(
            LinRecMachine(
                "1RB 1RB  1RC 0LC  0LB 1RC"
            ).run(check_rec = 1))

        self.assertIsNotNone(
            LinRecMachine(
                "1RB 0LB  1LA 0RA"
            ).run(
                check_rec = 1,
                sim_lim = 50,
            ))

        self.assertIsNotNone(
            LinRecMachine(
                "1RB 1LA  0LB 1LB"
            ).run(check_rec = 1))

    def test_prover(self):
        self.assertIsNotNone(
            Machine(
                "1RB 2LA 1R_ 5LB 5LA 4LB  1LA 4RB 3RB 5LB 1LB 4RA"
            ).run())

        self.assertTrue(
            Machine(
                "1RB 0RA  1RA ..."
            ).run())

        self.assertIsNotNone(
            Machine(
                "1RB 2LA 1RA  1LB 1LA 2RB"
            ).run(
                sim_lim = 5739,
            ).xlimit)

    def test_rule_limit(self):
        progs = (
            "1RB 2LA 3LA 2LA  3LB 3RA 0RA 0RB",
            "1RB 1LA  1RC 0RE  1LD 0LA  1LC 0RD  1RC 1RB",
            "1RB 2RB 3RB 4RB 5LA 4RA  0LA 1RB 5RA ... ... 1LB",
            "1RB 0LB 1R_ 3LA  0LC 3RB 3RC 1LB  2RB 2LA 3RA 1LC",
            "1RB 3LA 4LB 0RB 1RA 3LA  2LA 2RA 4LA 1RA 5RB 1R_",
        )

        for prog in progs:
            self.assertIsNotNone(
                Machine(
                    prog,
                    opt_macro = 500,
                ).run().limrul)

    def test_config_limit(self):
        self.assertIsNotNone(
            Machine(
                "1RB 1LA  0RC 1RC  1LD 0RB  0LD 1LA",
                blocks = 4,
            ).run().cfglim
        )

    def test_reasoner(self):
        self.assertFalse(
            cant_halt(
                "1RB 0RA  1LA 1R_"))

        self.assertFalse(
            cant_blank(
                "1RB ...  1LB 0RB"))

        self.assertFalse(
            cant_blank(
                "1RB 0RA  1LB 1LA"))

        self.assertTrue(
            cant_blank(
                "1RB 2LA 1LA  2LA 2RB 0RA"))

        self.assertFalse(
            cant_spin_out(
                "1RB 0RB 0LB  1LB 2RA 1LA"))

        self.assertFalse(
            cant_halt(
                "1RB ...  1LC 0RC  1RA 0LC"))

        _ = instr_seq(
            "1RB 1LB  1LA 1R_")

        _ = instr_seq(
            "1RB ...  0RC 0LA  1LC 1LD  0RB 0RD")

    def test_machine_macros(self):
        self.assertIsNotNone(
            Machine(
                "1RB 1LB  1LA 1R_",
                opt_macro = 10,
            ))

        _ = Machine(
            "1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  1R_ 0LA",
            opt_macro = 100,
            backsym = 1,
        )

    def test_run_variations(self):
        self.assertEqual(
            3,
            len(
                list(
                    run_variations(
                        "1RB 1LB  1LA 1R_",
                        sim_lim = 10))))

    def test_branch_init(self):
        self.assertEqual(
            8,
            len(
                Program.branch_init(
                    2, 2)))

    def test_tape(self):
        self.assertIsNotNone(
            Machine(
                "1RB 0LB  1RC 0RC  1LB 1LA"
            ).run())

    def test_diff_lim(self):
        self.assertIsNotNone(
            Machine(
                "1RB 1LA  1LC 0RD  ... 0RA  1LD 0LA"
            ).run())

    def test_num(self):
        marks = Machine(
            "1RB 1R_  0LC 0LD  1LD 1LC  1RE 1LB  1RF 1RD  0LD 0RA",
            opt_macro = 56,
        ).run().marks

        print(marks)
        assert not isinstance(marks, int)
        print(marks.estimate())
        more_marks = marks * 3 // 2
        assert not isinstance(more_marks, int)
        print(more_marks.estimate())

        self.assertGreater(marks, 5)
        self.assertGreaterEqual(marks, 5)
        self.assertLess(5, marks)
        self.assertLessEqual(5, marks)
        _ = marks < 5

    def test_algebra_compiler_error(self):
        progs = (
            "1RB 0LE  0RC 1RB  0RD 1RA  1LD 1LA  1LC 0RB",
            "1RB 1RA  0LC 1RE  0LE 1LD  0LB 1LC  1LA 0LE",
        )

        for prog in progs:
            self.assertIsNotNone(
                Machine(prog).run())

    def test_uninvertible(self):
        self.assertIsNotNone(
            Machine(
                "1RB 0LE  1RC 1RA  1RD 0LA  0LA 1LD  0RB 1LA",
                opt_macro = 63,
            ).run())

    def test_depth_limit(self):
        self.assertIsNotNone(
            Machine(
                "1RB 1RE  1LC 0LE  1RD 0LB  1RE 0RA  1LE 1RD",
                opt_macro = 73,
            ).run().limrul)

        # self.assertIsNotNone(
        #     Machine(
        #         "1RB 0RA  1RC 1RE  1LD 0LA  1LC 0RD  0RB 1RB",
        #         opt_macro = 40,
        #     ).run().limrul)

    def test_exp_mod_limit(self):
        self.assertIsNotNone(
            machine := Machine(
                "1RB 0LD  1RC 1LB  1LA 1RE  1LE 1LA  1RC 0RA",
                opt_macro = 41,
            ).run())

        print(machine)

    def test_exp_mod_special_cases(self):
        self.assertIsNotNone(
            Machine(
                "1RB 0LD  1RC 0RF  1LC 1LA  0LE 1R_  1LF 0RB  0RC 0RE",
            ).run().halted)
