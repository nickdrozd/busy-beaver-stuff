from __future__ import annotations

from typing import List, Optional, Tuple, Union

class BlockTape:
    def __init__(
            self,
            lspan: List[List[int]],
            scan: Union[int, str],
            rspan: List[List[int]],
            head: Optional[int] = None,
            init: Optional[int] = None,
            extend_to: Optional[int] = None,
    ):
        self.lspan = lspan
        self.scan = int(scan)
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
            [[color, count] for color, count in self.lspan],
            self.scan,
            [[color, count] for color, count in self.rspan],
            head = self.head,
            init = self.init,
            extend_to = self.extend_to,
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

        if len(self.lspan) <= 1:
            if not self.lspan or self.lspan[0][0] == 0:
                return 0

        if len(self.rspan) <= 1:
            if not self.rspan or self.rspan[0][0] == 0:
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
                self.init += 1

    def step(self, shift: int, color: int) -> int:
        pull, push = (
            (self.rspan, self.lspan)
            if shift else
            (self.lspan, self.rspan)
        )

        if not push:
            push.append([color, 1])
        else:
            next_color, next_count = push[-1]

            if next_color == color:
                push[-1][1] += 1
            else:
                push.append([color, 1])

        if not pull:
            next_color = 0
        else:
            next_color, next_count = pull[-1]

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

        if not pull:
            return self.step(shift, color)

        block_color, block_count = pull[-1]

        if block_color != self.scan:
            return self.step(shift, color)

        pull.pop()

        if not pull:
            next_color = 0
        else:
            next_color, next_count = pull[-1]

            if next_count > 1:
                pull[-1][1] -= 1
            else:
                pull.pop()

        self.scan = next_color

        stepped = 1 + block_count

        if not push:
            push.append([color, stepped])
        else:
            next_push_color, _ = push[-1]

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
