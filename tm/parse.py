from tm.instrs import (
    RIGHT, HALT, UNDF,
    State, LetterState, Instr, LetterInstr, Prog
)


def parse(program: str) -> tuple[tuple[Instr | None, ...], ...]:
    return tuple(
        tuple(
            (
                int(instr[0]),
                instr[1] == RIGHT,
                str_st(instr[2]),
            )
            if UNDF not in instr else None
            for instr in instrs.split(' ')
        )
        for instrs in program.strip().split('  ')
    )


def comp_instr(instr: LetterInstr | None) -> Instr | None:
    return (
        instr[0],
        instr[1] == RIGHT,
        str_st(instr[2]),
    ) if instr else None


def tcompile(program: str) -> Prog:
    return {
        (state, color): instr
        for state, instrs in enumerate(parse(program))
        for color, instr in enumerate(instrs)
    }


def st_str(state: State | None) -> LetterState:
    return (
        HALT if state == -1 else
        UNDF if state is None else
        chr(state + 65)
    )


def str_st(state: LetterState) -> State:
    return -1 if state == HALT else ord(state) - 65
