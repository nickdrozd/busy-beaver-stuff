from __future__ import annotations

from copy import copy
from collections import defaultdict
from dataclasses import dataclass, field

from tm.tape import Tape
from tm.instrs import Color, State, Slot


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


RecRes = tuple[int, int]
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

    def add_tape_at_step(self, step: int, tape: Tape) -> None:
        pos = tape.head
        self.positions += [pos] * (step - len(self.positions))
        self.positions.append(pos)

        self.tapes[step] = PtrTape(
            sum(q for (_, q) in tape.lspan) - tape.head,
            [
                color
                for color, count in reversed(tape.lspan)
                for _ in range(count)
            ] + [tape.scan] + [
                color
                for color, count in tape.rspan
                for _ in range(count)
            ]
        )

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
        for pstep in self.slots[slot]:
            if (result := self.verify_lin_recurrence(
                    pstep,
                    step,
                    self.tapes[pstep],
                    self.tapes[step],
            )) is not None:
                return result

        return None

    def verify_lin_recurrence(
            self,
            steps: int,
            recurrence: int,
            tape1: PtrTape | None = None,
            tape2: PtrTape | None = None,
    ) -> RecRes | None:
        assert self.states[steps] == self.states[recurrence]

        if tape1 is None or tape2 is None:  # no-coverage
            tape1 = self.tapes[steps]
            tape2 = self.tapes[recurrence]

        assert tape1 is not None and tape2 is not None

        positions = self.positions

        if 0 < (diff := positions[recurrence] - positions[steps]):
            leftmost = min(positions[steps:])

            if tape2.r_end > tape1.r_end:
                # pylint: disable = pointless-statement
                tape1[ : tape2.r_end ]

            slice1 = tape1[        leftmost : ]
            slice2 = tape2[ diff + leftmost : ]

            slice2 = slice2 + [0] * (len(slice1) - len(slice2))

        elif diff < 0:
            rightmost = max(positions[steps:]) + 1

            if tape2.l_end < tape1.l_end:
                # pylint: disable = pointless-statement
                tape1[ tape2.l_end : ]

            slice1 = tape1[ : rightmost        ]
            slice2 = tape2[ : rightmost + diff ]

            slice2 = [0] * (len(slice1) - len(slice2)) + slice2

        else:
            assert diff == 0

            leftmost  = min(positions[steps:])
            rightmost = max(positions[steps:]) + 1

            slice1 = tape1[ leftmost : rightmost ]
            slice2 = tape2[ leftmost : rightmost ]

        return (
            (steps, _period := recurrence - steps)
            if slice1 == slice2 else
            None
        )