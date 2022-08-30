from itertools import product
from typing import List, Tuple

from tm.parse import tcompile, dcompile, CompProg, Instr
from generate.program import Program

Color = int
State = int
Tape = List[Color]

class MacroConverter:
    def __init__(self, program: str):
        self.prog: CompProg = tcompile(program)

        self.states: int = len(self.prog)
        self.colors: int = len(self.prog[0])

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
                for tape in product(
                        range(self.colors),
                        repeat = cells)
            )
            for st_sh in range(2 * self.states)
        )


def run_macro_simulator(
        st_sh: State,
        tape: Tape,
        prog: CompProg,
        states: int,
        colors: int,
) -> Instr:
    state: State
    edge: int
    state, edge = divmod(st_sh, 2)

    cells: int = len(tape)

    pos: int = 0 if edge == 0 else cells - 1

    for _ in range((states * cells) * (colors ** cells)):
        scan: Color = tape[pos]

        assert (instr := prog[state][scan]) is not None

        color: Color
        shift: int
        next_state: State

        color, shift, next_state = instr

        tape[pos] = color

        pos += 1 if shift else -1

        if (state := next_state) == 30:
            return (
                tape_to_color(tape, colors),
                1,
                state,
            )

        if 0 <= pos < cells:
            continue

        next_edge: int = 0 if pos < 0 else 1
        edge_diff: int = 1 if pos < 0 else 0
        out_state: State = (2 * state) + edge_diff

        return (
            tape_to_color(tape, colors),
            next_edge,
            out_state,
        )

    return 0, 0, 0


def tape_to_color(tape: Tape, colors: int) -> Color:
    return int(
        ''.join(map(str, tape)),
        colors)
