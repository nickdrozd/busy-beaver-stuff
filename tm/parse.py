from typing import TYPE_CHECKING

# pylint: disable-next = unused-import
from tm.rust_stuff import (
    parse,
    tcompile,
)

if TYPE_CHECKING:
    from typing import Protocol
    from collections.abc import Sized

    Color = int
    State = int
    Shift = bool

    Slot = tuple[State, Color]
    Instr = tuple[Color, Shift, State]
    Prog = dict[Slot, Instr | None]

    class GetInstr(Protocol, Sized):
        def __getitem__(self, slot: Slot) -> Instr | None: ...
