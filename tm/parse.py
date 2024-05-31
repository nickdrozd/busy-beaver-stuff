# ruff: noqa: F401
# pylint: disable-next = unused-import, wrong-import-order
from tm.rust_stuff import (
    parse_to_vec as parse,
    tcompile,
    read_slot,
    init_prog,
)

Color = int
State = int
Shift = bool

Slot = tuple[State, Color]
Instr = tuple[Color, Shift, State]

CompProg = dict[Slot, Instr]

Params = tuple[int, int]
