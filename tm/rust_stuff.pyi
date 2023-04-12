from tm.instrs import State, Instr, Prog

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
