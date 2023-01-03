from typing import Protocol
from abc import abstractmethod

Color = int
Shift = str
State = str

Slot = tuple[State, Color]
Instr = tuple[Color, Shift, State]

CompState = int
CompShift = int

CompSlot = tuple[CompState, Color]
CompInstr = tuple[Color, CompShift, CompState]
CompProg = dict[CompSlot, CompInstr | None]


INIT: State = "A"
HALT: State = "_"
UNDF: State = "."
LEFT: Shift = "L"
RIGHT: Shift = "R"
BLANK: Color = 0


class GetCompInstr(Protocol):
    @abstractmethod
    def __getitem__(self, slot: CompSlot) -> CompInstr | None: ...
