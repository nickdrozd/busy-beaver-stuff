from enum import Enum

## prover ##############################

class PastConfigs:
    def __init__(self, state: State, cycle: int) -> None: ...

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

Params = tuple[State, Color]

CompProg = dict[Slot, Instr]

LetterState = str

def tcompile(program: str) -> CompProg: ...

def halt_slots(prog: str) -> list[Slot]: ...

def erase_slots(prog: str) -> list[Slot]: ...

def zero_reflexive_slots(prog: str) -> list[Slot]: ...

def show_state(state: State | None) -> LetterState: ...

def show_slot(slot: Slot) -> str: ...

def read_slot(slot: str) -> Slot: ...

def show_instr(instr: Instr | None) -> str: ...

def read_instr(instr: str) -> Instr | None: ...

def show_comp_py(
        comp: CompProg,
        params: Params | None = None,
) -> str: ...

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
    rulapp: int

    last_slot: Slot | None
    blanks: dict[State, int]

    def __init__(
            self,
            result: TermRes,
            steps: int,
            cycles: int,
            marks: int,
            rulapp: int,
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
    @property
    def cfglim(self) -> int | None: ...

def run_prover(prog: str, sim_lim: int = 0) -> MachineResult: ...

def run_quick_machine(prog: str, sim_lim: int = 0) -> MachineResult: ...

def quick_term_or_rec_py(prog: str, sim_lim: int) -> bool: ...

## rules ###############################

Plus = int

Mult = tuple[int, int]

Op = Plus | Mult

Index = tuple[int, int]

Rule = dict[Index, Op]

## blocks ##############################

def opt_block_py(prog: str, steps: int) -> int: ...

## reason ##############################

def cant_halt_py(prog: str) -> bool: ...
def cant_blank_py(prog: str) -> bool: ...
def cant_spin_out_py(prog: str) -> bool: ...

## tree ################################

def tree_progs(
        params: Params,
        halt: bool,
        sim_lim: int,
) -> list[str]: ...
