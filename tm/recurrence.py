from collections import defaultdict
from typing import Dict, List, Optional, Tuple

class History:
    def __init__(self, tapes = None):
        self.tapes = [] if tapes is None else tapes
        self.beeps = []
        self.states: List[int] = []
        self.changes: List[bool] = []
        self.positions: List[int] = []
        self.actions = defaultdict(lambda: [])

    def add_position_at_step(self, step: int, pos: int):
        self.positions += [pos] * (step - len(self.positions))
        self.positions.append(pos)

    def add_state_at_step(self, step: int, state: int):
        self.states += [state] * (step - len(self.states))
        self.states.append(state)

    def add_tape_at_step(self, step: int, tape):
        self.tapes += [tape] * (step - len(self.tapes))
        self.tapes.append(tape)

    def add_change_at_step(self, step: int, change: bool):
        self.changes += [change] * (step - len(self.changes))
        self.changes.append(change)

    def calculate_beeps(
            self,
            through: Optional[int] = None,
    ) -> Dict[int, int]:
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

    def check_for_recurrence(
            self,
            step: int,
            action: Tuple[int, int]
    ) -> Optional[Tuple[int, int]]:
        for pstep in self.actions[action]:
            if (result := self.verify_lin_recurrence(
                    pstep,
                    step,
                    self.tapes[pstep],
                    self.tapes[step],
            )) is not None:
                return result

        return None

    def tape_is_fixed(self, start: int) -> bool:
        return not any(self.changes[start:])

    def verify_lin_recurrence(
            self,
            steps: int,
            recurrence: int,
            tape1 = None,
            tape2 = None,
    ) -> Optional[Tuple[int, int]]:
        if self.states[steps] != self.states[recurrence]:
            return None

        if tape1 is None or tape2 is None:
            tape1 = self.tapes[steps]
            tape2 = self.tapes[recurrence]

            if tape1 is None or tape2 is None:
                return None

        # pylint: disable = pointless-statement
        tape1[ tape2.lspan : tape2.rspan ]

        positions = self.positions

        pos1 = positions[steps]
        pos2 = positions[recurrence]

        if pos1 < pos2:
            diff = pos2 - pos1
            leftmost = min(positions[steps:])

            slice1 = tape1[        leftmost : ]
            slice2 = tape2[ diff + leftmost : ]

            if (slice_diff := len(slice1) - len(slice2)) > 0:
                slice2 = slice2 + [0] * slice_diff

        elif pos1 > pos2:
            diff = pos1 - pos2
            rightmost = max(positions[steps:]) + 1

            slice1 = tape1[ : rightmost        ]
            slice2 = tape2[ : rightmost - diff ]

            if (slice_diff := len(slice1) - len(slice2)) > 0:
                slice2 = [0] * slice_diff + slice2

        else:
            assert pos1 == pos2

            leftmost  = min(positions[steps:])
            rightmost = max(positions[steps:]) + 1

            slice1 = tape1[ leftmost : rightmost ]
            slice2 = tape2[ leftmost : rightmost ]

        return (
            (steps, _period := recurrence - steps)
            if slice1 == slice2 else
            None
        )
