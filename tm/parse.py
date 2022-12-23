from tm.instrs import Instr, CompInstr, CompProg


def parse(program: str) -> tuple[tuple[Instr | None, ...], ...]:
    return tuple(
        tuple(
            (int(instr[0]), instr[1], instr[2])
            if '.' not in instr else None
            for instr in instrs.split(' ')
        )
        for instrs in program.strip().split('  ')
    )


def comp_instr(instr: Instr | None) -> CompInstr | None:
    return (
        instr[0],
        0 if instr[1] == 'L' else 1,
        str_st(instr[2]),
    ) if instr else None


def tcompile(program: str) -> CompProg:
    return {
        (state, color): comp_instr(instr)
        for state, instrs in enumerate(parse(program))
        for color, instr in enumerate(instrs)
    }


def st_str(state: int) -> str:
    return '_' if state == -1 else chr(state + 65)


def str_st(state: str) -> int:
    return -1 if state == '_' else ord(state) - 65
