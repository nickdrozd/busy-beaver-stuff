from unittest import TestCase

from tm.macro import make_macro

MACROS = {
    ("0RB 0LC  1LA 1RB  1RD 0RE  1LC 1LA  ... 0LD", 2): {
        (0, 0): (1, 1, 2),
        (2, 0): (2, 0, 1),
        (1, 1): (2, 0, 5),
        (5, 0): (1, 1, 6),
        (6, 2): (2, 0, 1),
    }
}


class TestMacro(TestCase):
    def test_macro(self):
        for (prog, blocks), instrs in MACROS.items():
            macro = make_macro(prog, blocks = blocks)

            for slot, instr in instrs.items():
                self.assertEqual(macro[slot], instr)
