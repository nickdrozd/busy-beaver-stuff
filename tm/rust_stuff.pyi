## prover ##############################

class PastConfig:
    cycles: list[int]

    def next_deltas(self, cycle: int) -> tuple[int, int, int] | None: ...

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

## graph ###############################

ConGraph = dict[State, set[State]]

class Graph:
    arrows: dict[State, list[State | None]]
    def __init__(self, program: str): ...
    @property
    def states(self) -> tuple[State, ...]: ...
    @property
    def colors(self) -> tuple[Color, ...]: ...
    @property
    def exit_points(self) -> ConGraph: ...
    @property
    def entry_points(self) -> ConGraph: ...
    @property
    def is_normal(self) -> bool: ...
    @property
    def is_strongly_connected(self) -> bool: ...
    @property
    def reflexive_states(self) -> set[State]: ...
    @property
    def zero_reflexive_states(self) -> set[State]: ...
    @property
    def is_irreflexive(self) -> bool: ...
    @property
    def is_zero_reflexive(self) -> bool: ...
    @property
    def entries_dispersed(self) -> bool: ...
    @property
    def exits_dispersed(self) -> bool: ...
    @property
    def is_dispersed(self) -> bool: ...
    @property
    def is_simple(self) -> bool: ...
    @property
    def reduced(self) -> ConGraph: ...
