from __future__ import annotations

from dataclasses import dataclass
from collections.abc import Iterator

from tm.parse import tcompile
from tm.instrs import Color, State, Slot, Instr, GetInstr

Tape = list[Color]
Config = tuple[State, tuple[bool, Tape]]

########################################

@dataclass
class MacroProg:
    program: str | MacroProg
    comp: GetInstr

    base_states: int
    base_colors: int

    macro_states: int
    macro_colors: int

    sim_lim: int

    instrs: dict[Slot, Instr | None]

    cells: int

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

        seen: set[tuple[State, int, tuple[Color, ...]]] = set()

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

            if (curr := (state, pos, tuple(tape))) in seen:
                return None

            seen.add(curr)

        else:
            return None

        return state, (cells <= pos, tape)

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

        assert (prev := self.color_to_tape_cache.get(color)) is not None

        return list(prev)

########################################

@dataclass
class BlockMacro(MacroProg):
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
        return f'{self.program} ({self.cells}-cell block macro)'

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
            right_edge,
            (
                (2 * state) + int(not right_edge)
                if state != -1 else
                state
            ),
        )

########################################

@dataclass
class BacksymbolMacro(MacroProg):
    backsymbols: int

    def __init__(self, program: str | MacroProg, cell_seq: list[int]):
        *seq, cells = cell_seq

        if seq:
            program = BacksymbolMacro(program, seq)

        super().__init__(program)

        self.macro_colors = self.base_colors

        self.cells = cells

        self.backsymbols = self.base_colors ** self.cells

        self.macro_states = 2 * self.base_states * self.backsymbols

        self.sim_lim = self.macro_states * self.macro_colors

    def __str__(self) -> str:
        return f'{self.program} ({self.cells}-cell backsymbol macro)'

    def deconstruct_inputs(
            self,
            macro_state: State,
            macro_color: Color,
    ) -> Config:
        st_co, at_right = divmod(macro_state, 2)

        state, backsymbol = divmod(st_co, self.backsymbols)

        tape = (
            [macro_color] + self.color_to_tape(backsymbol)
            if at_right else
            self.color_to_tape(backsymbol) + [macro_color]
        )

        return state, (not at_right, tape)

    def reconstruct_outputs(self, config: Config) -> Instr:
        state, (right_edge, tape) = config

        out_color, backsymbol = (
            (tape[0], tape[1:])
            if right_edge else
            (tape[-1], tape[:-1])
        )

        return (
            out_color,
            not right_edge,
            (
                int(not right_edge)
                + (2
                   * (self.tape_to_color(backsymbol)
                      + (state
                         * self.backsymbols)))
                if state != -1 else
                state
            ),
        )

########################################

def macro_variations(
        prog: str,
        max_block: int,
        back_wrap: int,
) -> Iterator[str | MacroProg]:
    yield prog

    for block in range(2, 1 + max_block):
        yield BacksymbolMacro(
            BlockMacro(
                prog, [block]), [1])

    for wrap in range(1, 1 + back_wrap):
        yield BacksymbolMacro(prog, [1] * wrap)
