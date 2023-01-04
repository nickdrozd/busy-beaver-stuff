from abc import abstractmethod
from typing import Final, Protocol

Color = int
LetterShift = str
LetterState = str

LetterSlot = tuple[LetterState, Color]
LetterInstr = tuple[Color, LetterShift, LetterState]

State = int
Shift = bool

Slot = tuple[State, Color]
Instr = tuple[Color, Shift, State]
Prog = dict[Slot, Instr | None]


BLANK: Final[Color] = 0

INIT: Final[LetterState] = "A"
HALT: Final[LetterState] = "_"
UNDF: Final[LetterState] = "."

LEFT: Final[LetterShift] = "L"
RIGHT: Final[LetterShift] = "R"


class GetInstr(Protocol):
    @abstractmethod
    def __getitem__(self, slot: Slot) -> Instr | None: ...
