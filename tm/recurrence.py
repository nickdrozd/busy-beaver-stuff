from __future__ import annotations

from copy import copy
from collections import defaultdict
from dataclasses import dataclass, field

from tm.instrs import State, Slot, GetInstr
from tm.rules import make_rule, Rule, ImplausibleRule
from tm.tape import (
    Signature, MinSig,
    Tape, BlockTape, TagTape, EnumTape, PtrTape,
)

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
    times_seen: int = 0

    cycles: list[int] = field(default_factory = list)

    def next_deltas(self, cycle: int) -> tuple[int, int] | None:
        self.times_seen += 1

        (cycles := self.cycles).append(cycle)

        if len(cycles) < 5:
            return None

        # pylint: disable = invalid-name

        *_, e, d, c, b, a = cycles

        cycles.pop(0)

        for i in range(1, 5):
            p1 = a - (b * i)
            p2 = b - (c * i)
            p3 = c - (d * i)
            p4 = d - (e * i)

            if (diff := p1 - p2) == p2 - p3 == p3 - p4:
                nxt = a * i + p1 + diff
                nxxt = nxt * i + p1 + 2 * diff

                return (
                    nxt - cycle,
                    nxxt - nxt,
                )

        return None


@dataclass
class Prover:
    prog: GetInstr

    rules: dict[
        Slot,
        dict[MinSig, Rule],
    ] = field(default_factory = dict)

    configs: dict[
        Signature,
        dict[State, PastConfig],
    ] = field(default_factory = dict)

    def get_rule(
            self,
            state: State,
            tape: BlockTape,
            sig: Signature | None = None,
    ) -> Rule | None:
        if (temp := self.rules.get((state, tape.scan))) is None:
            return None

        if sig is None:
            sig = tape.signature

        for ((scan, lspan, rspan), (lex, rex)), rule in temp.items():
            if (scan == sig[0]
                and lspan == (sig[1] if lex else sig[1][:len(lspan)])
                and rspan == (sig[2] if rex else sig[2][:len(rspan)])):
                return rule

        return None

    def set_rule(
            self,
            rule: Rule,
            state: State,
            sig: MinSig,
    ) -> None:
        if (slot := (state, sig[0][0])) not in self.rules:
            self.rules[slot] = {}

        self.rules[slot][sig] = rule

    def run_simulator(
            self,
            steps: int,
            state: State,
            tape: TagTape,
    ) -> tuple[int, State] | None:
        rec_rule = 0

        for _ in range(steps):
            if (rule := self.get_rule(state, tape)) is not None:
                if tape.apply_rule(rule) is not None:
                    rec_rule += 1
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

        return rec_rule, state

    def get_min_sig(
            self,
            steps: int,
            state: State,
            tape: EnumTape,
            sig: Signature,
    ) -> Signature:
        for _ in range(steps):
            if (rule := self.get_rule(state, tape)) is not None:
                if tape.apply_rule(rule) is not None:
                    continue

            assert (instr := self.prog[state, tape.scan]) is not None

            color, shift, next_state = instr

            tape.step(
                shift,
                color,
                state == next_state)

            state = next_state

        lmax, rmax = tape.offsets

        return sig[0], sig[1][:lmax], sig[2][:rmax]

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

        tags: TagTape = tape.to_tag()

        for span in tags.spans:
            for num, block in enumerate(span):
                if block[1] > 1:
                    block.append(num)

        counts = []

        rec_rule = 0

        for delta in deltas:
            if (result := self.run_simulator(
                    delta, state, tags)) is None:
                return None

            rec, end_state = result

            if rec:
                rec_rule = rec

            if (
                end_state != state
                or tags.scan != sig[0]
                or tags.signature != sig
                or tags.missing_tags()
            ):
                return None

            counts.append(tags.counts)

        try:
            # pylint: disable = no-value-for-parameter
            rule = make_rule(rec_rule, tape.counts, *counts)
        except ImplausibleRule:
            return None

        min_sig = self.get_min_sig(
            deltas[0],
            state,
            (etap := tape.to_enum()),
            sig,
        )

        self.set_rule(
            rule,
            state,
            (min_sig, (etap.edges[0], etap.edges[1])),
        )

        return rule
