from __future__ import annotations

from copy import copy
from collections import defaultdict
from typing import Dict, List, Optional, Tuple

from tm.tape import PtrTape, BlockTape
from tm.types import Action, State

RecRes = Optional[Tuple[int, int]]
Tapes = Dict[int, PtrTape]

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
