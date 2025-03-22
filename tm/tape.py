# ruff: noqa: FBT001
from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

from tm.num import show_number

if TYPE_CHECKING:
    from tm.num import Count
    from tm.parse import Color, Shift

    Signature = tuple[
        Color,
        tuple[Color | tuple[Color], ...],
        tuple[Color | tuple[Color], ...],
    ]

    Index = tuple[int, int]

    Counts = tuple[
        list[Count],
        list[Count],
    ]

########################################

@dataclass(slots = True)
class Block:
    color: Color
    count: Count

    def __str__(self) -> str:
        color, count = self.color, self.count

        return (
            f"{color}"
            if count == 1 else
            f"{color}^{show_number(count)}"
        )

    def clone(self) -> Block:
        return Block(self.color, self.count)

@dataclass(slots = True)
class Tape:
    lspan: list[Block]
    scan: Color
    rspan: list[Block]

    def __init__(
            self,
            lspan: list[Block] | None = None,
            scan: Color = 0,
            rspan: list[Block] | None = None,
    ):
        self.lspan = lspan or []
        self.scan = scan
        self.rspan = rspan or []

    def clone(self) -> Tape:
        return Tape(
            lspan = [block.clone() for block in self.lspan],
            scan = self.scan,
            rspan = [block.clone() for block in self.rspan],
        )

    def to_enum(self) -> EnumTape:
        return EnumTape(self.clone())

    def __str__(self) -> str:
        return ' '.join(
            [
                *list(map(str, reversed(self.lspan))),
                f"[{self.scan}]",
                *list(map(str, self.rspan)),
            ],
        )

    @property
    def blank(self) -> bool:
        return self.scan == 0 and not self.lspan and not self.rspan

    @property
    def marks(self) -> Count:
        scan: int = 1 if self.scan != 0 else 0

        lspan: Count = sum(
            blk.count for blk in self.lspan if blk.color != 0)
        rspan: Count = sum(
            blk.count for blk in self.rspan if blk.color != 0)

        return scan + lspan + rspan

    def at_edge(self, edge: Shift) -> bool:
        return (
            self.scan == 0
            and not (self.rspan if edge else self.lspan)
        )

    @property
    def span_lens(self) -> tuple[int, int]:
        return len(self.lspan), len(self.rspan)

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

    def sig_compatible(self, sig: Signature) -> bool:
        scan, lspan, rspan = sig

        return (
            self.scan == scan
            and len(self.lspan) == len(lspan)
            and len(self.rspan) == len(rspan)
            and all(
                block.color == (
                    color[0]
                    if isinstance(color, tuple) else
                    color
                ) for block, color in zip(
                    self.lspan[:len(lspan)], lspan, strict = True))
            and all(
                block.color == (
                    color[0]
                    if isinstance(color, tuple) else
                    color
                ) for block, color in zip(
                    self.rspan[:len(rspan)], rspan, strict = True))
        )

    def step(self, shift: Shift, color: Color, skip: bool) -> Count:
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
            next_scan = (next_pull := pull[0]).color

            if next_pull.count != 1:
                next_pull.count -= 1
            else:
                popped = pull.pop(0)

                if push_block is None:
                    push_block = popped
                    push_block.count = 0

        if push and (top_block := push[0]).color == color:
            top_block.count += stepped
        elif push or color != 0:
            if push_block is None:
                push_block = Block(color, 1)
            else:
                push_block.color = color
                push_block.count += 1

            push.insert(0, push_block)

        self.scan = next_scan

        return stepped

########################################

BlockId = int
Enums = tuple[int, int]

class EnumTape:
    tape: Tape

    l_offset: int
    r_offset: int

    l_edge: bool
    r_edge: bool

    enums: dict[BlockId, Enums]

    def __init__(self, tape: Tape):
        self.tape = tape

        self.l_offset = 0
        self.r_offset = 0

        self.l_edge = False
        self.r_edge = False

        self.enums = {
            id(block): (side, num)
            for side, span in enumerate((tape.lspan, tape.rspan))
            for num, block in enumerate(span, start = 1)
        }

    @property
    def offsets(self) -> tuple[int, int]:
        return self.l_offset, self.r_offset

    @property
    def edges(self) -> tuple[bool, bool]:
        return self.l_edge, self.r_edge

    def touch_edge(self, shift: Shift) -> None:
        if shift:
            self.r_edge = True
        else:
            self.l_edge = True

    def check_offsets(self, block: Block) -> None:
        if (enums := self.enums.get(id(block))) is None:
            return

        side, offset = enums

        s_offset = ('r' if side else 'l') + '_offset'

        # pylint: disable = bad-builtin
        if offset > getattr(self, s_offset):
            setattr(self, s_offset, offset)

    def get_count(self, index: Index) -> Count:
        side, pos = index

        self.check_offsets(
            block := (
                self.tape.rspan
                if side else
                self.tape.lspan
            )[pos]
        )

        return block.count

    def set_count(self, index: Index, val: Count) -> None:
        self.tape.set_count(index, val)

    @property
    def scan(self) -> Color:
        return self.tape.scan

    @property
    def signature(self) -> Signature:
        return self.tape.signature

    def step(self, shift: Shift, color: Color, skip: bool) -> None:
        pull, push = (
            (self.tape.rspan, self.tape.lspan)
            if shift else
            (self.tape.lspan, self.tape.rspan)
        )

        if not pull:
            self.touch_edge(shift)
        else:
            self.check_offsets(near_block := pull[0])

            if skip and near_block.color == self.tape.scan:
                if not pull[1:]:
                    self.touch_edge(shift)
                else:
                    self.check_offsets(pull[1])

        if push and color == (opp := push[0]).color:
            self.check_offsets(opp)

        _ = self.tape.step(shift, color, skip)
