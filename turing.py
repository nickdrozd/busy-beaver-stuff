STATE_MAP = {
    'A': '0',
    'B': '1',
    'C': '2',
    'D': '3',
    'E': '4',
    'F': '5',
    'G': '6',
    'H': '7',
}

HALT = 7

SHIFT_MAP = {
    'L': 0,
    'R': 1,
}


def parse(program_string):
    instructions = iter([
        instr[1:].replace(
            instr[2],
            STATE_MAP[instr[2]])
        for instr in
        program_string.split()
    ])

    return tuple(
        tuple(
            (int(SHIFT_MAP[action[0]]), int(action[1]))
            for action in instr)
        for instr in
        zip(instructions, instructions)
    )


BB3_STRING = "1RB   1RH   1LB   0RC   1LC   1LA"

print(parse(BB3_STRING))


class Machine:
    def __init__(self, tape):
        assert len(tape)
        self._tape = tape
        self._pos = 0
        self._state = 1

    def move_left(self):
        if self._pos == 0:
            self._tape.insert(0, 0)
        else:
            self._pos -= 1

    def move_right(self):
        if self._pos == len(self._tape) - 1:
            self._tape.append(0)

        self._pos += 1

    def write(self, color):
        self._tape[self._pos] = color

    def exec(self):
        pass

    def run_to_halt(self):
