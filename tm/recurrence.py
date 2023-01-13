from __future__ import annotations

from copy import copy
from collections import defaultdict
from dataclasses import dataclass, field

from tm.instrs import State, Slot, GetInstr
from tm.tape import PtrTape, Tape, TagTape, Signature, Rule

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
                for color, count in tape.lspan
                for _ in range(count)
            ] + [tape.scan] + [
                color
                for color, count in reversed(tape.rspan)
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

        if tape1 is None or tape2 is None:
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

#################################################

@dataclass
class PastConfig:
    cycles: list[int] = field(default_factory = list)

    def next_deltas(self, cycle: int) -> tuple[int, int] | None:
        (cycles := self.cycles).append(cycle)

        if len(cycles) < 4:
            return None

        # pylint: disable = invalid-name

        *_, d, c, b, a = cycles

        cycles.pop(0)

        for i in range(1, 5):
            p1 = a - (b * i)
            p2 = b - (c * i)
            p3 = c - (d * i)

            if p1 == p2 == p3:
                nxt = a * i + p2
                nxxt = nxt * i + p1

                return (
                    nxt - cycle,
                    nxxt - nxt,
                )

        return None

class InfiniteRule(Exception):
    pass

@dataclass
class Prover:
    prog: GetInstr

    diff_lim: int | None

    rules: dict[
        Slot,
        dict[Signature, Rule],
    ] = field(default_factory = dict)

    configs: dict[
        Signature,
        dict[State, PastConfig],
    ] = field(default_factory = dict)

    def get_rule(
            self,
            state: State,
            tape: Tape | TagTape,
            sig: Signature | None = None,
    ) -> Rule | None:
        if (temp := self.rules.get((state, tape.scan))) is None:
            return None

        return temp.get(sig or tape.signature)

    def run_simulator(
            self,
            steps: int,
            state: State,
            tape: TagTape,
    ) -> tuple[bool, State] | None:
        implausible = False

        for _ in range(steps):
            if (rule := self.get_rule(state, tape)) is not None:
                marks = tape.marks

                if tape.apply_rule(rule) is not None:
                    if abs(tape.marks - marks) > steps:
                        implausible = True

                    continue

            if (instr := self.prog[state, tape.scan]) is None:
                return None

            color, shift, next_state = instr

            tape.step(
                shift,
                color,
                state == next_state)

            if (state := next_state) == -1:
                return None

        return implausible, state

    def try_rule(
            self,
            cycle: int,
            state: State,
            tape: Tape,
    ) -> Rule | None:
        if (known_rule := self.get_rule(
                state, tape, sig := tape.signature)) is not None:
            return known_rule

        if (temp := self.configs.get(sig)) is None:
            temp = defaultdict(PastConfig)
            self.configs[sig] = temp

        if (deltas := temp[state].next_deltas(cycle)) is None:
            return None

        if deltas[0] > (self.diff_lim or 0):
            return None

        tags: TagTape = tape.to_tag()

        for curr_span, prev_span in zip(tags.spans, tape.spans):
            for num, (old, new) in enumerate(zip(prev_span, curr_span)):
                if new[1] > 1:
                    new.append(num)

        counts = []

        implausible = False

        for delta in deltas:
            if (result := self.run_simulator(
                    delta, state, tags)) is None:
                return None

            imp, end_state = result

            if imp:
                implausible = True

            if (
                end_state != state
                or tags.scan != sig[0]
                or any(
                    new[1] > 1 and len(new) != 3
                    for curr_span, prev_span in
                        zip(tags.spans, tape.spans)
                    for num, (old, new) in
                        enumerate(zip(prev_span, curr_span)))
                or tags.signature != sig
            ):
                return None

            counts.append(tags.counts)

        if (rule := tape.make_rule(implausible, *counts)) is None:
            return None

        if all(diff >= 0
               for diff in rule.values()
               if isinstance(diff, int)):
            raise InfiniteRule()

        if (slot := (state, sig[0])) not in self.rules:
            self.rules[slot] = {}

        self.rules[slot][sig] = rule

        return rule
