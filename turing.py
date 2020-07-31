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
