from __future__ import annotations

import math
from typing import TYPE_CHECKING
from dataclasses import dataclass, field

from tm.rules import ApplyRule

if TYPE_CHECKING:
    from tm.instrs import Color, Shift
    from tm.rules import Counts, Index

    Signature = tuple[
        Color,
        tuple[Color | tuple[Color], ...],
        tuple[Color | tuple[Color], ...],
    ]


TRUNCATE_COUNT = 10 ** 12

def show_number(num: int) -> str:
    return (
        str(num)
        if num < TRUNCATE_COUNT else
        f"(~10^{math.log10(num):.0f})"
    )


@dataclass
class BasicBlock:
    color: Color
    count: int

    def __str__(self) -> str:
        return f"{self.color}^{show_number(self.count)}"


class BlockTape(ApplyRule):
    lspan: list[BasicBlock]
    scan: Color
    rspan: list[BasicBlock]

    def __str__(self) -> str:
        return ' '.join(
            list(map(str, reversed(self.lspan)))
            + [f'[{self.scan}]']
            + list(map(str, self.rspan)))

    @property
    def blank(self) -> bool:
        return self.scan == 0 and not self.lspan and not self.rspan

    @property
    def marks(self) -> int:
        return (
            (1 if self.scan != 0 else 0)
            + sum(blk.count for blk in self.lspan if blk.color != 0)
            + sum(blk.count for blk in self.rspan if blk.color != 0)
        )

    @property
    def blocks(self) -> int:
        return len(self.lspan) + len(self.rspan)

    def at_edge(self, edge: Shift) -> bool:
        return (
            self.scan == 0
            and not (self.rspan if edge else self.lspan)
        )

    @property
    def counts(self) -> Counts:
        return (
            [block.count for block in self.lspan],
            [block.count for block in self.rspan],
        )

    @property
    def signature(self) -> Signature:
        return (
            self.scan,
            tuple(
                block.color if block.count != 1 else (block.color,)
                for block in self.lspan),
            tuple(
                block.color if block.count != 1 else (block.color,)
                for block in self.rspan),
        )

    def get_count(self, index: Index) -> int:
        side, pos = index

        span = self.rspan if side else self.lspan

        return span[pos].count

    def set_count(self, index: Index, val: int) -> None:
        side, pos = index

        span = self.rspan if side else self.lspan

        span[pos].count = val


@dataclass
class Block(BasicBlock):
    pass


@dataclass
class Tape(BlockTape):
    lspan: list[Block]  # type: ignore[assignment]
    scan: Color
    rspan: list[Block]  # type: ignore[assignment]

    head: int = 0

    def __init__(
            self,
            lspan: list[tuple[int, int]],
            scan: Color,
            rspan: list[tuple[int, int]],
            head: int = 0,
    ):
        self.lspan = [Block(color, count) for color, count in lspan]
        self.scan = scan
        self.rspan = [Block(color, count) for color, count in rspan]

        self.head = head

    def __hash__(self) -> int:
        return hash((
            self.scan,
            tuple((block.color, block.count) for block in self.lspan),
            tuple((block.color, block.count) for block in self.rspan),
        ))

    @staticmethod
    def init(scan: Color = 0) -> Tape:
        return Tape([], scan, [])

    def copy(self) -> Tape:
        return Tape(
            [(block.color, block.count) for block in self.lspan],
            self.scan,
            [(block.color, block.count) for block in self.rspan],
            head = self.head
        )

    def to_tag(self) -> TagTape:
        return TagTape(
            [
                (
                    block.color,
                    block.count,
                    [2 * i] if block.count > 1 else [],
                )
                for i, block in enumerate(self.lspan)
            ],
            self.scan,
            [
                (
                    block.color,
                    block.count,
                    [2 * i + 1] if block.count > 1 else [],
                )
                for i, block in enumerate(self.rspan)
            ],
        )

    def to_enum(self) -> EnumTape:
        return EnumTape(
            [(block.color, block.count) for block in self.lspan],
            self.scan,
            [(block.color, block.count) for block in self.rspan],
        )

    def to_ptr(self) -> PtrTape:
        return PtrTape(
            sum(q.count for q in self.lspan) - self.head,
            [
                block.color
                for block in reversed(self.lspan)
                for _ in range(block.count)
            ] + [self.scan] + [
                block.color
                for block in self.rspan
                for _ in range(block.count)
            ]
        )

    def step(self, shift: Shift, color: Color, skip: bool) -> int:
        pull, push = (
            (self.rspan, self.lspan)
            if shift else
            (self.lspan, self.rspan)
        )

        push_block = (
            pull.pop(0)
            if skip and pull and pull[0].color == self.scan else
            None
        )

        stepped = 1 if push_block is None else 1 + push_block.count

        next_scan: Color

        if not pull:
            next_scan = 0
        else:
            next_pull = pull[0]

            if next_pull.count > 1:
                next_pull.count -= 1
            else:
                popped = pull.pop(0)

                if push_block is None:
                    push_block = popped
                    push_block.count = 0

            next_scan = next_pull.color

        if push and (top_block := push[0]).color == color:
            top_block.count += stepped
        else:
            if push_block is None:
                push_block = Block(color, 1)
            else:
                push_block.color = color
                push_block.count += 1

            if push or color != 0:
                push.insert(0, push_block)

        self.scan = next_scan

        if shift:
            self.head += stepped
        else:
            self.head -= stepped

        return stepped


@dataclass
class TagBlock(BasicBlock):
    tags: list[int] = field(
        default_factory = list)


