from __future__ import annotations

from copy import copy
from collections import defaultdict
from dataclasses import dataclass, field

from tm.parse import CompProg
from tm.tape import PtrTape, Tape, Signature
from tm.macro import MacroProg

State = int | str
Color = int | str
Action = tuple[State, Color]

RecRes = tuple[int, int] | None  # type: ignore[misc]
Tapes = dict[int, PtrTape]

Rule = tuple[tuple[int, ...], ...]

@dataclass
class History:
    tapes: Tapes

    states: list[State] = field(default_factory = list)
    positions: list[int] = field(default_factory = list)

    actions: dict[Action, list[int]] = field(
        default_factory = lambda: defaultdict(list))

    def copy(self) -> History:
        new_copy = History(tapes = dict(self.tapes.items()))

        new_copy.states = copy(self.states)
        new_copy.positions = copy(self.positions)

        new_copy.actions = defaultdict(list)

        for action, steps in self.actions.items():
            new_copy.actions[action] = copy(steps)

        return new_copy

    def add_action_at_step(self, step: int, action: Action) -> None:
        self.actions[action].append(step)

    def add_state_at_step(self, step: int, state: State) -> None:
        self.states += [state] * (step - len(self.states))
        self.states.append(state)

    def add_tape_at_step(self, step: int, tape: Tape) -> None:
        self.tapes[step] = tape.to_ptr()

        pos = tape.head
        self.positions += [pos] * (step - len(self.positions))
        self.positions.append(pos)

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

    def check_rec(self, step: int, action: Action) -> RecRes:
        for pstep in self.actions[action]:
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
    ) -> RecRes:
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
    last_cycle: int | None = None
    last_delta: int | None = None
    tape: Tape | None = None

    def check(self, cycle: int, tape: Tape) -> bool:
        self.times_seen += 1

        if self.last_cycle is None:
            self.last_cycle = cycle
            return False

        delta = cycle - self.last_cycle

        if self.last_delta is None or self.last_delta != delta:
            self.last_delta = delta
            self.last_cycle = cycle
            self.tape = tape.copy()
            return False

        return True

class InfiniteRule(Exception):
    pass

@dataclass
class Prover:
    prog: CompProg | MacroProg

    diff_lim: int | None

    rules: dict[
        Action,
        dict[Signature, Rule],
    ] = field(default_factory = dict)

    configs: dict[
        Signature,
        dict[State, PastConfig],
    ] = field(default_factory = dict)

    def get_rule(
            self,
            state: State,
            tape: Tape,
            sig: Signature | None = None,
    ) -> Rule | None:
        if (temp := self.rules.get((state, tape.scan))) is None:
            return None

        return temp.get(sig or tape.signature)

    def add_rule(
            self,
            state: State,
            sig: Signature,
            rule: Rule,
    ) -> None:
        if (action := (state, sig[0])) not in self.rules:
            self.rules[action] = {}

        self.rules[action][sig] = rule

    @staticmethod
    def apply_rule(tape: Tape, rule: Rule) -> int | None:
        diffs, blocks = (
            rule[0] + rule[1],
            tape.lspan + tape.rspan,
        )

        divs = []

        for diff, block in zip(diffs, blocks):
            if diff < 0:
                if (abs_diff := abs(diff)) >= block[1]:
                    return None

                div, rem = divmod(block[1], abs_diff)
                divs.append(div if rem > 0 else div - 1)

        times = min(divs)

        for diff, block in zip(diffs, blocks):
            block[1] += diff * times

        return times

    def run_simulator(
            self,
            steps: int,
            state: State,
            tape: Tape,
    ) -> tuple[bool, State] | None:
        rec_rule = False

        for _ in range(steps):
            if (rule := self.get_rule(state, tape)) is not None:
                if self.apply_rule(tape, rule) is not None:
                    rec_rule = True
                    continue

            try:
                color, shift, next_state = \
                    self.prog[state][tape.scan] # type: ignore
            except TypeError:
                return None

            _ = tape.step(
                shift,
                color,
                state == next_state)

            state = next_state

        return rec_rule, state

    def try_rule(
            self,
            cycle: int,
            state: State,
            tape: Tape,
    ) -> int | None:
        # If we already have a rule, apply it
        if (rule := self.get_rule(
                state, tape, sig := tape.signature)) is not None:
            return self.apply_rule(tape, rule)

        # If this is a new config, record it
        if (temp := self.configs.get(sig)) is None:
            temp = defaultdict(PastConfig)
            self.configs[sig] = temp

        if not (past_config := temp[state]).check(cycle, tape):
            return None

        assert (last_delta := past_config.last_delta) is not None

        if self.diff_lim is not None and last_delta > self.diff_lim:
            return None

        assert (past_tape := past_config.tape) is not None

        for curr_span, prev_span in zip(tape.spans, past_tape.spans):
            assert len(curr_span) == len(prev_span)

            for old, new in zip(prev_span, curr_span):
                assert old[0] == new[0]

                if (diff := old[1] - new[1]) > 0 and new[1] <= diff:
                    return None

        tape_copy = tape.copy()

        spans = tuple(zip(tape_copy.spans, past_tape.spans))

        for curr_span, prev_span in spans:
            for num, (old, new) in enumerate(zip(prev_span, curr_span)):
                if new[1] > 1:
                    new.append(num)

        if (result := self.run_simulator(
                last_delta, state, tape_copy)) is None:
            return None

        rec_rule, end_state = result

        if end_state != state:
            return None

        if tape_copy.scan != sig[0]:
            return None

        for curr_span, prev_span in spans:
            for num, (old, new) in enumerate(zip(prev_span, curr_span)):
                if new[1] > 1:
                    if len(new) != 3:
                        return None

        if tape_copy.signature != sig:
            return None

        rule = tuple(
            tuple(
                old[1] - new[1]
                for old, new in zip(*spans)
            ) for spans in zip(tape.spans, past_tape.spans)
        )

        if any(diff < 0 for span in rule for diff in span):
            self.add_rule(state, sig, rule)

            return self.apply_rule(tape, rule)

        if not rec_rule:
            raise InfiniteRule()

        for _ in range(last_delta):
            result = self.run_simulator(
                last_delta, state, tape_copy)
            assert result is not None

            rec_rule, end_state = result

            if end_state != state:
                return None

            if tape_copy.signature != sig:
                return None

        raise InfiniteRule()
