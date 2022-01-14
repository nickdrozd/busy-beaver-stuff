from collections import defaultdict
from typing import Dict, List, Optional, Tuple

class History:
    def __init__(self, tapes=None):
        self.tapes = [] if tapes is None else tapes
        self.beeps = []
        self.states: List[int] = []
        self.changes: List[bool] = []
        self.positions: List[int] = []
        self.actions = defaultdict(lambda: [])

    def add_position_at_step(self, pos: int, step: int):
        self.positions += [pos] * (step - len(self.positions))
        self.positions.append(pos)

    def add_state_at_step(self, state: int, step: int):
        self.states += [state] * (step - len(self.states))
        self.states.append(state)

    def add_tape_at_step(self, tape, step: int):
        self.tapes += [tape] * (step - len(self.tapes))
        self.tapes.append(tape)

    def add_change_at_step(self, change: bool, step: int):
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
            if self.states[pstep] != self.states[step]:
                continue

            if self.verify_lin_recurrence(pstep, step - pstep):
                return pstep, step - pstep

        return None

    def tape_is_fixed(self, start: int) -> bool:
        for tape1 in self.tapes[start:]:
            for tape2 in self.tapes[start + 1:]:
                # pylint: disable = pointless-statement
                tape1[ tape2.lspan : tape2.rspan ]

                if tape1[:] != tape2[:]:
                    return False

        return True

    def verify_lin_recurrence(self, steps: int, period: int) -> bool:
        tapes     = self.tapes
        positions = self.positions

        recurrence = steps + period

        tape1 = tapes[steps]
        tape2 = tapes[recurrence]

        if tape1 is None or tape2 is None:
            return False

        # pylint: disable = pointless-statement
        tape1[ tape2.lspan : tape2.rspan ]

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

        return slice1 == slice2
