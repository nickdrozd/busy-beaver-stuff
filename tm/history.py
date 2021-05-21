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
