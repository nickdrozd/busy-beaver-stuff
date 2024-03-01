from unittest import TestCase

from test.prog_data import BRANCH, PROGS, NORMALIZE

from tm.program import Program
from tm.show import show_slot, show_instr
from tools.normalize import Normalizer


class TestProgram(TestCase):
    prog: Program

    def assert_last_slot(self, slot: str | None):
        self.assertEqual(
            slot,
            None
            if (last := self.prog.last_slot) is None else
            show_slot(last))

    def assert_slots(self, slots: tuple[str, ...]):
        self.assertEqual(
            slots,
            tuple(map(show_slot, self.prog.slots)))

    def assert_instrs(self, instrs: tuple[str, ...]):
        self.assertEqual(
            instrs,
            tuple(map(show_instr,reversed(self.prog.available_instrs))))

    def test_used_available(self):
        for prog, (last, slots, avail_instr) in PROGS.items():
            self.prog = Program(prog)

            self.assert_last_slot(last)
            self.assert_slots(slots)
            self.assert_instrs(avail_instr)

    def test_branch(self):
        for (prog, loc), extensions in BRANCH.items():
            self.assertEqual(
                set(Program(prog).branch_read(loc)),
                extensions)

    def test_normalize(self):
        progs = set()

        for norm, devs in NORMALIZE.items():
            for dev in devs:
                normalized = Normalizer(dev)

                self.assertEqual(
                    norm,
                    normalized.normalize())

                program = Program(dev)

                progs.add(program)
                progs.add(normalized)

                self.assertIn(program, progs)
                self.assertIn(normalized, progs)

    def test_branch_init(self):
        self.assertEqual(
            sorted(Program.branch_init(2, 2)),
            sorted(BRANCH[("1RB ...  ... ...", 'B0')]))
