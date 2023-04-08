from __future__ import annotations

import math
from dataclasses import dataclass, field

from tm.instrs import Color, Shift
from tm.rules import Counts, Index, ApplyRule

Block     = list[int]
BlockSpan = list[Block]

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

def show_block(block: Block) -> str:
    color, count, *_ = block

    return f"{color}^{show_number(count)}"


@dataclass
class BlockTape(ApplyRule):
    lspan: BlockSpan
    scan: Color
    rspan: BlockSpan

    def __str__(self) -> str:
        return ' '.join(
            list(map(show_block, reversed(self.lspan)))
            + [f'[{self.scan}]']
            + list(map(show_block, self.rspan)))

    @property
    def counts(self) -> Counts:
        return (
            tuple(block[1] for block in self.lspan),
            tuple(block[1] for block in self.rspan),
        )

    @property
    def spans(self) -> tuple[BlockSpan, BlockSpan]:
        return self.lspan, self.rspan

    @property
    def signature(self) -> Signature:
        return (
            self.scan,
            tuple(c if q != 1 else (c,) for (c, q, *_) in self.lspan),
            tuple(c if q != 1 else (c,) for (c, q, *_) in self.rspan),
        )

    def __getitem__(self, index: Index) -> int:
        side, pos = index

        span = self.rspan if side else self.lspan

        return span[pos][1]

    def __setitem__(self, index: Index, val: int) -> None:
        side, pos = index

        span = self.rspan if side else self.lspan

        span[pos][1] = val


@dataclass
class Tape(BlockTape):
    head: int = 0

    def __hash__(self) -> int:
        return hash(str(self))

    @staticmethod
    def init(scan: Color = 0) -> Tape:
        return Tape([], scan, [])

    def lblocks(self) -> BlockSpan:
        return [[color, count] for color, count in self.lspan]

    def rblocks(self) -> BlockSpan:
        return [[color, count] for color, count in self.rspan]

    def copy(self) -> Tape:
        tape = Tape(self.lblocks(), self.scan, self.rblocks())
        tape.head = self.head
        return tape

    def to_tag(self) -> TagTape:
        return TagTape(self.lblocks(), self.scan, self.rblocks())

    def to_enum(self) -> EnumTape:
        return EnumTape(self.lblocks(), self.scan, self.rblocks())

    def to_ptr(self) -> PtrTape:
        return PtrTape(
            sum(q for (_, q) in self.lspan) - self.head,
            [
                color
                for color, count in reversed(self.lspan)
                for _ in range(count)
            ] + [self.scan] + [
                color
                for color, count in self.rspan
                for _ in range(count)
            ]
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
            pull.pop(0)
            if skip and pull and pull[0][0] == self.scan else
            None
        )

        stepped = 1 if push_block is None else 1 + push_block[1]

        next_scan: Color

        if not pull:
            next_scan = 0
        else:
            next_scan = (next_pull := pull[0])[0]

            if next_pull[1] > 1:
                next_pull[1] -= 1
            else:
                popped = pull.pop(0)

                if push_block is None:
                    push_block = popped
                    push_block[1] = 0

        if push and (top_block := push[0])[0] == color:
            top_block[1] += stepped
        else:
            if push_block is None:
                push_block = [color, 1]
            else:
                push_block[0] = color
                push_block[1] += 1

            if  push or color != 0:
                push.insert(0, push_block)

        self.scan = next_scan

        if shift:
            self.head += stepped
        else:
            self.head -= stepped

        return stepped


@dataclass
class TagTape(BlockTape):
    scan_info: list[int] = field(
        default_factory = list)

    def missing_tags(self) -> bool:
        return any(
            block[1] > 1 and len(block) != 3
            for span in self.spans
            for num, block in enumerate(span))

    def step(self, shift: Shift, color: Color, skip: bool) -> None:
        pull, push = (
            (self.rspan, self.lspan)
            if shift else
            (self.lspan, self.rspan)
        )

        push_block = (
            pull.pop(0)
            if (skip :=
                (skip and bool(pull) and pull[0][0] == self.scan)) else
            None
        )

        stepped = 1 if push_block is None else 1 + push_block[1]

        scan_info: list[int] = []

        next_scan: Color

        dec_pull: bool = False
        inc_push: bool = False

        if not pull:
            next_scan = 0
        else:
            next_scan = (next_pull := pull[0])[0]

            if next_pull[1] > 1:
                next_pull[1] -= 1
                dec_pull = True
            else:
                popped = pull.pop(0)

                if push_block is None:
                    push_block = popped
                    push_block[1] = 0

                if (extra := popped[2:]):
                    scan_info += extra
                    push_block = push_block[:2]

        if push and (top_block := push[0])[0] == color:
            inc_push = True
            top_block[1] += stepped
            top_block += self.scan_info

            if push_block is not None:
                top_block += push_block[2:]
        else:
            if push_block is None:
                push_block = [color, 1]

                if push and color != self.scan:
                    if len(top_block := push[0]) > 3:
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

                if push and len(top_block := push[0]) > 3:
                    push_block.append(
                        top_block.pop())

                if self.scan_info:
                    push_block.extend(
                        self.scan_info)

            if push or color != 0 or push_block[2:] or skip:
                if color == 0 and not push:
                    push_block[1] = 1

                push.insert(0, push_block)

                if self.scan_info and not (top_block := push[0])[2:]:
                    top_block.extend(
                        self.scan_info)

        if inc_push and not (top_block := push[0])[2:]:
            top_block.extend(scan_info)
        else:
            self.scan_info = scan_info

        self.scan = next_scan


@dataclass
class EnumTape(BlockTape):
    offsets: list[int] = field(
        default_factory = lambda: [0, 0])

    _edges: list[bool] = field(
        default_factory = lambda: [False, False])

    def __post_init__(self) -> None:
        for s, span in enumerate(self.spans):
            for i, block in enumerate(span, start = 1):
                block.append(s)
                block.append(i)

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
            if other := (near_block := pull[0])[2:]:
                ind, offset = other

                if offset > self.offsets[ind]:
                    self.offsets[ind] = offset

            if skip and near_block[0] == self.scan:
                if not pull[1:]:
                    self._edges[bool(shift)] = True
                elif next_block := pull[1][2:]:
                    ind, offset = next_block

                    if offset > self.offsets[ind]:
                        self.offsets[ind] = offset

        if push and (other := (opp := push[0])[2:]) and color == opp[0]:
            ind, offset = other

            if offset > self.offsets[ind]:
                self.offsets[ind] = offset

        push_block = (
            pull.pop(0)
            if skip and pull and pull[0][0] == self.scan else
            None
        )

        stepped = 1 if push_block is None else 1 + push_block[1]

        next_scan: Color

        if not pull:
            next_scan = 0
        else:
            next_scan = (next_pull := pull[0])[0]

            if next_pull[1] > 1:
                next_pull[1] -= 1
            else:
                popped = pull.pop(0)

                if push_block is None:
                    push_block = popped
                    push_block[1] = 0

        if push and (top_block := push[0])[0] == color:
            top_block[1] += stepped
        else:
            if push_block is None:
                push_block = [color, 1]
            else:
                push_block[0] = color
                push_block[1] += 1

            if  push or color != 0:
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
