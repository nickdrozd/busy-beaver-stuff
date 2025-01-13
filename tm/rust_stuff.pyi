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

type Color = int
type State = int
type Shift = bool

type Slot = tuple[State, Color]
type Instr = tuple[Color, Shift, State]

type Params = tuple[State, Color]

type CompProg = dict[Slot, Instr]

def tcompile(program: str) -> CompProg: ...

def halt_slots(prog: str) -> list[Slot]: ...

def erase_slots(prog: str) -> list[Slot]: ...

def zero_reflexive_slots(prog: str) -> list[Slot]: ...

def show_state(state: State | None) -> str: ...

def show_slot(slot: Slot) -> str: ...

def read_slot(slot: str) -> Slot: ...

def show_instr(instr: Instr | None) -> str: ...

def read_instr(instr: str) -> Instr | None: ...

def py_show_comp(
        comp: CompProg,
        params: Params | None = None,
) -> str: ...

## machine #############################

class TermRes(Enum):  # type: ignore[misc]
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

def py_quick_term_or_rec(prog: str, sim_lim: int) -> bool: ...

## rules ###############################

Plus = int

type Mult = tuple[int, int]

type Op = Plus | Mult

type Index = tuple[int, int]

type Rule = dict[Index, Op]

## blocks ##############################

def py_opt_block(prog: str, steps: int) -> int: ...

## reason ##############################

def py_is_connected(prog: str, states: int) -> bool: ...

class BackwardResult:
    @property
    def step(self) -> int: ...
    def is_refuted(self) -> bool: ...
    def is_settled(self) -> bool: ...
    class refuted:
        ...
    class init:
        ...
    class linrec:
        ...
    class spinout:
        ...
    class step_limit:
        ...
    class depth_limit:
        ...

def py_cant_halt(prog: str, depth: int) -> BackwardResult: ...
def py_cant_blank(prog: str, depth: int) -> BackwardResult: ...
def py_cant_spin_out(prog: str, depth: int) -> BackwardResult: ...

class SegmentResult:
    @property
    def step(self) -> int: ...
    def is_refuted(self) -> bool: ...
    def is_settled(self) -> bool: ...
    class refuted:
        ...
    class halt:
        ...
    class blank:
        ...
    class repeat:
        ...
    class spinout:
        ...
    class depth_limit:
        ...
    class segment_limit:
        ...

def py_segment_cant_halt(prog: str, segs: int) -> SegmentResult: ...
def py_segment_cant_blank(prog: str, segs: int) -> SegmentResult: ...
def py_segment_cant_spin_out(prog: str, segs: int) -> SegmentResult: ...

## tree ################################

def tree_progs(
        params: Params,
        halt: bool,
        sim_lim: int,
) -> list[str]: ...
