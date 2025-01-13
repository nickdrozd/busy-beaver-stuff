from typing import TYPE_CHECKING

# ruff: noqa: F401
from tm.rust_stuff import (
    read_slot,
    tcompile,
)

if TYPE_CHECKING:
    type Color = int
    type State = int
    type Shift = bool

    type Slot = tuple[State, Color]
    type Instr = tuple[Color, Shift, State]

    type CompProg = dict[Slot, Instr]

    type Params = tuple[int, int]
