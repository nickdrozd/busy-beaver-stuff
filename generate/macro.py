from itertools import product
from typing import Dict, List, Optional, Tuple, Union

from tm.parse import tcompile, dcompile, CompProg, Instr
from generate.program import Program

Color = int
State = int
Tape = List[Color]

########################################

class MacroRunner:
    def __init__(self, program: str):
        self.program: str = program

        self.comp: CompProg = tcompile(program)

        self.states: int = len(self.comp)
        self.colors: int = len(self.comp[0])

    def run_macro_simulator(
            self,
            st_sh: State,
            tape: Tape,
    ) -> Instr:
        state, edge = divmod(st_sh, 2)

        cells = len(tape)

        pos = 0 if edge == 0 else cells - 1

        for _ in range((self.states * cells) * (self.colors ** cells)):
            assert (instr := self.comp[state][tape[pos]]) is not None

            color, shift, next_state = instr

            tape[pos] = color

            pos += 1 if shift else -1

            if (state := next_state) == -1:
                return (
                    self.tape_to_color(tape),
                    1,
                    state,
                )

            if 0 <= pos < cells:
                continue

            next_edge = 0 if pos < 0 else 1
            edge_diff = 1 if pos < 0 else 0
            out_state = (2 * state) + edge_diff

            return (
                self.tape_to_color(tape),
                next_edge,
                out_state,
            )

        return 0, 0, 0

    def tape_to_color(self, tape: Tape) -> Color:
        return int(
            ''.join(map(str, tape)),
            self.colors)

########################################

class MacroCompiler(MacroRunner):
    def macro_prog(self, cells: int) -> Program:
        return Program(
            dcompile(
                self.macro_comp(
                    cells))
        ).normalize()

    def macro_comp(self, cells: int) -> CompProg:
        return tuple(
            tuple(
                self.run_macro_simulator(
                    st_sh,
                    list(tape),
                )
                for tape in product(
                        range(self.colors),
                        repeat = cells)
            )
            for st_sh in range(2 * self.states)
        )

########################################

class DynamicMacroProg(MacroRunner):
    def __init__(self, program: str, cells: int):
        super().__init__(program)

        self.cells: int = cells

        self.instrs: Dict[Tuple[State, Color], Instr] = {}

        self.tape_colors: Dict[Color, Tuple[Color, ...]] = {}

        self._state: Optional[State] = None

    def __str__(self) -> str:
        return f'{self.program} ({self.cells}-cell macro)'

    def __getitem__(self, stco: Union[State, Color]) -> Instr:
        if self._state is None:
            self._state = stco
            return self  # type: ignore

        state, color = self._state, stco
        self._state = None

        try:
            return self.instrs[(state, color)]
        except KeyError:
            instr = self.run_macro_simulator(
                state,
                self.color_to_tape(color),
            )

            self.instrs[(state, color)] = instr

            return instr

    def tape_to_color(self, tape: Tape) -> Color:
        color = super().tape_to_color(tape)

        self.tape_colors[color] = tuple(tape)

        return color

    def color_to_tape(self, color: Color) -> Tape:
        if color == 0:
            return [0] * self.cells

        return list(self.tape_colors[color])
