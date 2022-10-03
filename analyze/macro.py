from __future__ import annotations

from typing import Any, Dict, List, Optional, Tuple, Union

from tm.parse import tcompile, Instr, ProgLike

Color = int
State = int
Tape = List[Color]
SimInput  = Tuple[State, bool, Tape]

########################################

class MacroProg:
    def __init__(self, program: ProgLike):
        self.program = program

        self.comp: Any

        self.base_states: int
        self.base_colors: int

        if isinstance(program, str):
            self.comp = tcompile(program)

            self.base_states = len(self.comp)
            self.base_colors = len(self.comp[0])
        else:
            self.comp = program

            self.base_states = program.macro_states
            self.base_colors = program.macro_colors

        self.sim_lim: int = 0

        self.instrs: Dict[Tuple[State, Color], Optional[Instr]] = {}

        self.tape_colors: Dict[Color, Tuple[Color, ...]] = {}

        self._state: Optional[State] = None

    def __getitem__(
            self,
            stco: Union[State, Color],
    ) -> Union[MacroProg, Optional[Instr]]:
        if self._state is None:
            self._state = stco
            return self

        state, color = self._state, stco
        self._state = None

        try:
            instr = self.instrs[(state, color)]
        except KeyError:
            instr = self.calculate_instr(state, color)

            self.instrs[(state, color)] = instr

        return instr

    def __len__(self) -> int:
        return len(self.instrs)

    def calculate_instr(
            self,
            macro_state: State,
            macro_color: Color,
    ) -> Optional[Instr]:
        return self.reconstruct_outputs(*result) if (
            result :=
            self.run_simulator(
                *self.deconstruct_inputs(
                    macro_state,
                    macro_color))
        ) is not None else None

    def deconstruct_inputs(
            self,
            macro_state: State,
            macro_color: Color,
    ) -> SimInput:
        raise NotImplementedError()

    def reconstruct_outputs(
            self,
            state: State,
            tape: Tape,
            right_edge: bool,
    ) -> Instr:
        raise NotImplementedError()

    def run_simulator(
            self,
            state: State,
            right_edge: bool,
            tape: Tape,
    ) -> Optional[Tuple[State, Tape, bool]]:
        cells = len(tape)

        pos = cells - 1 if right_edge else 0

        for _ in range(self.sim_lim):
            if (instr := self.comp[state][tape[pos]]) is None:
                return None

            color, shift, next_state = instr

            tape[pos] = color

            pos += 1 if shift else -1

            if (state := next_state) == -1:
                break

            if not 0 <= pos < cells:
                break

        return state, tape, cells <= pos

########################################

class BlockMacro(MacroProg):
    def __init__(self, program: ProgLike, cell_seq: List[int]):
        *seq, cells = cell_seq

        if seq:
            program = BlockMacro(program, seq)

        super().__init__(program)

        self.macro_states: int = self.base_states * 2
        self.macro_colors: int = self.base_colors ** cells

        self.cells: int = cells

        self.sim_lim: int = (
            self.base_states
            * self.cells
            * self.macro_colors
        )

    def __str__(self) -> str:
        return f'{self.program} ({self.cells}-cell macro)'

    def deconstruct_inputs(
            self,
            macro_state: State,
            macro_color: Color,
    ) -> SimInput:
        in_state, right_edge = divmod(macro_state, 2)

        return (
            in_state,
            right_edge == 1,
            self.color_to_tape(macro_color),
        )

    def reconstruct_outputs(
            self,
            state: State,
            tape: Tape,
            right_edge: bool,
    ) -> Instr:
        return (
            self.tape_to_color(tape),
            int(right_edge),
            (
                (2 * state) + int(not right_edge)
                if state != -1 else
                -1
            ),
        )

    def tape_to_color(self, tape: Tape) -> Color:
        try:
            color = int(
                ''.join(map(str, tape)),
                self.base_colors)
        except ValueError:
            color = sum(
                value * self.base_colors ** place
                for place, value in enumerate(reversed(tape))
            )

        self.tape_colors[color] = tuple(tape)

        return color

    def color_to_tape(self, color: Color) -> Tape:
        if color == 0:
            return [0] * self.cells

        return list(self.tape_colors[color])

########################################

class BacksymbolMacro(MacroProg):
    def __init__(self, program: ProgLike, cell_seq: List[int]):
        *seq, _ = cell_seq

        if seq:
            program = BacksymbolMacro(program, seq)

        super().__init__(program)

        self.macro_states: int = self.base_states * self.base_colors * 2
        self.macro_colors: int = self.base_colors

        self.sim_lim: int = self.macro_states * self.macro_colors

    def __str__(self) -> str:
        return f'{self.program} (backsymbol macro)'

    def deconstruct_inputs(
            self,
            macro_state: State,
            macro_color: Color,
    ) -> SimInput:
        st_co, backsymbol_to_right = divmod(macro_state, 2)

        in_mini_state, in_backsymbol = divmod(st_co, self.base_colors)

        at_right = bool(backsymbol_to_right)

        return (
            in_mini_state,
            not at_right,
            (
                [macro_color, in_backsymbol]
                if at_right else
                [in_backsymbol, macro_color]
            ),
        )

    def reconstruct_outputs(
            self,
            state: State,
            tape: Tape,
            right_edge: bool,
    ) -> Instr:
        out_color, backsymbol = (
            tape
            if right_edge else
            reversed(tape)
        )

        return (
            out_color,
            not right_edge,
            (
                int(not right_edge)
                + (2
                   * (backsymbol
                      + (state
                         * self.base_colors)))
                if state != -1 else
                -1
            ),
        )
