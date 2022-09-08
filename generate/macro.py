# pylint: disable = too-few-public-methods

from itertools import product
from typing import Dict, Iterator, List, Optional, Tuple, Union

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

    def run_simulator(
            self,
            state: State,
            right_edge: bool,
            tape: Tape,
    ) -> Tuple[Tape, int, State]:
        cells = len(tape)

        pos = cells - 1 if right_edge else 0

        for _ in range((self.states * cells) * (self.colors ** cells)):
            assert (instr := self.comp[state][tape[pos]]) is not None

            color, shift, next_state = instr

            tape[pos] = color

            pos += 1 if shift else -1

            if (state := next_state) == -1:
                return tape, pos, -1

            if 0 <= pos < cells:
                continue

            return tape, pos, state

        return tape, pos, -1

########################################

class BlockMacro(MacroRunner):
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
            instr = self.calculate_instr(
                state,
                self.color_to_tape(color),
            )

            self.instrs[(state, color)] = instr

            return instr

    def calculate_instr(self, st_sh: State, in_tape: Tape) -> Instr:
        in_state, right_edge = divmod(st_sh, 2)

        tape, pos, state = self.run_simulator(
            in_state,
            right_edge == 1,
            in_tape,
        )

        return (
            self.tape_to_color(tape),
            int(0 <= pos),
            (
                (2 * state) + int(pos < 0)
                if state != -1 else
                -1
            ),
        )

    def tape_to_color(self, tape: Tape) -> Color:
        color = int(
            ''.join(map(str, tape)),
            self.colors)

        self.tape_colors[color] = tuple(tape)

        return color

    def color_to_tape(self, color: Color) -> Tape:
        if color == 0:
            return [0] * self.cells

        return list(self.tape_colors[color])

    @property
    def all_tapes(self) -> Iterator[Tape]:
        return map(
            list,
            product(
                range(self.colors),
                repeat = self.cells))

    @property
    def fully_specified(self) -> CompProg:
        return tuple(
            tuple(
                self.calculate_instr(st_sh, tape)
                for tape in self.all_tapes
            )
            for st_sh in range(2 * self.states)
        )

    @property
    def dump_program(self) -> Program:
        return Program(
            dcompile(
                self.fully_specified)
        ).normalize()
