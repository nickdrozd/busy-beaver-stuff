from unittest import TestCase

from tm.machine import Machine, LinRecMachine
from tm.macro import BlockMacro, BacksymbolMacro


class TestDisplay(TestCase):
    def test_display(self):
        machine = Machine(
            "1RB 2LA 1R_  1LB 1LA 0RA"
        ).run(watch_tape = True)

        print(machine)

        machine = Machine(
            "1RB 1RA  0RC 0RB  1LC 1LD  1RA 1LB"
        ).run(prover = True, watch_tape = True)

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
    # pylint: disable = no-self-use
    def test_macro(self):
        _ = Machine(
            BlockMacro(
                "1RB 2LA 1RA 1RA  1LB 1LA 3RB 1R_",
                [2])
        ).run(prover = True)

        _ = Machine(
            macro := BacksymbolMacro(
                BlockMacro(
                    "1RB 0RA 1LB  2LA 2RB 0LA",
                    [3, 3]),
                [1, 1])
        ).run()

        print(len(macro), macro)

    def test_machine(self):
        _ = Machine(
            "1RB 1RA  1RC 0RD  1LE 0RA  0R_ 0RB  1LB 1LE"
        ).run().simple_termination

        # pylint: disable = redefined-variable-type
        _ = LinRecMachine(
            "1RB 0LB  1LA 0RB"
        ).run(check_rec = 1)

    def test_prover(self):
        _ = Machine(
            "1RB 2LA 1R_ 5LB 5LA 4LB  1LA 4RB 3RB 5LB 1LB 4RA"
        ).run(prover = True)
