from __future__ import annotations

from typing import List, Optional, Tuple

class MacroTape:
    def __init__(
            self,
            lspan: List[Tuple[int, int]],
            scan: int,
            rspan: List[Tuple[int, int]],
            head: Optional[int] = None,
            init: Optional[int] = None,
    ):
        self.lspan = lspan
        self.scan = scan
        self.rspan = rspan

        self.head: int = 0 if head is None else head
        self.init: int = (
            sum(q for (_, q) in self.lspan)
            if init is None else
            init
        )

    def listify(self) -> List[int]:
        lspan, rspan = [], []

        for color, count in self.lspan:
            lspan += [color] * count

        for color, count in self.rspan:
            rspan += [color] * count

        return lspan + [self.scan] + list(reversed(rspan))

    def copy(self) -> MacroTape:
        return MacroTape(
            self.lspan.copy(),
            self.scan,
            self.rspan.copy(),
            head = self.head,
            init = self.init,
        )

    def to_ptr(self) -> PtrTape:
        return PtrTape(
            self.listify(),
            self.init,
            self.head,
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

    def shift_head(self, shift: int, stepped: int):
        if shift:
            self.head += stepped
        else:
            if self.head + self.init == 0:
                self.init += stepped

            self.head -= stepped

    def step(self, shift: int, color: int) -> int:
        pull, push = (
            (self.rspan, self.lspan)
            if shift else
            (self.lspan, self.rspan)
        )

        try:
            next_color, next_count = push.pop()
        except IndexError:
            push.append((color, 1))
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

        self.shift_head(shift, 1)

        return 1

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

        push.append((color, stepped))

        self.shift_head(shift, stepped)

        return stepped


class PtrTape:
    def __init__(self, tape, init, head):
        self.tape = tape
        self.init = init
        self.head = head

        self.lspan: int =              0 - self.init
        self.rspan: int = len(self.tape) - self.init

    def __repr__(self) -> str:
        squares = [
            '_' if square == 0 else str(square)
            for square in self.tape
        ]

        return ''.join([
            (f'[{square}]' if i != self.init else f'[<{square}>]')
            if i == self.head + self.init else
            (square if i != self.init else f'<{square}>')
            for i, square in enumerate(squares)
        ])

    def __getitem__(self, tape_index: slice) -> List[int]:
        if (stop := tape_index.stop) is None:
            right = None
        else:
            if (right := stop + self.init - len(self.tape)) > 0:
                self.tape.extend([0] * right)
                self.rspan += right

            right = stop + self.init

        if (start := tape_index.start) is None:
            left = None
        else:
            if (left := 0 - (start + self.init)) > 0:
                self.tape = [0] * left + self.tape
                self.init += left
                self.lspan -= left

            left = start + self.init

        return self.tape[ left : right ]
