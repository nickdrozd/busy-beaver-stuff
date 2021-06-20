from collections import defaultdict

class History:
    def __init__(self, tapes=None):
        self.tapes = [] if tapes is None else tapes
        self.beeps = []
        self.states = []
        self.actions = defaultdict(lambda: [])
        self.positions = []

    def calculate_beeps(self, through=None):
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

    def check_for_recurrence(self, step, action):
        for pstep in self.actions[action]:
            if verify_lin_recurrence(pstep, step - pstep, self):
                return pstep, step - pstep

        return None


def verify_lin_recurrence(steps, period, history):
    tapes     = history.tapes
    states    = history.states
    positions = history.positions

    recurrence = steps + period

    st1 = states[steps]
    st2 = states[recurrence]

    tape1 = tapes[steps]
    tape2 = tapes[recurrence]

    # pylint: disable = pointless-statement
    tape1[ tape2.lspan : tape2.rspan ]

    pos1 = positions[steps]
    pos2 = positions[recurrence]

    if st1 != st2:
        return False

    if pos1 < pos2:
        diff = pos2 - pos1
        leftmost = min(positions[steps:])

        slice1 = tape1[        leftmost : ]
        slice2 = tape2[ diff + leftmost : ]

        slice_diff = len(slice1) - len(slice2)

        if slice_diff > 0:
            slice2 = slice2 + [0] * slice_diff

    elif pos1 > pos2:
        diff = pos1 - pos2
        rightmost = max(positions[steps:]) + 1

        slice1 = tape1[ : rightmost        ]
        slice2 = tape2[ : rightmost - diff ]

        slice_diff = len(slice1) - len(slice2)

        if slice_diff > 0:
            slice2 = [0] * slice_diff + slice2

    else:
        assert pos1 == pos2

        leftmost  = min(positions[steps:])
        rightmost = max(positions[steps:]) + 1

        slice1 = tape1[ leftmost : rightmost ]
        slice2 = tape2[ leftmost : rightmost ]

    return slice1 == slice2
