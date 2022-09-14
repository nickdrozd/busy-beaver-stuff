from typing import Optional, Tuple


Instr = Tuple[int, int, int]
CompProg = Tuple[Tuple[Optional[Instr], ...], ...]


def parse(program: str) -> Tuple[Tuple[str, ...], ...]:
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


def convert_instr(instr: Optional[Instr]) -> str:
    if instr is None:
        return '...'

    pr, sh, tr = instr

    return (
        str(pr)
        + ('R' if sh else 'L')
        + (chr(65 + tr) if tr != -1 else '_')
    )
