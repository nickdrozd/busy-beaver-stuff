from __future__ import annotations

from typing import TYPE_CHECKING
from dataclasses import dataclass, field

from tm.num import show_number
from tm.rules import ApplyRule

if TYPE_CHECKING:
    from tm.parse import Color, Shift
    from tm.rules import Count, Counts, Index, Rule

    Signature = tuple[
        Color,
        tuple[Color | tuple[Color], ...],
        tuple[Color | tuple[Color], ...],
    ]


@dataclass(slots = True)
class Block:
    color: Color
    count: Count

    def __str__(self) -> str:
        return f"{self.color}^{show_number(self.count)}"

    def clone(self) -> Block:
        return Block(self.color, self.count)


class BlockTape(ApplyRule):
    lspan: list[Block]
    scan: Color
    rspan: list[Block]

    def __str__(self) -> str:
        return ' '.join(
            list(map(str, reversed(self.lspan)))
            + [f'[{self.scan}]']
            + list(map(str, self.rspan)))

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

########################################

class Tape(BlockTape):
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

    def to_tag(self) -> TagTape:
        return TagTape(self.lspan, self.scan, self.rspan)

    def clone(self) -> Tape:
        return Tape(
            lspan = [block.clone() for block in self.lspan],
            scan = self.scan,
            rspan = [block.clone() for block in self.rspan],
        )

    def to_enum(self) -> EnumTape:
        return EnumTape(self.clone())

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

@dataclass(slots = True)
class TagBlock(Block):
    tags: list[int] = field(
        default_factory = list)


class TagTape(BlockTape):
    lspan: list[TagBlock]  # type: ignore[assignment]
    scan: Color
    rspan: list[TagBlock]  # type: ignore[assignment]

    scan_info: list[int]

    def __init__(
            self,
            lspan: list[Block],
            scan: Color,
            rspan: list[Block],
    ):
        self.lspan = [
            TagBlock(
                block.color,
                block.count,
                [2 * i] if block.count != 1 else [])
            for i, block in enumerate(lspan)
        ]

        self.scan = scan

        self.rspan = [
            TagBlock(
                block.color,
                block.count,
                [2 * i + 1] if block.count != 1 else [])
            for i, block in enumerate(rspan)
        ]

        self.scan_info = []

    @property
    def missing_tags(self) -> bool:
        return any(
            block.count != 1 and len(block.tags) != 1
            for span in (self.lspan, self.rspan)
            for block in span)

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
                ) for block, color in zip(self.lspan, lspan))
            and all(
                block.color == (
                    color[0]
                    if isinstance(color, tuple) else
                    color
                ) for block, color in zip(self.rspan, rspan))
        )

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

            if next_pull.count != 1:
                next_pull.count -= 1
                dec_pull = True
            else:
                popped = pull.pop(0)

                if push_block is None:
                    push_block = popped
                    push_block.count = 0

                if (extra := popped.tags):
                    if push_block.tags:
                        scan_info += extra

                        if push_block.tags == extra:
                            push_block.tags = []

                    else:
                        target = (
                            push_block.tags
                            if push_block.count > popped.count else
                            scan_info
                        )

                        target += extra

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

                if (len(push) == 1
                        and (top_block := push[0]).color == 0
                        and top_block.tags
                        and not push_block.tags):
                    push_block.tags = top_block.tags
                    push.pop()

                push.insert(0, push_block)

                if self.scan_info and not (top_block := push[0]).tags:
                    top_block.tags.extend(self.scan_info)

        if inc_push and not (top_block := push[0]).tags:
            top_block.tags.extend(scan_info)
        else:
            self.scan_info = scan_info

        self.scan = next_scan

########################################

BlockId = int
Enums = tuple[int, int]

class EnumTape:
    tape: Tape

    offsets: list[int]

    l_edge: bool
    r_edge: bool

    enums: dict[BlockId, Enums]

    def __init__(self, tape: Tape):
        self.tape = tape

        self.offsets = [0, 0]

        self.l_edge = False
        self.r_edge = False

        self.enums = {
            id(block): (side, num)
            for side, span in enumerate((tape.lspan, tape.rspan))
            for num, block in enumerate(span, start = 1)
        }

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

        ind, offset = enums

        if offset > self.offsets[ind]:
            self.offsets[ind] = offset

    def apply_rule(self, rule: Rule) -> Count | None:
        for side, pos in rule.keys():
            self.check_offsets(
                (self.tape.rspan if side else self.tape.lspan)[pos])

        return self.tape.apply_rule(rule)

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
