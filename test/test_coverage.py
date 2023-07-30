from unittest import TestCase

from tm.reason import Program, BackwardReasoner
from tm.machine import Machine, LinRecMachine, run_variations


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
            ).run(prover = False).simple_termination,
            15721562)

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
        self.assertIsNotNone(
            Machine(
                "1RB 2RB 3RB 4RB 5LA 4RA  0LA 1RB 5RA ... ... 1LB",
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
            BackwardReasoner("1RB 0RA  1LA 1R_").cant_halt)

        self.assertFalse(
            BackwardReasoner("1RB ...  1LB 0RB").cant_blank)

        self.assertFalse(
            BackwardReasoner("1RB 0RA  1LB 1LA").cant_blank)

        self.assertTrue(
            BackwardReasoner("1RB 2LA 1LA  2LA 2RB 0RA").cant_blank)

        self.assertFalse(
            BackwardReasoner("1RB 0RB 0LB  1LB 2RA 1LA").cant_spin_out)

        self.assertFalse(
            BackwardReasoner("1RB ...  1LC 0RC  1RA 0LC").cant_halt)

        _ = BackwardReasoner("1RB 1LB  1LA 1R_").instr_seq
        _ = BackwardReasoner(
            "1RB ...  0RC 0LA  1LC 1LD  0RB 0RD").instr_seq

    def test_machine_macros(self):
        self.assertIsNotNone(
            Machine(
                "1RB 1LB  1LA 1R_",
                opt_blocks = 10,
            ))

        _ = Machine(
            "1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  1R_ 0LA",
            opt_blocks = 100,
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
