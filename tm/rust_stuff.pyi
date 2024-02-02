## prover ##############################

class PastConfigs:
    def __init__(self) -> None: ...

    def next_deltas(
            self,
            state: State,
            cycle: int,
    ) -> tuple[int, int, int] | None: ...

    def delete_configs(self, state: State) -> None: ...

## parse ###############################

Color = int
State = int
Shift = bool

Slot = tuple[State, Color]
Instr = tuple[Color, Shift, State]
Prog = dict[Slot, Instr]

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

# ruff: noqa: E701
class InfiniteRule(Exception): pass
class RuleLimit(Exception): pass
class UnknownRule(Exception): pass

## blocks ##############################

def measure_blocks(prog: str, steps: int) -> int | None: ...

def unroll_tape(prog: str, steps: int) -> list[Color]: ...
