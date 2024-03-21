from enum import Enum

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
CompProg = dict[Slot, Instr]

LetterState = str

def parse(program: str) -> list[list[Instr | None]]: ...

def tcompile(program: str) -> CompProg: ...

def halt_slots(prog: str) -> list[Slot]: ...

def erase_slots(prog: str) -> list[Slot]: ...

def zero_reflexive_slots(prog: str) -> list[Slot]: ...

def show_state(state: State | None) -> LetterState: ...

def show_slot(slot: Slot) -> str: ...

def read_slot(slot: str) -> Slot: ...

def show_instr(instr: Instr | None) -> str: ...

## machine #############################

class TermRes(Enum):
    undfnd: int
    spnout: int
    halted: int
    infrul: int
    xlimit: int

class MachineResult:
    result: TermRes

    steps: int
    cycles: int
    marks: int

    last_slot: Slot | None
    blanks: dict[State, int]

    def __init__(
            self,
            result: TermRes,
            steps: int,
            cycles: int,
            marks: int,
            last_slot: Slot | None,
            blanks: dict[State, int],
    ): ...

    @property
    def simple_termination(self) -> int | None: ...
    @property
    def undfnd(self) -> tuple[int, Slot] | None: ...
    @property
    def halted(self) -> int | None: ...
    @property
    def infrul(self) -> int | None: ...
    @property
    def spnout(self) -> int | None: ...
    @property
    def xlimit(self) -> int | None: ...

def run_machine(prog: str, sim_lim: int = 0) -> MachineResult: ...

## rules ###############################

Plus = int

Mult = tuple[int, int]

Op = Plus | Mult

Index = tuple[int, int]

Rule = dict[Index, Op]

class InfiniteRule(Exception): ...
class RuleLimit(Exception): ...
class UnknownRule(Exception): ...

## blocks ##############################

def measure_blocks(prog: str, steps: int) -> int | None: ...

def unroll_tape(prog: str, steps: int) -> list[Color]: ...

## reason ##############################

TupleBlock = tuple[int, int]

TupleTape = tuple[
    tuple[TupleBlock, ...],
    Color,
    tuple[TupleBlock, ...],
]

def reason_parse(prog: str) -> tuple[
        int,
        dict[State, list[State]],
        dict[State, list[Instr]],
]: ...

class BackstepMachineHalt:
    def __init__(self, prog: str): ...

    def backstep_run(
            self,
            sim_lim: int,
            init_tape: TupleTape,
            state: State,
            shift: Shift,
            color: Color,
    ) -> int | None: ...

class BackstepMachineBlank:
    def __init__(self, prog: str): ...

    def backstep_run(
            self,
            sim_lim: int,
            init_tape: TupleTape,
            state: State,
            shift: Shift,
            color: Color,
    ) -> int | None: ...

class BackstepMachineSpinout:
    def __init__(self, prog: str): ...

    def backstep_run(
            self,
            sim_lim: int,
            init_tape: TupleTape,
            state: State,
            shift: Shift,
            color: Color,
    ) -> int | None: ...

## tree ################################

class TreeSkip(Exception): ...

def run_for_undefined(prog: str, sim_lim: int) -> Slot | None: ...
