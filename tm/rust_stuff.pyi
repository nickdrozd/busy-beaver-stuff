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
