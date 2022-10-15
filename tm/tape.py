from __future__ import annotations

from dataclasses import dataclass
from typing import List, Optional

Color = int

@dataclass
class BlockTape:
    lspan: List[List[int]]
    scan: Color
    rspan: List[List[int]]
    head: int = 0

    def __repr__(self) -> str:
        return ' '.join([
            f'{color}^{count}'
            for color, count in self.lspan
        ] + [
            f'[{self.scan}]'
        ] + [
            f'{color}^{count}'
            for color, count in reversed(self.rspan)
        ])

    @property
    def init(self) -> int:
        return sum(q for (_, q) in self.lspan) - self.head

    def copy(self) -> BlockTape:
        return BlockTape(
            [[color, count] for color, count in self.lspan],
            self.scan,
            [[color, count] for color, count in self.rspan],
            head = self.head,
        )

    def to_ptr(self) -> PtrTape:
        lspan, rspan = [], []

        for color, count in self.lspan:
            lspan += [color] * count

        for color, count in self.rspan:
            rspan += [color] * count

        return PtrTape(
            self.init,
            lspan + [self.scan] + list(reversed(rspan)),
        )

    @property
    def signature(self) -> str:
        l_sig = '|'.join([str(c) for c, _ in self.lspan])
        r_sig = '|'.join(reversed([str(c) for c, _ in self.rspan]))

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

    def step(self, shift: int, color: int) -> int:
        pull, push = (
            (self.rspan, self.lspan)
            if shift else
            (self.lspan, self.rspan)
        )

        if not pull:
            next_color = 0
        else:
            next_color, next_count = pull[-1]

            if next_count > 1:
                pull[-1][1] -= 1
            else:
                pull.pop()

        self.scan = next_color

        if push and (block := push[-1])[0] == color:
            block[1] += 1
        else:
            push.append([color, 1])

        if shift:
            self.head += 1
        else:
            self.head -= 1

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

        if push and (block := push[-1])[0] == color:
            block[1] += stepped
        else:
            push.append([color, stepped])

        if shift:
            self.head += stepped
        else:
            self.head -= stepped

        return stepped


@dataclass
class PtrTape:
    init: int
    tape: List[Color]

    @property
    def r_end(self) -> int:
        return len(self.tape) - self.init

    @property
    def l_end(self) -> int:
        return 0 - self.init

    def __getitem__(self, tape_index: slice) -> List[int]:
        if (stop := tape_index.stop) is None:
            stop = self.r_end + 1
        else:
            self.extend_to_bound_right(stop)

        if (start := tape_index.start) is None:
            start = self.l_end
        else:
            self.extend_to_bound_left(start)

        return self.tape[ start + self.init : stop + self.init ]

    def extend_to_bound_right(self, stop: int) -> None:
        if (rdiff := stop + self.init - self.r_end) > 0:
            self.tape.extend([0] * rdiff)

    def extend_to_bound_left(self, start: int) -> None:
        if (ldiff := 0 - (start + self.init)) > 0:
            self.tape = [0] * ldiff + self.tape
            self.init += ldiff
