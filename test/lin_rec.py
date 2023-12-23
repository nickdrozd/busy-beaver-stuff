from __future__ import annotations

from copy import copy
from typing import TYPE_CHECKING
from collections import defaultdict
from dataclasses import dataclass, field

from tm.parse import tcompile
from tm.tape import HeadTape, init_stepped

if TYPE_CHECKING:
    from typing import Self

    from tm.parse import Color, Shift, State, Slot, GetInstr
    from tm.machine import Undfnd

    RecRes = tuple[int, int]
    LinRec = tuple[int | None, int]

    TapeSlice = list[Color]


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


if TYPE_CHECKING:
    Tapes = dict[int, PtrTape]


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
    program: str
    comp: GetInstr

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
        self.program = prog
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

            if (instr := comp[slot]) is None:
                self.undfnd = step, slot
                break

            color, shift, next_state = instr

            step += tape.step(shift, color, False)

            if (state := next_state) == -1:
                self.halted = step
                break

            if not color and tape.blank and state not in self.blanks:
                self.blanks[state] = step

        else:
            self.xlimit = step

        self.cycles = cycle

        return self


class LooseLinRecMachine(LinRecMachine):
    # pylint: disable = while-used, too-many-locals, line-too-long
    def run(self, sim_lim: int) -> Self:  # no-cover
        self.blanks = {}

        comp = self.comp

        state = 1

        step = 1

        self.tape = tape = init_stepped()

        cycle = 1

        while cycle < sim_lim:
            steps_reset = 2 * step

            leftmost = rightmost = tape.head

            init_state = state

            init_tape = tape.copy()

            while step < steps_reset and cycle < sim_lim:
                if (instr := comp[state, tape.scan]) is None:
                    self.undfnd = step, (state, tape.scan)
                    break

                color, shift, next_state = instr

                if (same := state == next_state) and tape.at_edge(shift):
                    self.spnout = step
                    break

                stepped = tape.step(shift, color, same)

                step += stepped

                cycle += 1

                if (state := next_state) == -1:
                    self.halted = step
                    break

                if not color and tape.blank:
                    if state in self.blanks:
                        self.infrul = step
                        break

                    self.blanks[state] = step

                    if state == 0:
                        self.infrul = step
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
                    self.infrul = step
                    break

            else:
                continue

            self.xlimit = None

            break

        else:
            self.xlimit = stepped

        self.cycles = cycle

        return self
