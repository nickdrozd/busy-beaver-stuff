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

## tape ################################

TupleBlock = tuple[int, int]

TupleTape = tuple[
    tuple[TupleBlock, ...],
    Color,
    tuple[TupleBlock, ...],
]

class BackstepMachine:
    blanks: dict[State, int]

    halted: int | None
    spnout: int | None
    undfnd: int | None

    def __init__(self, prog: str): ...

    def get_halt(self) -> int | None: ...
    def get_spinout(self) -> int | None: ...
    def get_min_blank(self) -> int | None: ...

    def backstep_run(
            self,
            sim_lim: int,
            init_tape: TupleTape,
            state: State,
            shift: Shift,
            color: Color,
    ) -> None: ...
