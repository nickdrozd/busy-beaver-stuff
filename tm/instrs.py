from typing import Protocol
from abc import abstractmethod

Color = int
LetterShift = str
LetterState = str

LetterSlot = tuple[LetterState, Color]
LetterInstr = tuple[Color, LetterShift, LetterState]

State = int
Shift = int

Slot = tuple[State, Color]
Instr = tuple[Color, Shift, State]
Prog = dict[Slot, Instr | None]


INIT: LetterState = "A"
HALT: LetterState = "_"
UNDF: LetterState = "."
LEFT: LetterShift = "L"
RIGHT: LetterShift = "R"
BLANK: Color = 0


class GetInstr(Protocol):
    @abstractmethod
    def __getitem__(self, slot: Slot) -> Instr | None: ...
