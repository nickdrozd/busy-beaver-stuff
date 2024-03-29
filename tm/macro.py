from __future__ import annotations

from abc import abstractmethod
from typing import TYPE_CHECKING
from collections import defaultdict

from tm.parse import comp_thin

if TYPE_CHECKING:
    from tm.parse import Color, State, Slot, Instr, GetInstr

    Tape = list[Color]
    Config = tuple[State, tuple[bool, Tape]]

########################################

CONVERTERS: dict[
    int,
    dict[int, TapeColorConverter],
] = defaultdict(dict)


def make_converter(base_colors: int, cells: int) -> TapeColorConverter:
    if (cached := CONVERTERS[base_colors].get(cells)) is not None:
        return cached

    converter = TapeColorConverter(base_colors, cells)
    CONVERTERS[base_colors][cells] = converter
    return converter


class TapeColorConverter:
    base_colors: int

    color_to_tape_cache: dict[Color, tuple[Color, ...]]
    tape_to_color_cache: dict[tuple[Color, ...], Color]

    def __init__(self, base_colors: int, cells: int):
        self.base_colors = base_colors

        self.tape_to_color_cache = {}
        self.color_to_tape_cache = { 0: (0,) * cells }

    def color_to_tape(self, color: Color) -> Tape:
        assert (prev := self.color_to_tape_cache.get(color)) is not None

        return list(prev)

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

########################################

class MacroInfLoop(Exception):
    pass


def prog_params(program: str | MacroProg) -> tuple[GetInstr, int, int]:
    comp: GetInstr

    if isinstance(program, str):
        comp = comp_thin(program)

        base_states = len(set(map(lambda s: s[0], comp)))
        base_colors = len(set(map(lambda s: s[1], comp)))

    else:
        comp = program

        base_states = program.macro_states
        base_colors = program.macro_colors

    return comp, base_states, base_colors


class MacroProg:
    @property
    @abstractmethod
    def comp(self) -> GetInstr: ...

    @property
    @abstractmethod
    def base_states(self) -> int: ...

    @property
    @abstractmethod
    def base_colors(self) -> int: ...

    @property
    @abstractmethod
    def macro_states(self) -> int: ...

    @property
    @abstractmethod
    def macro_colors(self) -> int: ...

    @property
    @abstractmethod
    def sim_lim(self) -> int: ...

    @property
    @abstractmethod
    def instrs(self) -> dict[Slot, Instr]: ...

    def __getitem__(self, slot: Slot) -> Instr:
        try:
            instr = self.instrs[slot]
        except KeyError:
            instr = self.calculate_instr(*slot)

            self.instrs[slot] = instr

        return instr

    def __len__(self) -> int:
        return len(self.instrs)

    @abstractmethod
    def __str__(self) -> str: ...

    def calculate_instr(
            self,
            macro_state: State,
            macro_color: Color,
    ) -> Instr:
        return self.reconstruct_outputs(
            self.run_simulator(
                self.deconstruct_inputs(
                    macro_state,
                    macro_color)))

    @abstractmethod
    def deconstruct_inputs(
            self,
            macro_state: State,
            macro_color: Color,
    ) -> Config: ...

    @abstractmethod
    def reconstruct_outputs(self, config: Config) -> Instr: ...

    def run_simulator(self, config: Config) -> Config:
        state, (right_edge, tape) = config

        cells = len(tape)

        pos = cells - 1 if right_edge else 0

        seen: set[tuple[State, int, tuple[Color, ...]]] = set()

        for _ in range(self.sim_lim):  # no-branch
            color, shift, next_state = \
                self.comp[state, scan := tape[pos]]

            if next_state != state:
                tape[pos] = color
                pos += 1 if shift else -1
            else:
                while tape[pos] == scan:  # pylint: disable = while-used
                    tape[pos] = color
                    pos += 1 if shift else -1

                    if not 0 <= pos < cells:
                        break

            state = next_state

            if not 0 <= pos < cells:
                break

            if (curr := (state, pos, tuple(tape))) in seen:
                raise MacroInfLoop

            seen.add(curr)

        else:
            raise MacroInfLoop  # no-cover

        return state, (cells <= pos, tape)

########################################

class BlockMacro(MacroProg):
    program: str | GetInstr

    cells: int

    _comp: GetInstr

    _base_states: int
    _base_colors: int

    _instrs: dict[Slot, Instr]

    converter: TapeColorConverter

    def __init__(self, program: str | MacroProg, cells: int):
        self.program = program

        self._comp, self._base_states, self._base_colors = \
            prog_params(program)

        self._instrs = {}

        self.cells = cells

        self.converter = make_converter(self.base_colors, self.cells)

    def __str__(self) -> str:
        return f'{self.program} ({self.cells}-cell block macro)'

    @property
    def comp(self) -> GetInstr:
        return self._comp

    @property
    def base_states(self) -> int:
        return self._base_states

    @property
    def base_colors(self) -> int:
        return self._base_colors

    @property
    def macro_states(self) -> int:
        return 2 * self.base_states

    @property
    def macro_colors(self) -> int:
        macro_colors: int = self.base_colors ** self.cells

        return macro_colors

    @property
    def sim_lim(self) -> int:
        return (
            self.base_states
            * self.cells
            * self.macro_colors
        )

    @property
    def instrs(self) -> dict[Slot, Instr]:
        return self._instrs

    def deconstruct_inputs(
            self,
            macro_state: State,
            macro_color: Color,
    ) -> Config:
        state, right_edge = divmod(macro_state, 2)

        return state, (
            right_edge == 1,
            self.converter.color_to_tape(macro_color),
        )

    def reconstruct_outputs(self, config: Config) -> Instr:
        state, (right_edge, tape) = config

        return (
            self.converter.tape_to_color(tape),
            right_edge,
            (2 * state) + int(not right_edge),
        )

########################################

class BacksymbolMacro(MacroProg):
    program: str | GetInstr

    cells: int
    backsymbols: int

    _comp: GetInstr

    _base_states: int
    _base_colors: int

    _instrs: dict[Slot, Instr]

    converter: TapeColorConverter

    def __init__(self, program: str | MacroProg, cells: int):
        self.program = program

        self._comp, self._base_states, self._base_colors = \
            prog_params(program)

        self._instrs = {}

        self.cells = cells

        self.backsymbols = self.base_colors ** self.cells

        self.converter = make_converter(self.base_colors, self.cells)

    def __str__(self) -> str:
        return f'{self.program} ({self.cells}-cell backsymbol macro)'

    @property
    def comp(self) -> GetInstr:
        return self._comp

    @property
    def base_states(self) -> int:
        return self._base_states

    @property
    def base_colors(self) -> int:
        return self._base_colors

    @property
    def macro_states(self) -> int:
        return 2 * self.base_states * self.backsymbols

    @property
    def macro_colors(self) -> int:
        return self.base_colors

    @property
    def sim_lim(self) -> int:
        return self.macro_states * self.macro_colors

    @property
    def instrs(self) -> dict[Slot, Instr]:
        return self._instrs

    def deconstruct_inputs(
            self,
            macro_state: State,
            macro_color: Color,
    ) -> Config:
        st_co, at_right = divmod(macro_state, 2)

        state, backsymbol = divmod(st_co, self.backsymbols)

        tape = (
            [macro_color] + self.converter.color_to_tape(backsymbol)
            if at_right else
            self.converter.color_to_tape(backsymbol) + [macro_color]
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
            int(not right_edge)
            + (2
               * (self.converter.tape_to_color(backsymbol)
                  + (state
                     * self.backsymbols)))
            ,
        )
