from itertools import product
from typing import List, Tuple

from tm.parse import tcompile, dcompile
from generate.program import Program


class MacroConverter:
    def __init__(self, program: str):
        self.prog = tcompile(program)
        self.states = len(self.prog)
        self.colors = len(self.prog[0])

    def macro_length(self, cells: int) -> Tuple[int, int]:
        return 2 * self.states, self.colors ** cells

    def macro_prog(self, cells: int) -> str:
        return Program(
            dcompile(
                self.macro_comp(
                    cells))
        ).normalize()

    def macro_comp(self, cells: int):
        return tuple(
            tuple(
                self.run(st_sh, list(tape))
                for tape in product(range(self.colors), repeat = cells)
            )
            for st_sh in range(2 * self.states)
        )

    def run(self, st_sh: int, tape: List[int]):
        state, edge = divmod(st_sh, 2)

        pos = 0 if edge == 0 else len(tape) - 1

        max_config = self.states * len(tape) * self.colors ** len(tape)

        for _ in range(max_config):
            scan = tape[pos]

            assert (instr := self.prog[state][scan]) is not None

            color, shift, next_state = instr

            tape[pos] = color

            pos += 1 if shift else -1

            state = next_state

            # pylint: disable = consider-using-assignment-expr
            if state == 30:
                return (
                    self.tape_to_color(tape),
                    1,
                    state,
                )

            if 0 <= pos < len(tape):
                continue

            next_edge = 0 if pos < 0 else 1
            edge_diff = 1 if pos < 0 else 0
            out_state = (2 * state) + edge_diff

            return (
                self.tape_to_color(tape),
                next_edge,
                out_state,
            )

    def tape_to_color(self, tape):
        return int(
            ''.join(map(str, tape)),
            self.colors)
