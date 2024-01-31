# ruff: noqa: F401
# pylint: disable-next = unused-import, wrong-import-order
from tm.rust_stuff import (
    parse,
    tcompile,
    read_slot,
)

from typing import Protocol
from collections.abc import Sized

Color = int
State = int
Shift = bool

Slot = tuple[State, Color]
Instr = tuple[Color, Shift, State]
Prog = dict[Slot, Instr]

class GetInstr(Protocol, Sized):
    def __getitem__(self, slot: Slot) -> Instr: ...
