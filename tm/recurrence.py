from __future__ import annotations

from copy import copy
from collections import defaultdict
from dataclasses import dataclass
from typing import Any, Dict, List, Optional, Tuple

from tm.tape import PtrTape, BlockTape
from tm.types import Action, State

RecRes = Optional[Tuple[int, int]]
Tapes = Dict[int, PtrTape]

Config = Tuple[State, str]
Rule = Tuple[Tuple[int, ...], ...]

class History:
    def __init__(self, tapes: Optional[Tapes] = None):
        self.tapes: Tapes = {} if tapes is None else tapes

        self.states: List[State] = []
        self.positions: List[int] = []
        self.actions: Dict[Action, List[int]] = defaultdict(list)

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

    def add_tape_at_step(self, step: int, tape: BlockTape) -> None:
        self.tapes[step] = tape.to_ptr()

        pos = tape.head
        self.positions += [pos] * (step - len(self.positions))
        self.positions.append(pos)

    def calculate_beeps(
            self,
            through: Optional[int] = None,
    ) -> Dict[State, int]:
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
            tape1: Optional[PtrTape] = None,
            tape2: Optional[PtrTape] = None,
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
    last_cycle: Optional[int] = None
    last_delta: Optional[int] = None
    tape: Optional[BlockTape] = None

    def check(self, cycle: int, tape: BlockTape) -> bool:
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

class Prover:
    # pylint: disable = too-few-public-methods

    def __init__(self, prog: Any):
        self.prog: Any = prog

        self.configs: Dict[Config, PastConfig] = defaultdict(PastConfig)
        self.rules: Dict[Config, Rule] = {}

    def try_rule(
            self,
            cycle: int,
            state: State,
            tape: BlockTape,
    ) -> Optional[int]:
        # If we already have a rule, apply it
        if (config := (state, tape.signature)) in self.rules:
            rule = self.rules[config]

            diffs, blocks = (
                rule[0] + rule[1],
                tape.lspan + tape.rspan,
            )

            if any(diff < 0 and abs(diff) >= block[1]
                   for diff, block in zip(diffs, blocks)):
                return None

            for diff, block in zip(diffs, blocks):
                block[1] += diff

            return 1

        # If this is a new config, record it
        if not (past_config := self.configs[config]).check(cycle, tape):
            return None

        assert (past_tape := past_config.tape) is not None

        ldiffs = tuple(
            old[1] - new[1]
            for old, new in
            zip(tape.lspan, past_tape.lspan)
        )

        rdiffs = tuple(
            old[1] - new[1]
            for old, new in
            zip(tape.rspan, past_tape.rspan)
        )

        block_diffs = ldiffs, rdiffs

        if all(diff >= 0 for span in block_diffs for diff in span):
            raise InfiniteRule()

        # pylint: disable = bad-builtin
        for span in ('lspan', 'rspan'):
            curr_span = getattr(tape, span)
            prev_span = getattr(past_tape, span)
            assert len(curr_span) == len(prev_span)

            for old, new in zip(prev_span, curr_span):
                assert old[0] == new[0]

                if (diff := (old[1] - new[1])) > 0 and new[1] <= diff:
                    return None

        tape_copy = tape.copy()

        for span in ('lspan', 'rspan'):
            curr_span = getattr(tape_copy, span)
            prev_span = getattr(past_tape, span)

            for num, (old, new) in enumerate(zip(prev_span, curr_span)):
                if old[1] != new[1]:
                    new.append(num)

        state_copy = state

        assert past_config.last_delta is not None

        for _ in range(past_config.last_delta):
            color, shift, next_state = \
                self.prog[state_copy][tape_copy.scan]

            _ = tape_copy.step(
                shift,
                color,
                state_copy == next_state)

            state_copy = next_state

        for span in ('lspan', 'rspan'):
            curr_span = getattr(tape_copy, span)
            prev_span = getattr(past_tape, span)

            for num, (old, new) in enumerate(zip(prev_span, curr_span)):
                if old[1] != new[1]:

                    if len(new) != 3:
                        return None

                    new.pop()

        try:
            copy_sig = tape_copy.signature
        except ValueError:
            return None

        if config != (state_copy, copy_sig):
            return None

        self.rules[config] = block_diffs

        return None
