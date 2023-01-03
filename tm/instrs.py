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


class GetCompInstr(Protocol):
    @abstractmethod
    def __getitem__(self, slot: CompSlot) -> CompInstr | None: ...
