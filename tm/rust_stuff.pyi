from tm.instrs import Color, Shift, State, Instr, Prog
from tm.rules import Counts

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

def calculate_diff(cnt1: int, cnt2: int, cnt3: int) -> Op | None: ...

def make_rule(cnts1: Counts, cnts2: Counts, cnts3: Counts) -> Rule: ...

class InfiniteRule(Exception): pass
class RuleLimit(Exception): pass
class UnknownRule(Exception): pass

## tape ################################

Signature = tuple[
    Color,
    tuple[Color | tuple[Color], ...],
    tuple[Color | tuple[Color], ...],
]

class TagTape:
    scan: int
    scan_info: list[int]
    def __init__(
            self,
            lspan: list[tuple[int, int, list[int]]],
            scan: Color,
            rspan: list[tuple[int, int, list[int]]],
    ): ...
    @property
    def lspan(self) -> list[tuple[Color, int, list[int]]]: ...
    @property
    def rspan(self) -> list[tuple[Color, int, list[int]]]: ...
    @property
    def blank(self) -> bool: ...
    def at_edge(self, edge: Shift) -> bool: ...
    @property
    def counts(self) -> Counts: ...
    @property
    def signature(self) -> Signature: ...
    @property
    def missing_tags(self) -> bool: ...
    def step(self, shift: Shift, color: Color, skip: bool) -> None: ...
    def apply_rule(self, rule: Rule) -> int | None: ...

class EnumTape:
    scan: int
    def __init__(
            self,
            lspan: list[tuple[int, int]],
            scan: Color,
            rspan: list[tuple[int, int]],
    ): ...
    @property
    def lspan(self) -> list[tuple[Color, int]]: ...
    @property
    def rspan(self) -> list[tuple[Color, int]]: ...
    @property
    def offsets(self) -> tuple[int, int]: ...
    @property
    def edges(self) -> tuple[bool, bool]: ...
    @property
    def signature(self) -> Signature: ...
    def step(self, shift: Shift, color: Color, skip: bool) -> None: ...
    def apply_rule(self, rule: Rule) -> int | None: ...
