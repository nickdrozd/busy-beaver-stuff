from unittest import TestCase

from tm.macro import BacksymbolLogic, make_macro, opt_block

MACROS = {
    ("0RB 0LC  1LA 1RB  1RD 0RE  1LC 1LA  ... 0LD", 2): {
        (0, 0): (1, 1, 1),
        (1, 0): (2, 0, 2),
        (2, 1): (2, 0, 3),
        (3, 0): (1, 1, 4),
        (4, 2): (2, 0, 2),
    }
}


class TestMacro(TestCase):
    def test_macro(self):
        for (prog, blocks), instrs in MACROS.items():
            macro = make_macro(prog, blocks = blocks)

            for slot, instr in instrs.items():
                self.assertEqual(macro[slot], instr)

    def test_backsym_reconstruct(self):
        logic = BacksymbolLogic(
            cells = 2,
            base_states = 2,
            base_colors = 3)

        config = 0, (False, (1, 2, 0))

        instr = logic.reconstruct_outputs(config)

        self.assertEqual(instr, (0, True, 11))



BLOCKS = {
    "1RB 0LC 0LC  2LA 0RA 1RB  1LA 2LB ...": {
        500: 8,
        1000: 1,
        2000: 45,
    },
}


class TestBlocks(TestCase):
    def test_blocks(self):
        for prog, block_steps in BLOCKS.items():
            for steps, blocks in block_steps.items():
                self.assertEqual(
                    blocks,
                    opt_block(prog, steps))
