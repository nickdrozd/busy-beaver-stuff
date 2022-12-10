Instr = tuple[int, str, str]
CompInstr = tuple[int, int, int]
CompProg = tuple[tuple[CompInstr | None, ...], ...]


def parse(program: str) -> tuple[tuple[Instr | None, ...], ...]:
    return tuple(
        tuple(
            (int(instr[0]), instr[1], instr[2])
            if '.' not in instr else None
            for instr in instrs.split(' ')
        )
        for instrs in program.strip().split('  ')
    )


def comp_instr(instr: Instr) -> CompInstr:
    return (
        instr[0],
        0 if instr[1] == 'L' else 1,
        str_st(instr[2]),
    )


def tcompile(program: str) -> CompProg:
    return tuple(
        tuple(
            comp_instr(instr)
            if instr is not None else None
            for instr in instrs
        )
        for instrs in parse(program)
    )


def dcompile(comp: CompProg) -> str:
    return '  '.join(
        ' '.join(map(convert_comp_instr, instrs))
        for instrs in comp
    )


def convert_comp_instr(instr: CompInstr | None) -> str:
    if instr is None:
        return '...'

    pr, sh, tr = instr

    return (
        str(pr)
        + ('R' if sh else 'L')
        + st_str(tr)
    )


def st_str(state: int) -> str:
    return '_' if state == -1 else chr(state + 65)


def str_st(state: str) -> int:
    return -1 if state == '_' else ord(state) - 65
