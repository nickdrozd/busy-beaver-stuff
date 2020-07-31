STATE_MAP = {
    'A': '1',
    'B': '2',
    'C': '3',
    'D': '4',
    'E': '5',
    'F': '6',
    'G': '7',
    'H': '0',
}


def parse(instructions):
    instructions = iter([
        instr[1:]
        for instr in
        instructions.split()
    ])

    return tuple(zip(instructions, instructions))


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
