from __future__ import annotations

from typing import List, Optional, Tuple

class BlockTape:
    def __init__(
            self,
            lspan: List[List[int]],
            scan: int,
            rspan: List[List[int]],
            head: Optional[int] = None,
            init: Optional[int] = None,
            extend_to: Optional[int] = None,
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

        self.extend_to = extend_to

    def __repr__(self) -> str:
        diff, listified = self.listify(self.extend_to)

        init = self.init + diff

        squares = [
            '_' if square == 0 else str(square)
            for square in listified
        ]

        return ''.join([
            (f'[{square}]' if i != init else f'[<{square}>]')
            if i == self.head + init else
            (square if i != init else f'<{square}>')
            for i, square in enumerate(squares)
        ])

    def listify(self, ext: Optional[int]) -> Tuple[int, List[int]]:
        lspan, rspan = [], []

        for color, count in self.lspan:
            lspan += [color] * count

        for color, count in self.rspan:
            rspan += [color] * count

        if ext is None:
            ldiff = 0
        else:
            ldiff = (ext // 2) - len(lspan) + self.head
            lspan = [0] * ldiff + lspan

            rdiff = (ext // 2) - len(rspan) - self.head
            rspan = [0] * rdiff + rspan

        return ldiff, lspan + [self.scan] + list(reversed(rspan))

    def copy(self) -> BlockTape:
        return BlockTape(
            self.lspan.copy(),
            self.scan,
            self.rspan.copy(),
            head = self.head,
            init = self.init,
        )

    def to_ptr(self) -> PtrTape:
        ldiff, listified = self.listify(None)

        return PtrTape(
            listified,
            self.init + ldiff,
            self.head,
        )

    @property
    def signature(self) -> str:
        l_sig = ''.join([str(c) for c, _ in self.lspan])
        r_sig = ''.join(reversed([str(c) for c, _ in self.rspan]))

        return f'{l_sig}[{self.scan}]{r_sig}'

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

            if self.head + self.init < 0:
                self.init -= (self.head + self.init)

    def step(self, shift: int, color: int) -> int:
        pull, push = (
            (self.rspan, self.lspan)
            if shift else
            (self.lspan, self.rspan)
        )

        try:
            next_color, next_count = push[-1]
        except IndexError:
            push.append([color, 1])
        else:
            if next_color == color:
                push[-1][1] += 1
            else:
                push.append([color, 1])

        try:
            next_color, next_count = pull[-1]
        except IndexError:
            next_color = 0
        else:
            if next_count > 1:
                pull[-1][1] -= 1
            else:
                pull.pop()

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
            next_color, next_count = pull[-1]
        except IndexError:
            next_color = 0
        else:
            if next_count > 1:
                pull[-1][1] -= 1
            else:
                pull.pop()

        self.scan = next_color

        stepped = 1 + block_count

        try:
            next_push_color, _ = push[-1]
        except IndexError:
            push.append([color, stepped])
        else:
            if next_push_color != color:
                push.append([color, stepped])
            else:
                push[-1][1] += stepped

        self.shift_head(shift, stepped)

        return stepped


class PtrTape:
    # pylint: disable = too-few-public-methods
    def __init__(self, tape, init, head):
        self.tape = tape
        self.init = init
        self.head = head

        self.lspan: int =              0 - self.init
        self.rspan: int = len(self.tape) - self.init

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
