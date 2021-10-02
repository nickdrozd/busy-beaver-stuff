def parse(program_string):
    return tuple(
        tuple(state.split(' '))
        for state in program_string.split('  ')
    )


def tcompile(program_string):
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
        for instr in parse(program_string)
    )
