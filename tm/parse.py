Instr = tuple[int, int, int]

CompProg = tuple[tuple[Instr | None, ...], ...]

def parse(program: str) -> tuple[tuple[str, ...], ...]:
    return tuple(
        tuple(state.split(' '))
        for state in program.strip().split('  ')
    )


def tcompile(program: str) -> CompProg:
    return tuple(
        tuple(
            (
                int(action[0]),
                0 if action[1] == 'L' else 1,
                ord(state) - 65
                    if (state := action[2]) != '_'
                    else -1,
            )
            if '.' not in action and '-' not in action
            else None
            for action in instr
        )
        for instr in parse(program)
    )


def dcompile(comp: CompProg) -> str:
    return '  '.join(
        ' '.join(map(convert_instr, instrs))
        for instrs in comp
    )


def convert_instr(instr: Instr | None) -> str:
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
