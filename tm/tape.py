from __future__ import annotations

from dataclasses import dataclass
from functools import lru_cache
from typing import TYPE_CHECKING

from tm.show import show_number

if TYPE_CHECKING:
    from tm.num import Count
    from tm.parse import Color, Shift

    Word = tuple[Color, ...]
    ProverWord = tuple[Color, ...]
    SigWord = ProverWord | tuple[ProverWord]
    Signature = tuple[Color, tuple[SigWord, ...], tuple[SigWord, ...]]
    Index = tuple[int, int]
    Counts = tuple[list[Count], list[Count]]
    MinSig = tuple[Signature, tuple[bool, bool]]

MAX_NORMALIZE_BLOCKS = 16
MAX_NORMALIZE_CELLS = 128
MAX_PERIOD = 8
MIN_PERIODIC_REPEATS = 3
MIN_PERIODIC_CELLS = 8
MIN_COMPACT_CONCRETE_BLOCKS = 2
MIN_COMPACT_CONCRETE_CELLS = 12


def is_plain_int(val: object) -> bool:
    return isinstance(val, int)


def is_zero(val: object) -> bool:
    try:
        return bool(val == 0)
    except Exception:
        return False


def is_one(val: object) -> bool:
    try:
        return bool(val == 1)
    except Exception:
        return False


def rotate(word: Word, phase: int) -> Word:
    if not word:
        return word
    phase %= len(word)
    if phase == 0:
        return word
    return word[phase:] + word[:phase]


def _word_at(word: Word, phase: int, idx: int) -> Color:
    return word[(phase + idx) % len(word)]


