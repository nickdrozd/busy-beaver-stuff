from __future__ import annotations

from copy import copy
from typing import TYPE_CHECKING
from collections import defaultdict
from dataclasses import dataclass, field

from tm.parse import tcompile
from tm.rust_stuff import TermRes, MachineResult

if TYPE_CHECKING:
    from typing import Self

    from tm.parse import Color, Shift, State, Slot, CompProg
    from tm.machine import Undfnd

    RecRes = tuple[int, int]
    LinRec = tuple[int | None, int]

    TapeSlice = list[Color]

########################################

@dataclass(slots = True)
class PtrTape:
    init: int
    scan: Color
    tape: list[Color]

    @classmethod
    def from_head(cls, tape: HeadTape) -> Self:
        return cls(
            sum(int(q.count) for q in tape.lspan) - tape.head,
            tape.scan,
            [
                block.color
                for block in reversed(tape.lspan)
                for _ in range(int(block.count))
            ] + [tape.scan] + [
                block.color
                for block in tape.rspan
                for _ in range(int(block.count))
            ],
        )

    def get_ltr(self, start: int) -> TapeSlice:
        start += self.init

        return (
            self.tape[ start : ]
            if (ldiff := -start) <= 0 else
            [0] * ldiff + self.tape
        )

    def get_rtl(self, stop: int) -> TapeSlice:
        stop += self.init + 1

        return (
            self.tape[ : stop ]
            if (rdiff := stop - len(self.tape)) <= 0 else
            self.tape[ : stop - rdiff ] + [0] * rdiff
        )

    def get_cnt(self, start: int, stop: int) -> TapeSlice:
        start += self.init
        stop += self.init + 1

        return [
            self.tape[pos] if 0 <= pos < len(self.tape) else 0
            for pos in range(start, stop)
        ]

    def aligns_with_optional_offsets(
            self,
            prev: PtrTape,
            diff: int,
            leftmost: int | None = None,
            rightmost: int | None = None,
    ) -> bool:
        if 0 < diff:
            assert leftmost is not None
            slice1 = prev.get_ltr(leftmost)
            slice2 = self.get_ltr(leftmost + diff)

        elif diff < 0:
            assert rightmost is not None
            slice1 = prev.get_rtl(rightmost)
            slice2 = self.get_rtl(rightmost + diff)

        else:
            assert leftmost is not None
            assert rightmost is not None
            slice1 = prev.get_cnt(leftmost, rightmost)
            slice2 = self.get_cnt(leftmost, rightmost)

        return slice1 == slice2

########################################

@dataclass(slots = True)
class HeadBlock:
    color: Color
    count: int

    def __str__(self) -> str:
        return f"{self.color}^{self.count}"


@dataclass(slots = True)
class HeadTape:
    lspan: list[HeadBlock]
    scan: Color
    rspan: list[HeadBlock]

    head: int = 0

    def __init__(
            self,
            lspan: list[HeadBlock] | None = None,
            scan: Color = 0,
            rspan: list[HeadBlock] | None = None,
            head: int = 0,
    ):
        self.lspan = lspan or []
        self.scan = scan
        self.rspan = rspan or []
        self.head = head

    def __str__(self) -> str:
        return ' '.join(
            list(map(str, reversed(self.lspan)))
            + [f'[{self.scan} ({self.head})]']
            + list(map(str, self.rspan)))

    def copy(self) -> HeadTape:
        return HeadTape(
            [HeadBlock(blk.color, blk.count) for blk in self.lspan],
            self.scan,
            [HeadBlock(blk.color, blk.count) for blk in self.rspan],
            head = self.head,
        )

    @property
    def blank(self) -> bool:
        return self.scan == 0 and not self.lspan and not self.rspan

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
            if skip and pull and pull[0].color == self.scan else
            None
        )

        stepped = 1 if push_block is None else 1 + push_block.count

        next_scan: Color

        if not pull:
            next_scan = 0
        else:
            next_pull = pull[0]

            if next_pull.count != 1:
                next_pull.count -= 1
            else:
                popped = pull.pop(0)

                if push_block is None:
                    push_block = popped
                    push_block.count = 0

            next_scan = next_pull.color

        if push and (top_block := push[0]).color == color:
            top_block.count += stepped
        elif push or color != 0:
            if push_block is None:
                push_block = HeadBlock(color, 1)
            else:
                push_block.color = color
                push_block.count += 1

            push.insert(0, push_block)

        self.scan = next_scan

        self.head += stepped if shift else -stepped

        return stepped

    def get_slice(self, start: int, *, ltr: bool) -> TapeSlice:
        if ltr:
            lspan, rspan = self.lspan, self.rspan
            diff = self.head - start
        else:
            lspan, rspan = self.rspan, self.lspan
            diff = start - self.head

        tape: TapeSlice = []

        if diff > 0:
            for block in lspan:
                if diff <= (count := block.count):
                    tape.extend(
                        [block.color] * diff)

                    break

                tape.extend(
                    [block.color] * count)

                diff -= count

            else:
                assert diff > 0

                tape.extend(
                    [0] * diff)

            tape.reverse()

        tape.append(self.scan)

        for block in rspan:
            tape.extend(
                [block.color] * block.count)

        return tape

    def get_ltr(self, start: int) -> TapeSlice:
        return self.get_slice(start, ltr = True)

    def get_rtl(self, start: int) -> TapeSlice:
        return self.get_slice(start, ltr = False)

    def get_cnt(self, start: int, stop: int) -> TapeSlice:
        assert start <= (head := self.head) <= stop

        if start == head:
            return self.get_ltr(start)

        if stop == head:
            return self.get_rtl(start)

        return self.get_rtl(head - 1) + self.get_ltr(head)

    def aligns_with(
            self,
            prev: HeadTape,
            leftmost: int,
            rightmost: int,
    ) -> bool:
        if 0 < (diff := self.head - prev.head):
            slice1 = prev.get_ltr(leftmost)
            slice2 = self.get_ltr(leftmost + diff)

        elif diff < 0:
            slice1 = prev.get_rtl(rightmost)
            slice2 = self.get_rtl(rightmost + diff)

        else:
            slice1 = prev.get_cnt(leftmost, rightmost)
            slice2 = self.get_cnt(leftmost, rightmost)

        return slice1 == slice2


