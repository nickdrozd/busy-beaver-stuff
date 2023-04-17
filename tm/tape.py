from __future__ import annotations

import math
from typing import TYPE_CHECKING
from dataclasses import dataclass

from tm.rules import ApplyRule
from tm.rust_stuff import TagTape, EnumTape

if TYPE_CHECKING:
    from tm.instrs import Color, Shift
    from tm.rules import Count, Counts, Index

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
    count: Count

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

    def get_count(self, index: Index) -> Count:
        side, pos = index

        span = self.rspan if side else self.lspan

        return span[pos].count

    def set_count(self, index: Index, val: Count) -> None:
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

    def unroll(self) -> list[Color]:
        return [
            block.color
            for block in reversed(self.lspan)
            for _ in range(block.count)
        ] + [self.scan] + [
            block.color
            for block in self.rspan
            for _ in range(block.count)
        ]

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
