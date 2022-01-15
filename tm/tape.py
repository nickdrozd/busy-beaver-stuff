class MicroTape:
    def __init__(self, lspan, scan, rspan):
        self.lspan = lspan
        self.scan = scan
        self.rspan = rspan

        self.head = 0
        self.init = len(lspan)

    def right(self, color):
        self.lspan.append(color)

        try:
            self.scan = self.rspan.pop()
        except IndexError:
            self.scan = 0

        self.head += 1

    def left(self, color):
        self.rspan.append(color)

        try:
            self.scan = self.lspan.pop()
        except IndexError:
            self.scan = 0

        if self.head + self.init == 0:
            self.init += 1

        self.head -= 1


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