def init_stepped() -> HeadTape:
    return HeadTape([HeadBlock(1, 1)], 0, [], head = 1)

########################################

if TYPE_CHECKING:
    Tapes = dict[int, PtrTape | None]


@dataclass(slots = True)
class History:
    tapes: Tapes

    positions: list[int] = field(default_factory = list)

    slots: dict[Slot, list[int]] = field(
        default_factory = lambda: defaultdict(list))

    def copy(self) -> History:
        return History(
            tapes = copy(self.tapes),
            positions = copy(self.positions),
            slots = defaultdict(
                list,
                {
                    slot: copy(steps)
                    for slot, steps in self.slots.items()
                },
            )
        )

    def add_slot_at_step(self, step: int, slot: Slot) -> None:
        self.slots[slot].append(step)

    def add_tape_at_step(self, step: int, tape: HeadTape) -> None:
        pos = tape.head

        self.positions += [pos] * (step - len(self.positions))
        self.positions.append(pos)

        self.tapes[step] = PtrTape.from_head(tape)

    def check_rec(self, step: int, slot: Slot) -> RecRes | None:
        for pstep in self.slots[slot]:
            if (result := self.verify_lin_rec(
                    pstep,
                    step,
            )) is not None:
                return result

        return None

    def verify_lin_rec(
            self,
            steps: int,
            recur: int,
    ) -> RecRes | None:
        tape1 = self.tapes[steps]
        tape2 = self.tapes[recur]

        assert tape1 is not None
        assert tape2 is not None

        positions = self.positions

        if 0 < (diff := positions[recur] - positions[steps]):
            aligns = tape2.aligns_with_optional_offsets(
                tape1,
                diff,
                leftmost = min(positions[steps:]),
            )

        elif diff < 0:
            aligns = tape2.aligns_with_optional_offsets(
                tape1,
                diff,
                rightmost = max(positions[steps:]),
            )

        else:
            aligns = tape2.aligns_with_optional_offsets(
                tape1,
                diff,
                leftmost = min(positions[steps:]),
                rightmost = max(positions[steps:]),
            )

        return (
            (steps, recur - steps)
            if aligns else
            None
        )


@dataclass(slots = True)
class BeepHistory(History):
    states: list[State] = field(default_factory = list)

    def add_state_at_step(self, step: int, state: State) -> None:
        self.states += [state] * (step - len(self.states))
        self.states.append(state)

    def calculate_beeps(
            self,
            through: int | None = None,
    ) -> dict[State, int]:
        states = (
            self.states
            if through is None else
            self.states[:through]
        )

        steps = len(states)
        rev   = list(reversed(states))

        return {
            state: steps - 1 - rev.index(state)
            for state in set(states)
        }


