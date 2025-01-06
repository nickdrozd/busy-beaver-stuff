# ruff: noqa: F401
from tm.rust_stuff import (
    read_slot,
    tcompile,
)

Color = int
State = int
Shift = bool

Slot = tuple[State, Color]
Instr = tuple[Color, Shift, State]

CompProg = dict[Slot, Instr]

Params = tuple[int, int]
