def parse(program_string):
    instructions = iter(program_string.split())

    return tuple(
        zip(instructions, instructions, instructions, instructions)
        if '3' in program_string else
        zip(instructions, instructions, instructions)
        if '2' in program_string else
        zip(instructions, instructions)
    )


def tcompile(parsed):
    return tuple(
        tuple(
            (
                int(action[0]),
                0 if action[1] == 'L' else 1,
                ord(action[2]) - 65,
            )
            if '.' not in action else None
            for action in instr
        )
        for instr in parsed
    )