class LinRecMachine:
    comp: CompProg

    tape: HeadTape
    steps: int
    cycles: int

    blanks: dict[State, int]

    halted: int | None = None
    spnout: int | None = None
    xlimit: int | None = None

    undfnd: Undfnd | None = None

    infrul: int | None = None

    def __init__(self, prog: str):
        self.comp = tcompile(prog)


class StrictLinRecMachine(LinRecMachine):
    history: BeepHistory

    linrec: LinRec | None = None
    qsihlt: bool | None = None

    def run(
        self,
        sim_lim: int | None = None,
        check_rec: int = 0,
    ) -> Self:
        self.blanks = {}

        comp = self.comp

        self.tape = tape = HeadTape()

        self.history = BeepHistory(tapes = {})

        step: int = 0
        state: State = 0

        for cycle in range(sim_lim or 1_000_000):
            slot: Slot = state, tape.scan

            if step >= check_rec:
                self.history.add_state_at_step(step, state)
                self.history.add_tape_at_step(step, tape)

                if self.check_rec(step, slot) is not None:
                    break

                self.history.add_slot_at_step(step, slot)

            try:
                instr = comp[slot]
            except KeyError:
                self.undfnd = step, slot
                break

            color, shift, next_state = instr

            step += tape.step(shift, color, state == next_state)

            state = next_state

            if not color and tape.blank and state not in self.blanks:
                self.blanks[state] = step

        else:
            self.xlimit = step

        self.cycles = cycle

        return self

    def check_rec(self, step: int, slot: Slot) -> RecRes | None:
        if (result := self.history.check_rec(step, slot)) is None:
            return None

        self.linrec = start, rec = result

        if rec == 1:
            self.spnout = step - 1

        hc_beeps = self.history.calculate_beeps()
        hp_beeps = self.history.calculate_beeps(start)

        self.qsihlt = any(
            hc_beeps[st] <= hp_beeps[st]
            for st in hp_beeps
        )

        return result


class LinRecSampler(LinRecMachine):
    history: BeepHistory

    def run(
        self,
        sim_lim: int,
        samples: Tapes,
    ) -> Self:
        self.blanks = {}

        comp = self.comp

        self.tape = tape = HeadTape()

        self.history = BeepHistory(tapes = samples)

        step: int = 0
        state: State = 0

        for cycle in range(sim_lim or 1_000_000):
            slot: Slot = state, tape.scan

            if step in self.history.tapes:
                self.history.add_state_at_step(step, state)
                self.history.add_tape_at_step(step, tape)

            try:
                instr = comp[slot]
            except KeyError:
                self.undfnd = step, slot
                break

            color, shift, next_state = instr

            step += tape.step(shift, color, False)

            state = next_state

            if not color and tape.blank and state not in self.blanks:
                self.blanks[state] = step

        else:
            self.xlimit = step

        self.cycles = cycle

        return self


def run_loose_linrec_machine(
        program: str,
        sim_lim: int = 100_000_000,
) -> MachineResult:
    # pylint: disable = while-used, too-many-locals
    blanks = {}

    comp = tcompile(program)

    state = 1

    step = 1

    tape = init_stepped()

    cycle = 1

    result: TermRes | None = None
    last_slot: Slot | None = None

    while cycle < sim_lim:
        steps_reset = 2 * step

        leftmost = rightmost = tape.head

        init_state = state

        init_tape = tape.copy()

        while step < steps_reset and cycle < sim_lim:
            try:
                instr = comp[state, tape.scan]
            except KeyError:
                last_slot = state, tape.scan
                result = TermRes.undfnd
                break

            color, shift, next_state = instr

            if (same := state == next_state) and tape.at_edge(shift):
                result = TermRes.spnout
                break

            stepped = tape.step(shift, color, same)

            step += stepped

            cycle += 1

            state = next_state

            if not color and tape.blank:
                if state in blanks:
                    result = TermRes.infrul
                    break

                blanks[state] = step

                if state == 0:
                    result = TermRes.infrul
                    break

            if (curr := tape.head) < leftmost:
                leftmost = curr
            elif rightmost < curr:
                rightmost = curr

            if state != init_state:
                continue

            if tape.scan != init_tape.scan:
                continue

            if tape.aligns_with(init_tape, leftmost, rightmost):
                result = TermRes.infrul
                break

        else:
            continue

        break

    else:
        result = TermRes.xlimit

    return MachineResult(
        result = result,
        steps = step,
        cycles = cycle,
        marks = 0,
        rulapp = 0,
        last_slot = last_slot,
        blanks = blanks,
    )
