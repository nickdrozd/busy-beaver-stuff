from itertools import product
from typing import List, Tuple

from tm.parse import tcompile, dcompile, CompProg, Instr
from generate.program import Program

Tape = List[int]

class MacroConverter:
    def __init__(self, program: str):
        self.prog = tcompile(program)
        self.states = len(self.prog)
        self.colors = len(self.prog[0])

    def macro_length(self, cells: int) -> Tuple[int, int]:
        return 2 * self.states, self.colors ** cells

    def macro_prog(self, cells: int) -> Program:
        return Program(
            dcompile(
                self.macro_comp(
                    cells))
        ).normalize()

    def macro_comp(self, cells: int) -> CompProg:
        return tuple(
            tuple(
                run_macro_simulator(
                    st_sh,
                    list(tape),
                    self.prog,
                    self.states,
                    self.colors,
                )
                for tape in product(range(self.colors), repeat = cells)
            )
            for st_sh in range(2 * self.states)
        )


def run_macro_simulator(
        st_sh: int,
        tape: Tape,
        prog,
        state_count: int,
        color_count: int,
) -> Instr:
    state, edge = divmod(st_sh, 2)

    pos = 0 if edge == 0 else len(tape) - 1

    max_config = state_count * len(tape) * color_count ** len(tape)

    for _ in range(max_config):
        scan = tape[pos]

        assert (instr := prog[state][scan]) is not None

        color, shift, next_state = instr

        tape[pos] = color

        pos += 1 if shift else -1

        if (state := next_state) == 30:
            return (
                tape_to_color(tape, color_count),
                1,
                state,
            )

        if 0 <= pos < len(tape):
            continue

        next_edge = 0 if pos < 0 else 1
        edge_diff = 1 if pos < 0 else 0
        out_state = (2 * state) + edge_diff

        return (
            tape_to_color(tape, color_count),
            next_edge,
            out_state,
        )

    return 0, 0, 0


def tape_to_color(tape: Tape, colors: int) -> int:
    return int(
        ''.join(map(str, tape)),
        colors)
