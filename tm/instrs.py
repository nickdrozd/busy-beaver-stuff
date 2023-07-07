from abc import abstractmethod
from typing import Protocol

Color = int
State = int
Shift = bool

Slot = tuple[State, Color]
Instr = tuple[Color, Shift, State]
Prog = dict[Slot, Instr | None]

class GetInstr(Protocol):
    @abstractmethod
    def __getitem__(self, slot: Slot) -> Instr | None: ...
