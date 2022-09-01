from itertools import product
from typing import Callable, Dict, List, Tuple

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
        state: State
        edge: int
        state, edge = divmod(st_sh, 2)

        cells: int = len(tape)

        pos: int = 0 if edge == 0 else cells - 1

        for _ in range((self.states * cells) * (self.colors ** cells)):
            scan: Color = tape[pos]

            assert (instr := self.comp[state][scan]) is not None

            color: Color
            shift: int
            next_state: State

            color, shift, next_state = instr

            tape[pos] = color

            pos += 1 if shift else -1

            if (state := next_state) == 30:
                return (
                    self.tape_to_color(tape),
                    1,
                    state,
                )

            if 0 <= pos < cells:
                continue

            next_edge: int = 0 if pos < 0 else 1
            edge_diff: int = 1 if pos < 0 else 0
            out_state: State = (2 * state) + edge_diff

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

class MacroState:
    # pylint: disable = too-few-public-methods
    def __init__(self, calculate: Callable[[Color], Instr]):
        self.instrs: Dict[Color, Instr] = {}
        self.calculate = calculate

    def __getitem__(self, color: Color) -> Instr:
        try:
            return self.instrs[color]
        except KeyError:
            instr = self.calculate(color)
            self.instrs[color] = instr
            return instr


class DynamicMacroProg(MacroRunner):
    def __init__(self, program: str, cells: int):
        super().__init__(program)

        self.cells: int = cells

        self.macro: Dict[State, MacroState] = {}

        self.tape_colors: Dict[Color, Tuple[Color, ...]] = {}

    def __str__(self) -> str:
        return f'{self.program} ({self.cells}-cell macro)'

    def __getitem__(self, state: State) -> MacroState:
        try:
            return self.macro[state]
        except KeyError:
            macro_state = MacroState(
                self.make_instr_calculator(state))
            self.macro[state] = macro_state
            return macro_state

    def make_instr_calculator(
            self,
            state: State,
    ) -> Callable[[Color], Instr]:

        def calculate_instr(color: Color) -> Instr:
            return self.run_macro_simulator(
                state,
                self.color_to_tape(color),
            )

        return calculate_instr

    def tape_to_color(self, tape: Tape) -> Color:
        color = super().tape_to_color(tape)

        self.tape_colors[color] = tuple(tape)

        return color

    def color_to_tape(self, color: Color) -> Tape:
        if color == 0:
            return [0] * self.cells

        return list(self.tape_colors[color])
