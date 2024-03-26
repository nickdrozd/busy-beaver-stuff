# ruff: noqa: F401
# pylint: disable-next = unused-import, wrong-import-order
from tm.rust_stuff import (
    parse_to_vec as parse,
    comp_thin,
    read_slot,
    init_prog,
)

from typing import Protocol
from collections.abc import Sized

Color = int
State = int
Shift = bool

Slot = tuple[State, Color]
Instr = tuple[Color, Shift, State]

CompThin = dict[Slot, Instr]

Switch = dict[Color, Instr | None]

class GetInstr(Protocol, Sized):
    def __getitem__(self, slot: Slot) -> Instr: ...
