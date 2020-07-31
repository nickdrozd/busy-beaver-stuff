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
        instr.replace(
            instr[2],
            STATE_MAP[instr[2]])
        for instr in
        program_string.split()
    ])

    return tuple(
        tuple(
            (int(action[0]), int(SHIFT_MAP[action[1]]), int(action[2]))
            for action in instr)
        for instr in
        zip(instructions, instructions)
    )


BB3 = "1RB   1RH   1LB   0RC   1LC   1LA"


class Machine:
    def __init__(self, tape, prog):
        self._prog = parse(prog)
        assert len(tape) > 0
        self._tape = tape
        self._pos = 0
        self._state = 0
        self._exec_count = 0

    def move_left(self):
        if self._pos == 0:
            self._tape.insert(0, 0)
        else:
            self._pos -= 1

    def move_right(self):
        if self._pos == len(self._tape) - 1:
            self._tape.append(0)

        self._pos += 1

    def move(self, shift):
        if shift == 0:
            self.move_left()
        else:
            self.move_right()

    def write(self, color):
        self._tape[self._pos] = color

    def get_instruction(self):
        return self._prog[self._state]

    def get_color(self):
        return self._tape[self._pos]

    def exec(self):
        instr = self.get_instruction()
        curr_color = self.get_color()

        (color, shift, state) = instr[curr_color]

        self.write(color)
        self.move(shift)
        self._state = state

        self._exec_count += 1

    def run_to_halt(self):
        while self._state != HALT:
            self.exec()

    def print_results(self):
        squares = [
            '_' if square == 0 else '#'
            for square in self._tape
        ]

        with_pos = ''.join([
            f'[{square}]' if i == self._pos else square
            for i, square in enumerate(squares)
        ])

        print(
            '\n** {} ** {} ** {}'.format(
                self._exec_count,
                sum(self._tape),
                with_pos))


if __name__ == '__main__':
    bb3 = Machine([0], BB3)
    bb3.run_to_halt()
    bb3.print_results()

    bb4 = Machine([0], '1RB   1LB   1LA   0LC   1RH   1LD   1RD   0RA')
    bb4.run_to_halt()
    bb4.print_results()
