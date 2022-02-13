from __future__ import annotations

from typing import List, Optional

class MacroTape:
    def __init__(self, lspan, scan, rspan):
        self.lspan = lspan
        self.scan = scan
        self.rspan = rspan

        self.head = 0
        self.init = sum(q for (_, q) in self.lspan)

    def copy(self) -> MacroTape:
        return MacroTape(
            self.lspan.copy(),
            self.scan,
            self.rspan.copy(),
        )

    @property
    def blank(self) -> bool:
        return (
            self.scan == 0
            and all(c == 0 for (c, _) in self.lspan)
            and all(c == 0 for (c, _) in self.rspan)
        )

    @property
    def marks(self) -> int:
        return (
            (1 if self.scan != 0 else 0)
            + sum(q for (c, q) in self.lspan if c != 0)
            + sum(q for (c, q) in self.rspan if c != 0)
        )

    @property
    def edge(self) -> Optional[int]:
        if self.scan != 0:
            return None

        if all(c == 0 for (c, _) in self.lspan):
            return 0

        if all(c == 0 for (c, _) in self.rspan):
            return 1

        return None

    def shift_head(self, shift, stepped) -> int:
        if shift:
            self.head += stepped
        else:
            if self.head + self.init == 0:
                self.init += stepped

            self.head -= stepped

        return stepped

    def step(self, shift: int, color: int) -> int:
        pull, push = (
            (self.rspan, self.lspan)
            if shift else
            (self.lspan, self.rspan)
        )

        try:
            next_color, next_count = push.pop()
        except IndexError:
            push.append([color, 1])
        else:
            if next_color == color:
                push.append((color, 1 + next_count))
            else:
                push.append((next_color, next_count))
                push.append((color, 1))

        try:
            next_color, next_count = pull.pop()
        except IndexError:
            next_color = 0
        else:
            if next_count > 1:
                pull.append((next_color, next_count - 1))

        self.scan = next_color

        return self.shift_head(shift, 1)

    def skip(self, shift: int, color: int) -> int:
        pull, push = (
            (self.rspan, self.lspan)
            if shift else
            (self.lspan, self.rspan)
        )

        try:
            block_color, block_count = pull[-1]
        except IndexError:
            return self.step(shift, color)

        if block_color != self.scan:
            return self.step(shift, color)

        pull.pop()

        try:
            next_color, next_count = pull.pop()
        except IndexError:
            next_color = 0
        else:
            if next_count > 1:
                pull.append((next_color, next_count - 1))

        self.scan = next_color

        stepped = 1 + block_count

        push.append([color, stepped])

        return self.shift_head(shift, stepped)

    def to_ptr(self) -> PtrTape:
        lspan, rspan = [], []

        for color, count in self.lspan:
            lspan += [color] * count

        for color, count in self.rspan:
            rspan += [color] * count

        return PtrTape(
            lspan + [self.scan] + list(reversed(rspan)),
            self.init,
            self.head,
        )


class PtrTape:
    def __init__(self, tape, init, head):
        self._list = tape
        self._init = init
        self.head = head

        self.lspan =               0 - self._init
        self.rspan = len(self._list) - self._init

    def __repr__(self) -> str:
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

    def __getitem__(self, tape_index) -> List[int]:
        if (stop := tape_index.stop) is None:
            right = None
        else:
            if (right := stop + self._init - len(self._list)) > 0:
                self._list.extend([0] * right)
                self.rspan += right

            right = stop + self._init

        if (start := tape_index.start) is None:
            left = None
        else:
            if (left := 0 - (start + self._init)) > 0:
                self._list = [0] * left + self._list
                self._init += left
                self.lspan -= left

            left = start + self._init

        return self._list[ left : right ]
