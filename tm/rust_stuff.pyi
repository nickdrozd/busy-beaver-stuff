## prover ##############################

class PastConfig:
    cycles: list[int]

    def next_deltas(self, cycle: int) -> tuple[int, int] | None: ...

## parse ###############################

Color = int
State = int
Shift = bool

Slot = tuple[State, Color]
Instr = tuple[Color, Shift, State]
Prog = dict[Slot, Instr | None]

LetterState = str

def parse(program: str) -> list[list[Instr | None]]: ...

def tcompile(program: str) -> Prog: ...

def show_state(state: State | None) -> LetterState: ...

def read_state(state: LetterState) -> State: ...

def show_slot(slot: Slot) -> str: ...

def read_slot(slot: str) -> Slot: ...

def show_instr(instr: Instr | None) -> str: ...

## rules ###############################

Plus = int

Mult = tuple[int, int]

Op = Plus | Mult

Index = tuple[int, int]

Rule = dict[Index, Op]

class InfiniteRule(Exception): pass
class RuleLimit(Exception): pass
class UnknownRule(Exception): pass