@lru_cache(maxsize=4096)
def primitive_root(word: Word) -> Word:
    n = len(word)
    for period in range(1, n + 1):
        if n % period == 0:
            root = word[:period]
            if root * (n // period) == word:
                return root
    raise AssertionError('primitive root must exist')


@lru_cache(maxsize=4096)
def canonical_rotation(word: Word) -> tuple[Word, int]:
    root = primitive_root(word)
    n = len(root)
    best_phase = 0
    for phase in range(1, n):
        cand_is_better = False
        for idx in range(n):
            a = _word_at(root, phase, idx)
            b = _word_at(root, best_phase, idx)
            if a < b:
                cand_is_better = True
                break
            if a > b:
                break
        if cand_is_better:
            best_phase = phase
    best = rotate(root, best_phase)
    for phase in range(n):
        if rotate(best, phase) == root:
            return best, phase
    raise AssertionError('rotation mismatch')


@lru_cache(maxsize=4096)
def encode_word(word: Word) -> tuple[Word, int]:
    return canonical_rotation(word)


@dataclass(slots=True)
class Block:
    root: Word
    phase: int
    count: Count

    def __post_init__(self) -> None:
        if not self.root:
            raise ValueError('block root must be non-empty')
        if is_zero(self.count):
            raise ValueError('block count must be non-zero')
        canon, phase0 = encode_word(self.root)
        self.root = canon
        self.phase = (self.phase + phase0) % len(self.root)

    @classmethod
    def _from_canonical(cls, root: Word, phase: int, count: Count) -> Block:
        if not root:
            raise ValueError('block root must be non-empty')
        if is_zero(count):
            raise ValueError('block count must be non-zero')
        obj = cls.__new__(cls)
        obj.root = root
        obj.phase = phase % len(root)
        obj.count = count
        return obj

    @classmethod
    def from_tape_word(cls, word: Word, count: Count) -> Block:
        if not word:
            raise ValueError('block word must be non-empty')
        if len(word) == 1:
            return cls._from_canonical(word, 0, count)
        prim = primitive_root(word)
        mult = len(word) // len(prim)
        canon, phase = encode_word(prim)
        return cls._from_canonical(canon, phase, count * mult)

    def clone(self) -> Block:
        return Block._from_canonical(self.root, self.phase, self.count)

    @property
    def word(self) -> Word:
        return rotate(self.root, self.phase)

    def __str__(self) -> str:
        body = ''.join(map(str, self.word))
        if is_one(self.count):
            return body
        return f"{body}^{show_number(self.count)}" if len(self.root) == 1 else f"({body})^{show_number(self.count)}"

    def expand(self) -> list[Color]:
        if not is_plain_int(self.count):
            raise TypeError('cannot fully expand dynamic block with non-int count')
        return list(self.word) * self.count

    def boundary_symbol(self, side: int) -> Color:
        tape_word = self.word
        return tape_word[-1] if side == 0 else tape_word[0]

    def boundary_run_len(self, side: int, color: Color) -> int:
        tape_word = self.word
        seq = reversed(tape_word) if side == 0 else tape_word
        run = 0
        for cell in seq:
            if cell != color:
                break
            run += 1
        if run == 0:
            return 0
        return self.count if len(self.root) == 1 else run


@dataclass(slots=True)
class Tape:
    lspan: list[Block]
    scan: Color
    rspan: list[Block]

    def __init__(self, lspan: list[Block] | None = None, scan: Color = 0, rspan: list[Block] | None = None):
        self.lspan = [b.clone() for b in (lspan or [])]
        self.scan = scan
        self.rspan = [b.clone() for b in (rspan or [])]
        self.normalize_span(0)
        self.normalize_span(1)

    def clone(self) -> Tape:
        return Tape(self.lspan, self.scan, self.rspan)

    def to_enum(self) -> EnumTape:
        return EnumTape(self.clone())

    def __str__(self) -> str:
        return ' '.join([*map(str, reversed(self.lspan)), f'[{self.scan}]', *map(str, self.rspan)])

    @property
    def blank(self) -> bool:
        return self.scan == 0 and not self.lspan and not self.rspan

    @property
    def marks(self) -> Count:
        total = 1 if self.scan != 0 else 0
        for block in (*self.lspan, *self.rspan):
            word = block.word
            nonzero = sum(1 for cell in word if cell != 0)
            if nonzero == 0:
                continue
            if nonzero == len(word):
                total += len(word) * block.count
            else:
                total += nonzero * block.count
        return total

    def at_edge(self, edge: Shift) -> bool:
        return self.scan == 0 and not (self.rspan if edge else self.lspan)

    @property
    def length_one_spans(self) -> bool:
        return len(self.lspan) == 1 and len(self.rspan) == 1

    @property
    def counts(self) -> Counts:
        return ([b.count for b in self.lspan], [b.count for b in self.rspan])

    @property
    def blocks(self) -> int:
        return len(self.lspan) + len(self.rspan)

    @property
    def signature(self) -> Signature:
        return (
            self.scan,
            tuple(sig_item_side(b, 0) for b in self.lspan),
            tuple(sig_item_side(b, 1) for b in self.rspan),
        )

    def get_count(self, index: Index) -> Count:
        side, pos = index
        return (self.rspan if side else self.lspan)[pos].count

    def set_count(self, index: Index, val: Count) -> None:
        side, pos = index
        span = self.rspan if side else self.lspan
        span[pos].count = val

    def sig_compatible(self, sig: Signature) -> bool:
        scan, lspan, rspan = sig
        return (
            self.scan == scan
            and tuple(sig_item_side(b, 0) for b in self.lspan) == lspan
            and tuple(sig_item_side(b, 1) for b in self.rspan) == rspan
        )

    def normalize_span(
        self,
        side: int,
        *,
        max_blocks: int = MAX_NORMALIZE_BLOCKS,
        max_cells: int = MAX_NORMALIZE_CELLS,
        max_period: int = MAX_PERIOD,
    ) -> None:
        span = self.rspan if side else self.lspan
        if not span:
            return
        merge_adjacent_inplace(span)
        if not should_periodically_normalize(span, side, max_blocks=max_blocks, max_cells=max_cells):
            return
        normalize_frontier_segments(
            span,
            side,
            max_blocks=max_blocks,
            max_cells=max_cells,
            max_period=max_period,
        )
        if should_compact_span(span, side, max_blocks=max_blocks, max_cells=max_cells):
            compact_span(
                span,
                side,
                max_blocks=max_blocks,
                max_cells=max_cells,
                max_period=max_period,
            )

    def _consume_boundary_cells(self, side: int, amount: Count, *, normalize: bool = True) -> None:
        span = self.rspan if side else self.lspan
        while not is_zero(amount) and span:
            block = span.pop(0)
            bword = block_boundary_word(block, side)
            wlen = len(bword)

            if wlen == 1:
                if amount == block.count:
                    amount = 0
                    continue
                remain_count = block.count - amount
                if not is_zero(remain_count):
                    span.insert(0, Block._from_canonical((bword[0],), 0, remain_count))
                amount = 0
                break

            if not is_plain_int(amount):
                raise TypeError('cannot consume non-int amount from non-unary dynamic block')

            full_reps, rem = divmod(amount, wlen)
            remaining_full = block.count - full_reps

            if rem == 0:
                if not is_zero(remaining_full):
                    span.insert(0, Block.from_tape_word(boundary_word_to_tape_word(bword, side), remaining_full))
                amount = 0
                break

            after_partial_full = remaining_full - 1
            inserts_boundary: list[Block] = []
            partial = bword[rem:]
            if partial:
                inserts_boundary.append(
                    Block._from_canonical((partial[0],), 0, 1)
                    if len(partial) == 1 else Block.from_tape_word(boundary_word_to_tape_word(partial, side), 1)
                )
            if not is_zero(after_partial_full):
                inserts_boundary.append(Block.from_tape_word(boundary_word_to_tape_word(bword, side), after_partial_full))
            span[:0] = inserts_boundary
            amount = 0
            break

        if normalize:
            self.normalize_span(side)

    def _pull_run(self, side: int, color: Color) -> int:
        span = self.rspan if side else self.lspan
        if not span:
            return 0
        run = span[0].boundary_run_len(side, color)
        if run:
            self._consume_boundary_cells(side, run, normalize=False)
        return run

    def _pull_scan_cell(self, side: int) -> Color:
        span = self.rspan if side else self.lspan
        if not span:
            return 0
        symbol = span[0].boundary_symbol(side)
        self._consume_boundary_cells(side, 1, normalize=False)
        return symbol

    def _push_cells(self, side: int, color: Color, count: int, *, normalize: bool = True) -> None:
        if is_zero(count):
            return
        span = self.rspan if side else self.lspan
        if not span and color == 0:
            return
        span.insert(0, Block._from_canonical((color,), 0, count))
        if normalize:
            self.normalize_span(side)

    def step(self, shift: Shift, color: Color, skip: bool) -> Count:
        pull_side, push_side = ((1, 0) if shift else (0, 1))
        skipped = self._pull_run(pull_side, self.scan) if skip else 0
        stepped = 1 + skipped
        next_scan = self._pull_scan_cell(pull_side)
        self._push_cells(push_side, color, stepped, normalize=False)
        self.scan = next_scan

        self.normalize_span(pull_side)
        if push_side != pull_side:
            self.normalize_span(push_side)
        return stepped


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
            for num, block in enumerate(span, start=1)
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
        if offset > getattr(self, s_offset):
            setattr(self, s_offset, offset)

    def get_count(self, index: Index) -> Count:
        side, pos = index
        block = (self.tape.rspan if side else self.tape.lspan)[pos]
        self.check_offsets(block)
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
        pull_side = 1 if shift else 0
        push_side = 0 if shift else 1
        pull = self.tape.rspan if pull_side else self.tape.lspan
        push = self.tape.rspan if push_side else self.tape.lspan
        if not pull:
            self.touch_edge(shift)
        else:
            self.check_offsets(pull[0])
        if skip and pull:
            self.check_offsets(pull[0])
        if push:
            self.check_offsets(push[0])
        _ = self.tape.step(shift, color, skip)

    def get_min_sig(self, sig: Signature) -> MinSig:
        return sig, (True, True)


def prover_word(block: Block, side: int) -> tuple[int, ...]:
    if len(block.root) == 1:
        return block.root
    boundary = block.boundary_symbol(side)
    return (boundary, *block.root)


def sig_item_side(block: Block, side: int) -> SigWord:
    word = prover_word(block, side)
    return word if not is_one(block.count) else (word,)


def span_to_boundary_cells(span: list[Block], side: int) -> list[Color]:
    cells: list[Color] = []
    if side == 0:
        for block in span:
            cells.extend(reversed(block.expand()))
    else:
        for block in span:
            cells.extend(block.expand())
    return cells


def expand_prefix_blocks(span: list[Block], side: int, *, max_blocks: int, max_cells: int) -> tuple[list[Color], int]:
    if not span:
        return [], 0
    cells: list[Color] = []
    used = 0
    for block in span[:max_blocks]:
        if not is_plain_int(block.count):
            break
        block_cells = list(reversed(block.expand())) if side == 0 else block.expand()
        if used > 0 and len(cells) + len(block_cells) > max_cells:
            break
        cells.extend(block_cells)
        used += 1
        if len(cells) >= max_cells:
            break
    return cells, used


def frontier_concrete_stats(span: list[Block], side: int, *, max_blocks: int, max_cells: int) -> tuple[int, int]:
    concrete_blocks = 0
    concrete_cells = 0
    for block in span[:max_blocks]:
        if not is_plain_int(block.count):
            break
        concrete_blocks += 1
        concrete_cells += len(block.root) * block.count
        if concrete_cells >= max_cells:
            concrete_cells = max_cells
            break
    return concrete_blocks, concrete_cells


def should_periodically_normalize(span: list[Block], side: int, *, max_blocks: int, max_cells: int) -> bool:
    concrete_blocks, concrete_cells = frontier_concrete_stats(span, side, max_blocks=max_blocks, max_cells=max_cells)
    if concrete_blocks == 0:
        return False
    if concrete_blocks == 1 and len(span[0].root) == 1:
        return False
    return concrete_cells >= 4


def should_compact_span(span: list[Block], side: int, *, max_blocks: int, max_cells: int) -> bool:
    concrete_blocks, concrete_cells = frontier_concrete_stats(span, side, max_blocks=max_blocks, max_cells=max_cells)
    return concrete_blocks >= MIN_COMPACT_CONCRETE_BLOCKS and concrete_cells >= MIN_COMPACT_CONCRETE_CELLS


def normalize_frontier_segments(
    span: list[Block],
    side: int,
    *,
    max_blocks: int,
    max_cells: int,
    max_period: int,
) -> None:
    if not span:
        return

    rebuilt: list[Block] = []
    i = 0
    frontier = min(len(span), max_blocks)

    while i < frontier:
        block = span[i]
        if not is_plain_int(block.count):
            rebuilt.append(block)
            i += 1
            continue

        cells, used = expand_prefix_blocks(span[i:frontier], side, max_blocks=frontier - i, max_cells=max_cells)
        if used == 0:
            rebuilt.append(block)
            i += 1
            continue

        factored = boundary_cells_to_span(factor_boundary_cells(cells, max_period=max_period), side)
        if used == 1 and len(factored) == 1 and same_block_shape(factored[0], block):
            rebuilt.append(block)
        else:
            rebuilt.extend(factored)
        i += used

    rebuilt.extend(span[frontier:])
    span[:] = rebuilt
    merge_adjacent_inplace(span)


def block_boundary_word(block: Block, side: int) -> Word:
    word = block.word
    return tuple(reversed(word)) if side == 0 else word


def match_period_prefix(cells: list[Color], period: Word) -> int:
    matched = 0
    plen = len(period)
    while matched < len(cells) and cells[matched] == period[matched % plen]:
        matched += 1
    return matched


def extend_periodic_at(
    span: list[Block],
    side: int,
    index: int,
    *,
    max_blocks: int,
    max_cells: int,
    max_period: int,
) -> bool:
    if index < 0 or index + 1 >= len(span):
        return False
    first = span[index]
    period = block_boundary_word(first, side)
    if len(period) > max_period:
        return False

    tail_cells, used = expand_prefix_blocks(
        span[index + 1:],
        side,
        max_blocks=max_blocks,
        max_cells=max_cells,
    )
    if not tail_cells or used == 0:
        return False

    matched = match_period_prefix(tail_cells, period)
    full_periods = matched // len(period)
    if full_periods <= 0:
        return False

    first.count += full_periods
    consumed = full_periods * len(period)
    remainder_cells = tail_cells[consumed:]
    rebuilt_tail = boundary_cells_to_span(factor_boundary_cells(remainder_cells, max_period=max_period), side)
    span[index + 1:index + 1 + used] = rebuilt_tail
    merge_around(span, index)
    return True


def compact_span(
    span: list[Block],
    side: int,
    *,
    max_blocks: int = MAX_NORMALIZE_BLOCKS,
    max_cells: int = MAX_NORMALIZE_CELLS,
    max_period: int,
) -> None:
    limit = min(len(span), max_blocks)
    idx = 0
    while idx + 1 < limit:
        block = span[idx]
        if len(block.root) == 1 or is_one(block.count):
            idx += 1
            continue
        _ = extend_periodic_at(
            span,
            side,
            idx,
            max_blocks=max_blocks - idx - 1,
            max_cells=max_cells,
            max_period=max_period,
        )
        limit = min(len(span), max_blocks)
        idx += 1


def boundary_word_to_tape_word(word: Word, side: int) -> Word:
    return word if side == 1 else tuple(reversed(word))


def boundary_cells_to_span(blocks_boundary_order: list[Block], side: int) -> list[Block]:
    out: list[Block] = []
    for block in blocks_boundary_order:
        if len(block.root) == 1:
            out.append(Block._from_canonical(block.root, 0, block.count))
        else:
            out.append(Block.from_tape_word(boundary_word_to_tape_word(block.word, side), block.count))
    return out


def merge_adjacent_inplace(blocks: list[Block]) -> bool:
    if not blocks:
        return False
    write = 1
    prev = blocks[0]
    changed = False
    for read in range(1, len(blocks)):
        block = blocks[read]
        if prev.root == block.root and prev.phase == block.phase:
            prev.count += block.count
            changed = True
        else:
            blocks[write] = block
            prev = block
            write += 1
    if write != len(blocks):
        del blocks[write:]
        changed = True
    return changed


def merge_around(span: list[Block], index: int) -> None:
    if not span:
        return
    start = max(0, index - 1)
    end = min(len(span), index + 3)
    chunk = span[start:end]
    merge_adjacent_inplace(chunk)
    span[start:end] = chunk


@lru_cache(maxsize=32768)
def _match_rotated_reps_cached(cells_t: tuple[Color, ...], root: Word, phase: int) -> int:
    p = len(root)
    pat = rotate(root, phase)
    reps = 0
    n = len(cells_t)
    pos = 0
    while pos + p <= n and cells_t[pos:pos + p] == pat:
        reps += 1
        pos += p
    return reps


def match_rotated_reps(cells: list[Color], root: Word, phase: int) -> int:
    return _match_rotated_reps_cached(tuple(cells), root, phase)


def best_periodic_prefix(cells: list[Color], max_period: int) -> tuple[Word, int, int] | None:
    best: tuple[Word, int, int] | None = None
    best_cover = 0
    n = len(cells)
    if n < 2:
        return None
    limit = min(max_period, n)
    cells_t = tuple(cells)
    for period in range(1, limit + 1):
        if period * (n // period) < best_cover:
            continue

        sample = cells_t[:period]
        if period == 1:
            if cells_t[0] != sample[0]:
                continue
        else:
            second = period * 2
            if second > n or cells_t[period:second] != sample:
                continue
            if MIN_PERIODIC_REPEATS >= 3:
                third = second + period
                if third > n or cells_t[second:third] != sample:
                    continue

        canon, _ = encode_word(sample)
        clen = len(canon)
        if clen != period:
            continue

        min_reps = 1 if clen == 1 else MIN_PERIODIC_REPEATS
        min_cover = clen if clen == 1 else max(MIN_PERIODIC_CELLS, clen * min_reps)
        if min_cover > n:
            continue

        max_possible_reps = n // clen
        if clen * max_possible_reps < max(best_cover, min_cover):
            continue

        reps = _match_rotated_reps_cached(cells_t, canon, 0)
        if reps < min_reps:
            continue

        cover = clen * reps
        if cover < min_cover:
            continue

        if cover > best_cover or (cover == best_cover and best is not None and clen < len(best[0])):
            best = (canon, 0, reps)
            best_cover = cover
        elif best is None:
            best = (canon, 0, reps)
            best_cover = cover
    return best


def factor_boundary_cells(cells: list[Color], *, max_period: int = MAX_PERIOD) -> list[Block]:
    out: list[Block] = []
    i = 0
    while i < len(cells):
        found = best_periodic_prefix(cells[i:], max_period)
        if found is None:
            block = Block._from_canonical((cells[i],), 0, 1)
            i += 1
        else:
            root, phase, reps = found
            block = Block._from_canonical(root, phase, reps)
            i += len(root) * reps
        if out and out[-1].root == block.root and out[-1].phase == block.phase:
            out[-1].count += block.count
        else:
            out.append(block)
    return out


def same_block_shape(a: Block, b: Block) -> bool:
    return a.root == b.root and a.phase == b.phase and a.count == b.count


def signatureish(blocks: list[Block]) -> tuple[tuple[Word, int, int], ...]:
    return tuple((b.root, b.phase, repr(b.count)) for b in blocks)
