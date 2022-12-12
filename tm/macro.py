from __future__ import annotations

from math import ceil, log
from dataclasses import dataclass

from tm.parse import tcompile, CompProg

Color = int
State = int
Slot = tuple[Color, State]
Instr = tuple[int, int, int]

Tape = list[Color]
Config = tuple[State, tuple[bool, Tape]]

########################################

@dataclass
class MacroProg:
    program: str | MacroProg
    comp: CompProg | MacroProg

    base_states: int
    base_colors: int

    macro_states: int
    macro_colors: int

    sim_lim: int

    instrs: dict[Slot, Instr | None]

    color_to_tape_cache: dict[Color, tuple[Color, ...]]
    tape_to_color_cache: dict[tuple[Color, ...], Color]

    _state: State | None

    def __init__(self, program: str | MacroProg):
        self.program = program

        if isinstance(program, str):
            self.comp = tcompile(program)

            self.base_states = len(set(map(lambda s: s[0], self.comp)))
            self.base_colors = len(set(map(lambda s: s[1], self.comp)))
        else:
            self.comp = program

            self.base_states = program.macro_states
            self.base_colors = program.macro_colors

        self.instrs = {}

        self.color_to_tape_cache = {}
        self.tape_to_color_cache = {}

        self._state = None

    def __getitem__(self, slot: Slot) -> Instr | None:
        try:
            instr = self.instrs[slot]
        except KeyError:
            instr = self.calculate_instr(*slot)

            self.instrs[slot] = instr

        return instr

    def __len__(self) -> int:
        return len(self.instrs)

    def calculate_instr(
            self,
            macro_state: State,
            macro_color: Color,
    ) -> Instr | None:
        return self.reconstruct_outputs(result) if (
            result :=
            self.run_simulator(
                self.deconstruct_inputs(
                    macro_state,
                    macro_color))
        ) is not None else None

    def deconstruct_inputs(
            self,
            macro_state: State,
            macro_color: Color,
    ) -> Config:
        raise NotImplementedError()

    def reconstruct_outputs(self, config: Config) -> Instr:
        raise NotImplementedError()

    def run_simulator(self, config: Config) -> Config | None:
        state, (right_edge, tape) = config

        cells = len(tape)

        pos = cells - 1 if right_edge else 0

        for _ in range(self.sim_lim):
            if (instr := self.comp[
                    state, scan := tape[pos]]) is None:
                return None

            color, shift, next_state = instr

            if next_state != state:
                tape[pos] = color
                pos += 1 if shift else -1
            else:
                while tape[pos] == scan:  # pylint: disable = while-used
                    tape[pos] = color
                    pos += 1 if shift else -1

                    if not 0 <= pos < cells:
                        break

            if (state := next_state) == -1:
                break

            if not 0 <= pos < cells:
                break
        else:
            return None

        return state, (cells <= pos, tape)

########################################

@dataclass
class BlockMacro(MacroProg):
    cells: int

    def __init__(self, program: str | MacroProg, cell_seq: list[int]):
        *seq, cells = cell_seq

        if seq:
            program = BlockMacro(program, seq)

        super().__init__(program)

        self.macro_states = self.base_states * 2
        self.macro_colors = self.base_colors ** cells

        self.cells = cells

        self.sim_lim = (
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
    ) -> Config:
        state, right_edge = divmod(macro_state, 2)

        return state, (
            right_edge == 1,
            self.color_to_tape(macro_color),
        )

    def reconstruct_outputs(self, config: Config) -> Instr:
        state, (right_edge, tape) = config

        return (
            self.tape_to_color(tape),
            int(right_edge),
            (
                (2 * state) + int(not right_edge)
                if state != -1 else
                state
            ),
        )

    def tape_to_color(self, tape: Tape) -> Color:
        if (cached := self.tape_to_color_cache.get(
                tuple_tape := tuple(tape))) is not None:
            return cached

        color: Color = sum(
            value * self.base_colors ** place
            for place, value in enumerate(reversed(tape))
        )

        self.tape_to_color_cache[tuple_tape] = color
        self.color_to_tape_cache[color] = tuple_tape

        return color

    def color_to_tape(self, color: Color) -> Tape:
        if color == 0:
            return [0] * self.cells

        if (prev := self.color_to_tape_cache.get(color)) is not None:
            return list(prev)

        tape: Tape = []

        num = color

        for _ in range(ceil(log(color, self.base_colors)) + 1):
            num, rem = divmod(num, self.base_colors)

            tape.insert(0, rem)

            if num == 0:
                break

        for _ in range(self.cells - len(tape)):
            tape.insert(0, 0)

        self.color_to_tape_cache[color] = tuple(tape)

        return tape

########################################

@dataclass
class BacksymbolMacro(MacroProg):
    def __init__(self, program: str | MacroProg, cell_seq: list[int]):
        *seq, _ = cell_seq

        if seq:
            program = BacksymbolMacro(program, seq)

        super().__init__(program)

        self.macro_states = self.base_states * self.base_colors * 2
        self.macro_colors = self.base_colors

        self.sim_lim = self.macro_states * self.macro_colors

    def __str__(self) -> str:
        return f'{self.program} (backsymbol macro)'

    def deconstruct_inputs(
            self,
            macro_state: State,
            macro_color: Color,
    ) -> Config:
        st_co, at_right = divmod(macro_state, 2)

        state, backsymbol = divmod(st_co, self.base_colors)

        tape = [macro_color, backsymbol]

        if not at_right:
            tape.reverse()

        return state, (not at_right, tape)

    def reconstruct_outputs(self, config: Config) -> Instr:
        state, (right_edge, tape) = config

        if not right_edge:
            tape.reverse()

        out_color, backsymbol = tape

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
                state
            ),
        )
