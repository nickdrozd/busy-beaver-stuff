class MacroTape:
    def __init__(self, lspan, scan, rspan):
        self.lspan = lspan
        self.scan = scan
        self.rspan = rspan

        self.head = 0
        self.init = sum(q for (_, q) in self.lspan)

    def copy(self):
        return MacroTape(
            self.lspan.copy(),
            self.scan,
            self.rspan.copy(),
        )

    def blank(self) -> bool:
        return (
            self.scan == 0
            and all(c == 0 for (c, _) in self.lspan)
            and all(c == 0 for (c, _) in self.rspan)
        )

    def marks(self) -> int:
        return (
            (1 if self.scan != 0 else 0)
            + sum(q for (c, q) in self.lspan if c != 0)
            + sum(q for (c, q) in self.rspan if c != 0)
        )

    def step(self, shift: int, color: int) -> int:
        pull, push = (
            (self.rspan, self.lspan)
            if shift else
            (self.lspan, self.rspan)
        )

        try:
            block = push[-1]
        except IndexError:
            push.append([color, 1])
        else:
            if block[0] == color:
                block[1] += 1
            else:
                push.append([color, 1])

        try:
            block = pull[-1]
        except IndexError:
            self.scan = 0
        else:
            self.scan = block[0]

            if block[1] < 2:
                pull.pop()
            else:
                block[1] -= 1

        if shift:
            self.head += 1
        else:
            if self.head + self.init == 0:
                self.init += 1

            self.head -= 1

        return 1

    def skip(self, shift: int, color: int) -> int:
        stepped = 0

        init_scan = self.scan

        side = self.rspan if shift else self.lspan

        while self.scan == init_scan:  # pylint: disable=while-used
            stepped += self.step(shift, color)

            try:
                next_square, _ = side[-1]
            except IndexError:
                break

            if next_square != init_scan:
                break

        return stepped

    def to_ptr(self):
        lspan, rspan = [], []

        for color, count in self.lspan:
            lspan += [color] * count

        for color, count in self.rspan:
            rspan += [color] * count

        return PtrTape(
            lspan,
            self.scan,
            rspan,
            self.init,
            self.head,
        )


class PtrTape:
    def __init__(self, lspan, scan, rspan, init, head = 0):
        self._list = lspan + [scan] + list(reversed(rspan))
        self._init = init
        self.head = head
        self._pos  = self.head + self._init

        self.lspan =               0 - self._init
        self.rspan = len(self._list) - self._init

    def __repr__(self):
        squares = [
            '_' if square == 0 else str(square)
            for square in self._list
        ]

        return ''.join([
            (f'[{square}]' if i != self._init else f'[<{square}>]')
            if i == self.head + self._init else
            (square if i != self._init else f'<{square}>')
            for i, square in enumerate(squares)
        ])

    def __len__(self):
        return len(self._list)

    @property
    def span(self):
        return self.lspan, self.rspan

    def extend_to(self, span):
        # pylint: disable = pointless-statement
        self[ span[0] : span[1] ]

    def __getitem__(self, tape_index):
        if tape_index.stop is None:
            right = None
        else:
            right = tape_index.stop + self._init - len(self._list)

            if right > 0:
                self._list = self._list + [0] * right
                self.rspan += right

            right = tape_index.stop + self._init

        if tape_index.start is None:
            left = None
        else:
            if (left := 0 - (tape_index.start + self._init)) > 0:
                self._list = [0] * left + self._list
                self._init += left
                self._pos  += left
                self.lspan -= left

            left = tape_index.start + self._init

        return self._list[ left : right ]

    def __setitem__(self, tape_index, value):
        self._list[tape_index + self._init] = value

    def read(self):
        return self._list[self._pos]

    def print(self, color):
        self._list[self._pos] = color

    def right(self):
        self.head += 1
        self._pos  += 1

        try:
            self._list[self._pos]
        except IndexError:
            self._list.append(0)
            self.rspan += 1

    def left(self):
        if self.head + self._init == 0:
            self._list.insert(0, 0)
            self._init += 1
            self._pos  += 1
            self.lspan -= 1

        self.head -= 1
        self._pos  -= 1
