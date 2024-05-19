from __future__ import annotations

from abc import abstractmethod
from typing import TYPE_CHECKING, Protocol
from collections import defaultdict

from tm.show import show_comp
from tm.parse import tcompile
from tm.rust_stuff import opt_block

if TYPE_CHECKING:
    from tm.parse import Color, State, Slot, Instr, CompProg

    Tape = tuple[Color, ...]
    Config = tuple[State, tuple[bool, Tape]]
    Params = tuple[int, int]


class GetInstr(Protocol):
    def __getitem__(self, slot: Slot) -> Instr: ...

########################################

def make_macro(
        prog: str,
        *,
        blocks: int | None = None,
        backsym: int | None = None,
        opt_macro: int | None = None,
        params: Params | None = None,
) -> GetInstr:
    comp: GetInstr = tcompile(prog)

    if opt_macro is not None:
        blocks = opt_block(prog, opt_macro)

    if blocks is not None and blocks > 1:
        if params is None:
            params = prog_params(comp)

        comp = make_block_macro(comp, blocks, params)

    if backsym is not None:
        if params is None or blocks is not None:
            params = prog_params(comp)

        comp = make_backsymbol_macro(comp, backsym, params)

    return comp


def make_block_macro(
        comp: GetInstr,
        blocks: int,
        params: Params,
) -> MacroProg:
    return MacroProg(comp, BlockLogic(blocks, *params))


def make_backsymbol_macro(
        comp: GetInstr,
        backsym: int,
        params: Params,
) -> MacroProg:
    return MacroProg(comp, BacksymbolLogic(backsym, *params))

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

        return prev

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


def prog_params(comp: GetInstr) -> Params:
    if isinstance(comp, MacroProg):
        base_states = comp.macro_states
        base_colors = comp.macro_colors

    else:
        assert isinstance(comp, dict)

        base_states = len(set(map(lambda s: s[0], comp)))
        base_colors = len(set(map(lambda s: s[1], comp)))

    return base_states, base_colors


class MacroProg:
    comp: GetInstr
    instrs: CompProg

    sim_lim: int

    logic: Logic

    def __init__(self, comp: GetInstr, logic: Logic):
        self.comp = comp
        self.instrs = {}

        self.logic = logic

        self.sim_lim = self.logic.sim_lim

    def __str__(self) -> str:
        comp_str = (
            show_comp(comp)
            if isinstance(comp := self.comp, dict) else
            str(comp)
        )

        return f'{comp_str} ({self.cells}-cell {self.logic.name} macro)'

    @property
    def macro_states(self) -> int:
        return self.logic.macro_states

    @property
    def macro_colors(self) -> int:
        return self.logic.macro_colors

    @property
    def cells(self) -> int:
        return self.logic.cells

    def __getitem__(self, slot: Slot) -> Instr:
        try:
            instr = self.instrs[slot]
        except KeyError:
            instr = self.calculate_instr(slot)

            self.instrs[slot] = instr

        return instr

    def calculate_instr(self, slot: Slot) -> Instr:
        return self.logic.reconstruct_outputs(
            self.run_simulator(
                self.logic.deconstruct_inputs(
                    slot)))

    def run_simulator(self, config: Config) -> Config:
        state, (right_edge, in_tape) = config

        tape = list(in_tape)

        cells = len(tape)

        pos = cells - 1 if right_edge else 0

        for _ in range(self.sim_lim):  # no-branch
            color, shift, next_state = \
                self.comp[state, scan := tape[pos]]

            if next_state != state:
                state = next_state

                tape[pos] = color

                if shift:
                    pos += 1
                    if cells <= pos:
                        break
                else:
                    if pos == 0:
                        break
                    pos -= 1

            else:
                if shift:
                    while tape[pos] == scan:  # pylint: disable = while-used
                        tape[pos] = color
                        pos += 1
                        if cells <= pos:
                            break
                    else:
                        continue

                else:
                    while tape[pos] == scan:  # pylint: disable = while-used
                        tape[pos] = color
                        if pos == 0:
                            break
                        pos -= 1
                    else:
                        continue

                break

        else:
            raise MacroInfLoop

        return state, (cells <= pos, tuple(tape))

########################################

class Logic(Protocol):
    cells: int
    base_states: int
    base_colors: int

    converter: TapeColorConverter

    @property
    @abstractmethod
    def name(self) -> str: ...

    @property
    @abstractmethod
    def macro_states(self) -> int: ...

    @property
    @abstractmethod
    def macro_colors(self) -> int: ...

    @property
    @abstractmethod
    def sim_lim(self) -> int: ...

    @abstractmethod
    def deconstruct_inputs(self, slot: Slot) -> Config: ...

    @abstractmethod
    def reconstruct_outputs(self, config: Config) -> Instr: ...


class BlockLogic:
    def __init__(self, cells: int, base_states: int, base_colors: int):
        self.cells = cells
        self.base_states = base_states
        self.base_colors = base_colors

        self.converter = make_converter(base_colors, cells)

    @property
    def name(self) -> str:
        return 'block'

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

    def deconstruct_inputs(self, slot: Slot) -> Config:
        macro_state, macro_color = slot

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


class BacksymbolLogic:
    backsymbols: int

    def __init__(self, cells: int, base_states: int, base_colors: int):
        self.cells = cells
        self.base_states = base_states
        self.base_colors = base_colors
        self.backsymbols = self.base_colors ** self.cells

        self.converter = make_converter(base_colors, cells)

    @property
    def name(self) -> str:
        return 'backsymbol'

    @property
    def macro_states(self) -> int:
        return 2 * self.base_states * self.backsymbols

    @property
    def macro_colors(self) -> int:
        return self.base_colors

    @property
    def sim_lim(self) -> int:
        return self.macro_states * self.macro_colors

    def deconstruct_inputs(self, slot: Slot) -> Config:
        macro_state, macro_color = slot

        st_co, at_right = divmod(macro_state, 2)

        state, backsymbol = divmod(st_co, self.backsymbols)

        backspan = self.converter.color_to_tape(backsymbol)

        return state, (
            (False, (macro_color,) + backspan)
            if at_right else
            ( True, backspan + (macro_color,))
        )

    def reconstruct_outputs(self, config: Config) -> Instr:
        state, (at_right, tape) = config

        out_color, backsymbol = (
            (tape[-1], tape[:-1])
            if (shift := not at_right) else
            (tape[0], tape[1:])
        )

        return (
            out_color,
            shift,
            int(shift)
            + (2
               * (self.converter.tape_to_color(backsymbol)
                  + (state
                     * self.backsymbols)))
            ,
        )
