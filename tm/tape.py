from __future__ import annotations

from dataclasses import dataclass

Color = int

Block = list[int]
Span  = list[Block]

Signature = tuple[
    Color,
    tuple[Color | tuple[Color], ...],
    tuple[Color | tuple[Color], ...],
]

@dataclass
class Tape:
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

    def copy(self) -> Tape:
        return Tape(
            [[color, count] for color, count in self.lspan],
            self.scan,
            [[color, count] for color, count in self.rspan],
            head = self.head,
        )

    def to_tag(self) -> TagTape:
        return TagTape(
            [[color, count] for color, count in self.lspan],
            self.scan,
            [[color, count] for color, count in self.rspan],
        )

    @property
    def signature(self) -> Signature:
        return (
            self.scan,
            tuple(c if q != 1 else (c,) for (c, q) in self.lspan),
            tuple(c if q != 1 else (c,) for (c, q) in self.rspan),
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

        stepped = 1 if push_block is None else 1 + push_block[1]

        next_scan: int

        if not pull:
            next_scan = 0
        else:
            next_scan = (next_pull := pull[-1])[0]

            if next_pull[1] > 1:
                next_pull[1] -= 1
            else:
                popped = pull.pop()

                if push_block is None:
                    push_block = popped
                    push_block[1] = 0

        if push and (top_block := push[-1])[0] == color:
            top_block[1] += stepped
        else:
            if push_block is None:
                push_block = [color, 1]
            else:
                push_block[0] = color
                push_block[1] += 1

            push.append(push_block)

        self.scan = next_scan

        if shift:
            self.head += stepped
        else:
            self.head -= stepped

        return stepped


@dataclass
class TagTape:
    lspan: Span
    scan: Color
    rspan: Span

    scan_info: int | None = None

    @property
    def signature(self) -> Signature:
        return (
            self.scan,
            tuple(c if q != 1 else (c,) for (c, q, *_) in self.lspan),
            tuple(c if q != 1 else (c,) for (c, q, *_) in self.rspan),
        )

    @property
    def spans(self) -> tuple[Span, Span]:
        return self.lspan, self.rspan

    def step(self, shift: int, color: int, skip: bool) -> None:
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

        stepped = 1 if push_block is None else 1 + push_block[1]

        scan_info: int | None = None

        next_scan: int

        if not pull:
            next_scan = 0
        else:
            next_scan = (next_pull := pull[-1])[0]

            if next_pull[1] > 1:
                next_pull[1] -= 1
            else:
                popped = pull.pop()

                if push_block is None:
                    push_block = popped
                    push_block[1] = 0

                if popped[2:]:
                    scan_info = popped[2]
                    push_block = push_block[:2]

        if push and (top_block := push[-1])[0] == color:
            top_block[1] += stepped

            if push_block is not None and not top_block[2:]:
                top_block += push_block[2:]
        else:
            if push_block is None:
                push_block = [color, 1]
                scan_info = None
                if color != self.scan:
                    self.scan_info = None
            else:
                push_block[0] = color
                push_block[1] += 1

                if self.scan_info is not None and not push_block[2:]:
                    push_block.append(self.scan_info)

            push.append(push_block)

        if self.scan_info is not None:
            if not push[-1][2:]:
                push[-1].append(self.scan_info)

        self.scan_info = scan_info

        self.scan = next_scan


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
