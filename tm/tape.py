from __future__ import annotations

from dataclasses import dataclass

Color = int

Block = list[int]
Span  = list[Block]

Signature = tuple[tuple[int, ...], Color, tuple[int, ...]]

@dataclass
class BlockTape:
    lspan: Span
    scan: Color
    rspan: Span
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
    def signature(self) -> Signature:
        return (
            tuple(c for c, _ in self.lspan),
            self.scan,
            tuple(c for c, _ in self.rspan),
        )

    @property
    def spans(self) -> tuple[Span, Span]:
        return self.lspan, self.rspan

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
    def edge(self) -> int | None:
        if self.scan != 0:
            return None

        if len(self.lspan) <= 1:
            if not self.lspan or self.lspan[0][0] == 0:
                return 0

        if len(self.rspan) <= 1:
            if not self.rspan or self.rspan[0][0] == 0:
                return 1

        return None

    def step(self, shift: int, color: int, skip: bool) -> int:
        pull, push = (
            (self.rspan, self.lspan)
            if shift else
            (self.lspan, self.rspan)
        )

        push_block = (
            pull.pop()
            if skip and pull and pull[-1][0] == self.scan else
            None
        )

        if not pull:
            self.scan = 0
        else:
            self.scan = (next_pull := pull[-1])[0]

            if next_pull[1] > 1:
                next_pull[1] -= 1
            else:
                pull.pop()

        stepped = 1 if push_block is None else 1 + push_block[1]

        if push and (block := push[-1])[0] == color:
            block[1] += stepped

            if push_block is not None:
                block += push_block[2:]
        else:
            if push_block is None:
                push_block = [color, 1]
            else:
                push_block[0] = color
                push_block[1] += 1
            push.append(push_block)

        if shift:
            self.head += stepped
        else:
            self.head -= stepped

        return stepped


@dataclass
class PtrTape:
    init: int
    tape: list[Color]

    @property
    def r_end(self) -> int:
        return len(self.tape) - self.init

    @property
    def l_end(self) -> int:
        return 0 - self.init

    def __getitem__(self, tape_index: slice) -> list[int]:
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
