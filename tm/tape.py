from __future__ import annotations

from dataclasses import dataclass, field

from tm.instrs import Color, Shift

Block = list[int]
Span  = list[Block]

Signature = tuple[
    Color,
    tuple[Color | tuple[Color], ...],
    tuple[Color | tuple[Color], ...],
]

Counts = tuple[tuple[int, ...], tuple[int, ...]]

Rule = dict[tuple[int, int], int]

@dataclass
class BlockTape:
    lspan: Span
    scan: Color
    rspan: Span

    @property
    def counts(self) -> Counts:
        return (
            tuple(block[1] for block in self.lspan),
            tuple(block[1] for block in self.rspan),
        )

    @property
    def spans(self) -> tuple[Span, Span]:
        return self.lspan, self.rspan

    @staticmethod
    def calculate_diff(old: int, new: int) -> int | None:
        if old == new:
            return None

        return new - old

    def make_rule(self, new_counts: Counts) -> Rule:
        return {
            (s, i): diff
            for s, spans in enumerate(zip(self.counts, new_counts))
            for i, counts in enumerate(zip(*spans))
            if (diff := self.calculate_diff(*counts)) is not None
        }

    def apply_rule(self, rule: Rule) -> int | None:
        divs: list[int] = []

        for (s, i), diff in rule.items():
            if diff >= 0:
                continue

            if ((abs_diff := abs(diff))
                    >= (count := self.spans[s][i][1])):
                return None

            div, rem = divmod(count, abs_diff)
            divs.append(div if rem > 0 else div - 1)

        times: int = min(divs)

        for (s, i), diff in rule.items():
            self.spans[s][i][1] += diff * times

        return times


@dataclass
class Tape(BlockTape):
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

    def __hash__(self) -> int:
        return hash(str(self))

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
    def blank(self) -> bool:
        return self.scan == 0 and not self.lspan and not self.rspan

    @property
    def marks(self) -> int:
        return (
            (1 if self.scan != 0 else 0)
            + sum(q for (c, q) in self.lspan if c != 0)
            + sum(q for (c, q) in self.rspan if c != 0)
        )

    def at_edge(self, edge: Shift) -> bool:
        return (
            self.scan == 0
            and not (self.rspan if edge else self.lspan)
        )

    def step(self, shift: Shift, color: Color, skip: bool) -> int:
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

        next_scan: Color

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

            if  push or color != 0:
                push.append(push_block)

        self.scan = next_scan

        if shift:
            self.head += stepped
        else:
            self.head -= stepped

        return stepped


@dataclass
class TagTape(BlockTape):
    lspan: Span
    scan: Color
    rspan: Span

    scan_info: list[int] = field(
        default_factory = list)

    @property
    def signature(self) -> Signature:
        return (
            self.scan,
            tuple(c if q != 1 else (c,) for (c, q, *_) in self.lspan),
            tuple(c if q != 1 else (c,) for (c, q, *_) in self.rspan),
        )

    @property
    def marks(self) -> int:
        return (
            (1 if self.scan != 0 else 0)
            + sum(q for (c, q, *_) in self.lspan if c != 0)
            + sum(q for (c, q, *_) in self.rspan if c != 0)
        )

    def step(self, shift: Shift, color: Color, skip: bool) -> None:
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

        scan_info: list[int] = []

        next_scan: Color

        dec_pull: bool = False

        if not pull:
            next_scan = 0
        else:
            next_scan = (next_pull := pull[-1])[0]

            if next_pull[1] > 1:
                next_pull[1] -= 1
                dec_pull = True
            else:
                popped = pull.pop()

                if push_block is None:
                    push_block = popped
                    push_block[1] = 0

                if (extra := popped[2:]):
                    scan_info += extra
                    push_block = push_block[:2]

        if push and (top_block := push[-1])[0] == color:
            top_block[1] += stepped
            top_block += self.scan_info

            if push_block is not None and not top_block[2:]:
                top_block += push_block[2:]
        else:
            if push_block is None:
                push_block = [color, 1]

                if push and color != self.scan:
                    top_block = push[-1]
                    if len(top_block) > 3:
                        push_block.append(
                            top_block.pop())

                if dec_pull:
                    push_block.extend(
                        self.scan_info)

                    self.scan_info.clear()
                    assert not scan_info
            else:
                push_block[0] = color
                push_block[1] += 1

                if push and len(top_block := push[-1]) > 3:
                    push_block.append(
                        top_block.pop())

            if push or color != 0:
                push.append(push_block)

                if self.scan_info and not (top_block := push[-1])[2:]:
                    top_block.extend(
                        self.scan_info)

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
