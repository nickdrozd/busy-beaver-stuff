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

CompThin = dict[Slot, Instr]
CompThic = dict[Slot, Instr | None]

LetterState = str

def parse_to_vec(program: str) -> list[list[Instr | None]]: ...

def comp_thin(program: str) -> CompThin: ...

def comp_thic(program: str) -> CompThic: ...

def halt_slots(prog: str) -> list[Slot]: ...

def erase_slots(prog: str) -> list[Slot]: ...

def zero_reflexive_slots(prog: str) -> list[Slot]: ...

def show_state(state: State | None) -> LetterState: ...

def show_slot(slot: Slot) -> str: ...

def read_slot(slot: str) -> Slot: ...

def show_instr(instr: Instr | None) -> str: ...

def init_prog(states: int, colors: int) -> str: ...

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

def quick_term_or_rec(prog: str, sim_lim: int) -> bool: ...

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

def cant_halt(prog: str) -> bool: ...
def cant_blank(prog: str) -> bool: ...
def cant_spin_out(prog: str) -> bool: ...

## tree ################################

class TreeSkip(Exception): ...

def run_for_undefined(prog: str, sim_lim: int) -> Slot | None: ...
