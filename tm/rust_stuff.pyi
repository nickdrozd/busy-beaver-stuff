from tm.instrs import Color, State, Instr, Prog

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

## prover ##############################

class PastConfig:
    cycles: list[int]

    def next_deltas(self, cycle: int) -> tuple[int, int] | None: ...

## pasre ###############################

LetterState = str

def parse(program: str) -> list[list[Instr | None]]: ...

def tcompile(program: str) -> Prog: ...

def st_str(state: State | None) -> LetterState: ...

def str_st(state: LetterState) -> State: ...

def dcomp_instr(instr: Instr | None) -> str: ...

## rules ###############################

Plus = int

Mult = tuple[int, int]

Op = Plus | Mult

Index = tuple[int, int]

Rule = dict[Index, Op]

Counts = tuple[tuple[int, ...], tuple[int, ...]]

def calculate_diff(cnt1: int, cnt2: int, cnt3: int) -> Op | None: ...

def make_rule(cnts1: Counts, cnts2: Counts, cnts3: Counts) -> Rule: ...

class InfiniteRule(Exception): pass
class RuleLimit(Exception): pass
class UnknownRule(Exception): pass