class TagTape(BlockTape):
    lspan: list[TagBlock]  # type: ignore[assignment]
    scan: Color
    rspan: list[TagBlock]  # type: ignore[assignment]

    scan_info: list[int]

    def __init__(
            self,
            lspan: list[tuple[int, int, list[int]]],
            scan: Color,
            rspan: list[tuple[int, int, list[int]]],
    ):
        self.lspan = [
            TagBlock(color, count, tags)
            for color, count, tags in lspan
        ]

        self.scan = scan

        self.rspan = [
            TagBlock(color, count, tags)
            for color, count, tags in rspan
        ]

        self.scan_info = []

    @property
    def missing_tags(self) -> bool:
        return any(
            block.count > 1 and len(block.tags) != 1
            for span in (self.lspan, self.rspan)
            for block in span)

    def step(self, shift: Shift, color: Color, skip: bool) -> None:
        pull, push = (
            (self.rspan, self.lspan)
            if shift else
            (self.lspan, self.rspan)
        )

        push_block = (
            pull.pop(0)
            if (skip := (skip and
                         bool(pull) and
                         pull[0].color == self.scan)) else
            None
        )

        stepped = 1 if push_block is None else 1 + push_block.count

        scan_info: list[int] = []

        next_scan: Color

        dec_pull: bool = False
        inc_push: bool = False

        if not pull:
            next_scan = 0
        else:
            next_scan = (next_pull := pull[0]).color

            if next_pull.count > 1:
                next_pull.count -= 1
                dec_pull = True
            else:
                popped = pull.pop(0)

                if push_block is None:
                    push_block = popped
                    push_block.count = 0

                if (extra := popped.tags):
                    scan_info += extra
                    push_block.tags = []

        if push and (top_block := push[0]).color == color:
            inc_push = True
            top_block.count += stepped
            top_block.tags += self.scan_info

            if push_block is not None:
                top_block.tags += push_block.tags
        else:
            if push_block is None:
                push_block = TagBlock(color, 1)

                if push and color != self.scan:
                    if len(tags := push[0].tags) > 1:
                        push_block.tags.append(tags.pop())

                if dec_pull:
                    push_block.tags.extend(self.scan_info)

                    self.scan_info.clear()
                    assert not scan_info
            else:
                push_block.color = color
                push_block.count += 1

                if push and len(tags := push[0].tags) > 1:
                    push_block.tags.append(tags.pop())

                if self.scan_info:
                    push_block.tags.extend(self.scan_info)

            if push or color != 0 or push_block.tags or skip:
                if color == 0 and not push:
                    push_block.count = 1

                push.insert(0, push_block)

                if self.scan_info and not (top_block := push[0]).tags:
                    top_block.tags.extend(self.scan_info)

        if inc_push and not (top_block := push[0]).tags:
            top_block.tags.extend(scan_info)
        else:
            self.scan_info = scan_info

        self.scan = next_scan


@dataclass
class EnumBlock(BasicBlock):
    enums: tuple[int, int] | None = None


class EnumTape(BlockTape):
    lspan: list[EnumBlock]  # type: ignore[assignment]
    scan: Color
    rspan: list[EnumBlock]  # type: ignore[assignment]

    offsets: list[int]

    _edges: list[bool]

    def __init__(
            self,
            lspan: list[tuple[int, int]],
            scan: Color,
            rspan: list[tuple[int, int]],
    ):
        self.lspan = [EnumBlock(color, count) for color, count in lspan]
        self.scan = scan
        self.rspan = [EnumBlock(color, count) for color, count in rspan]

        self.offsets = [0, 0]

        self._edges = [False, False]

        for s, span in enumerate(self.spans):
            for i, block in enumerate(span, start=1):
                block.enums = (s, i)

    @property
    def spans(self) -> tuple[list[EnumBlock], list[EnumBlock]]:
        return self.lspan, self.rspan

    @property
    def edges(self) -> tuple[bool, bool]:
        return self._edges[0], self._edges[1]

    def step(self, shift: Shift, color: Color, skip: bool) -> int:
        pull, push = (
            (self.rspan, self.lspan)
            if shift else
            (self.lspan, self.rspan)
        )

        if not pull:
            self._edges[bool(shift)] = True
        else:
            if enums := (near_block := pull[0]).enums:
                ind, offset = enums

                if offset > self.offsets[ind]:
                    self.offsets[ind] = offset

            if skip and near_block.color == self.scan:
                if not pull[1:]:
                    self._edges[bool(shift)] = True
                elif (next_block := pull[1].enums):
                    ind, offset = next_block

                    if offset > self.offsets[ind]:
                        self.offsets[ind] = offset

        if (push
                and (enums := (opp := push[0]).enums)
                and color == opp.color):
            ind, offset = enums

            if offset > self.offsets[ind]:
                self.offsets[ind] = offset

        push_block = (
            pull.pop(0)
            if skip and pull and pull[0].color == self.scan else
            None
        )

        stepped = 1 if push_block is None else 1 + push_block.count

        next_scan: Color

        if not pull:
            next_scan = 0
        else:
            next_scan = (next_pull := pull[0]).color

            if next_pull.count > 1:
                next_pull.count -= 1
            else:
                popped = pull.pop(0)

                if push_block is None:
                    push_block = popped
                    push_block.count = 0

        if push and (top_block := push[0]).color == color:
            top_block.count += stepped
        else:
            if push_block is None:
                push_block = EnumBlock(color, 1)
            else:
                push_block.color = color
                push_block.count += 1

            if push or color != 0:
                push.insert(0, push_block)

        self.scan = next_scan

        return stepped

##################################################

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

    def get(
            self,
            start: int | None = None,
            stop: int | None = None,
    ) -> list[Color]:
        if stop is None:
            stop = self.r_end + 1
        else:
            self.extend_to_bound_right(stop)

        if start is None:
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
