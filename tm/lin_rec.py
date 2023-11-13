from __future__ import annotations

from copy import copy
from typing import TYPE_CHECKING
from collections import defaultdict
from dataclasses import dataclass, field

from tm.tape import Tape, Block
from tm.machine import BasicMachine

if TYPE_CHECKING:
    from typing import Self

    from tm.tape import Shift
    from tm.parse import Color, State, Slot

    RecRes = tuple[int, int]


@dataclass
class HeadTape(Tape):
    head: int = 0

    def __init__(
            self,
            lspan: list[Block],
            scan: Color,
            rspan: list[Block],
            head: int = 0,
    ):
        self.head = head

        super().__init__(lspan, scan, rspan)

    def __hash__(self) -> int:
        return hash((
            self.scan,
            tuple((block.color, block.count) for block in self.lspan),
            tuple((block.color, block.count) for block in self.rspan),
        ))

    def copy(self) -> HeadTape:
        return HeadTape(
            [Block(block.color, block.count) for block in self.lspan],
            self.scan,
            [Block(block.color, block.count) for block in self.rspan],
            head = self.head,
        )

    def to_ptr(self) -> PtrTape:
        return PtrTape(
            sum(int(q.count) for q in self.lspan) - self.head,
            self.unroll(),
        )

    def unroll(self) -> list[Color]:
        return [
            block.color
            for block in reversed(self.lspan)
            for _ in range(int(block.count))
        ] + [self.scan] + [
            block.color
            for block in self.rspan
            for _ in range(int(block.count))
        ]

    def step(self, shift: Shift, color: Color, skip: bool) -> int:
        stepped = int(super().step(shift, color, skip))

        self.head += stepped if shift else -stepped

        return stepped


@dataclass
class PtrTape:
    init: int
    tape: list[Color]

    def __getitem__(self, pos: int) -> Color:
        pos += self.init

        if pos < 0:
            return 0

        try:
            return self.tape[pos]
        except IndexError:
            return 0

    @property
    def r_end(self) -> int:
        return len(self.tape) - self.init

    @property
    def l_end(self) -> int:
        return 0 - self.init

    def get_ltr(self, start: int) -> list[Color]:
        stop = self.r_end + 1

        self.extend_to_bound_left(start)

        return self.tape[ start + self.init : stop + self.init ]

    def get_rtl(self, stop: int) -> list[Color]:
        start = self.l_end

        self.extend_to_bound_right(stop)

        return self.tape[ start + self.init : stop + self.init ]

    def extend_to_bound_right(self, stop: int) -> None:
        if (rdiff := stop + self.init - self.r_end) > 0:
            self.tape.extend([0] * rdiff)

    def extend_to_bound_left(self, start: int) -> None:
        if (ldiff := 0 - (start + self.init)) > 0:
            self.tape = [0] * ldiff + self.tape
            self.init += ldiff


if TYPE_CHECKING:
    Tapes = dict[int, PtrTape]


@dataclass
class History:
    tapes: Tapes

    states: list[State] = field(default_factory = list)
    positions: list[int] = field(default_factory = list)

    slots: dict[Slot, list[int]] = field(
        default_factory = lambda: defaultdict(list))

    def copy(self) -> History:
        new_copy = History(tapes = dict(self.tapes.items()))

        new_copy.states = copy(self.states)
        new_copy.positions = copy(self.positions)

        new_copy.slots = defaultdict(list)

        for slot, steps in self.slots.items():
            new_copy.slots[slot] = copy(steps)

        return new_copy

    def add_slot_at_step(self, step: int, slot: Slot) -> None:
        self.slots[slot].append(step)

    def add_state_at_step(self, step: int, state: State) -> None:
        self.states += [state] * (step - len(self.states))
        self.states.append(state)

    def add_tape_at_step(self, step: int, tape: HeadTape) -> None:
        pos = tape.head

        self.positions += [pos] * (step - len(self.positions))
        self.positions.append(pos)

        self.tapes[step] = tape.to_ptr()

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

    def check_rec(self, step: int, slot: Slot) -> RecRes | None:
        return next((
            result
            for pstep in self.slots[slot]
            if (result :=
                self.verify_lin_recurrence(pstep, step)
            ) is not None
        ), None)

    def verify_lin_recurrence(
            self,
            steps: int,
            recurrence: int,
    ) -> RecRes | None:
        tape1 = self.tapes[steps]
        tape2 = self.tapes[recurrence]

        positions = self.positions

        if 0 < (diff := positions[recurrence] - positions[steps]):
            leftmost = min(positions[steps:])

            slice1 = tape1.get_ltr(leftmost)
            slice2 = tape2.get_ltr(leftmost + diff)

            recur = slice1[ : len(slice2)] == slice2 and all(
                cell == 0 for cell in
                slice1[len(slice2) : ]
            )

        elif diff < 0:
            rightmost = max(positions[steps:]) + 1

            if tape2.l_end < tape1.l_end:
                tape1.extend_to_bound_left(tape2.l_end)

            slice1 = tape1.get_rtl(rightmost)
            slice2 = tape2.get_rtl(rightmost + diff)

            recur = slice1[-len(slice2) : ] == slice2 and all(
                cell == 0 for cell in
                slice1[ : len(slice1) - len(slice2)]
            )

        else:
            leftmost  = min(positions[steps:])
            rightmost = max(positions[steps:]) + 1

            for pos in range(leftmost, rightmost):
                if tape1[pos] != tape2[pos]:
                    return None

            recur = True

        return (
            (steps, _period := recurrence - steps)
            if recur else
            None
        )


class LinRecMachine(BasicMachine):
    history: History

    def run(
        self,
        sim_lim: int | None = None,
        check_rec: int = 0,
    ) -> Self:
        self.blanks = {}

        comp = self.comp

        self.tape = tape = HeadTape.init()

        self.history = History(tapes = {})

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

            if (instr := comp[slot]) is None:
                self.undfnd = step, slot
                break

            color, shift, next_state = instr

            step += tape.step(shift, color, state == next_state)

            if (state := next_state) == -1:  # no-cover
                self.halted = step
                break

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
