from unittest import TestCase

from test.prog_data import BRANCH, PROGS, NORMALIZE

from tm.program import Program
from tm.show import show_slot, show_state
from tools.normalize import Normalizer


class TestProgram(TestCase):
    prog: Program

    def assert_used_states(self, states: set[str]):
        self.assertEqual(
            states,
            set(map(show_state, self.prog.used_states)))

    def assert_available_states(self, states: set[str]):
        self.assertEqual(
            states,
            set(map(show_state, self.prog.available_states)))

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
            slot,
            None
            if (last := self.prog.last_slot) is None else
            show_slot(last))

    def assert_slots(self, slots: tuple[str, ...]):
        self.assertEqual(
            slots,
            tuple(map(show_slot, self.prog.slots)))

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
