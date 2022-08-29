from typing import Optional, Tuple

def parse(program: str) -> Tuple[Tuple[str, ...], ...]:
    return tuple(
        tuple(state.split(' '))
        for state in program.strip().split('  ')
    )

Instr = Tuple[int, int, int]

def tcompile(program: str) -> Tuple[Tuple[Optional[Instr], ...], ...]:
    return tuple(
        tuple(
            (
                int(action[0]),
                0 if action[1] == 'L' else 1,
                ord(action[2]) - 65,
            )
            if '.' not in action and '-' not in action
            else None
            for action in instr
        )
        for instr in parse(program)
    )

def dcompile(comp: Tuple[Tuple[Instr, ...], ...]) -> str:
    def convert_instr(instr: Instr) -> str:
        # pylint: disable = invalid-name
        pr, sh, tr = instr

        return str(pr) + ('R' if sh else 'L') + chr(65 + tr)

    return '  '.join(
        ' '.join(map(convert_instr, instrs))
        for instrs in comp
    )
