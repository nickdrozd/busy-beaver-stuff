from typing import Optional, Tuple

def parse(program: str) -> Tuple[Tuple[str, ...], ...]:
    return tuple(
        tuple(state.split(' '))
        for state in program.split('  ')
    )

CompInstr = Optional[Tuple[int, int, int]]

def tcompile(program: str) -> Tuple[Tuple[CompInstr, ...], ...]:
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
