from typing import Protocol
from abc import abstractmethod

Color = int
Shift = str
State = str

Instr = tuple[Color, Shift, State]

CompSlot = tuple[int, int]
CompInstr = tuple[int, int, int]
CompProg = dict[CompSlot, CompInstr | None]


class GetCompInstr(Protocol):
    @abstractmethod
    def __getitem__(self, slot: CompSlot) -> CompInstr | None: ...
