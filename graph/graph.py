COLORS = (
    'blue',
    'red',
    'green',
    'black',
)

class Graph:
    def __init__(self, prog, colors=None):
        if colors is None:
            colors = 2

        if isinstance(prog, str):
            if ' ' in prog:
                prog = prog.split()
            else:
                prog = tuple(prog)

            try:
                colors = max(int(action[0]) for action in prog) + 1
            except ValueError:
                pass

        trans = iter(
            action[2] if len(action) == 3 else action
            for action in prog
        )

        connections = zip(*(trans for _ in range(colors)))

        self.arrows = {
            chr(65 + i): connection
            for i, connection in enumerate(connections)
        }

    def __str__(self):
        return self.flatten()

    @property
    def states(self):
        return tuple(self.arrows)

    @property
    def colors(self):
        return tuple(range(len(self.arrows['A'])))

    @property
    def dot(self):
        return 'digraph NAME {{\n{}\n}}'.format('\n'.join([
            f'  {node} -> {target} [ color=" {COLORS[i]}" ];'
            for node, targets in self.arrows.items()
            for i, target in enumerate(targets)
        ]))

    def flatten(self, sep=' '):
        return sep.join(
            dst
            for state in self.states
            for conn in self.arrows[state]
            for dst in conn
        )

    def is_normal(self):
        flat_graph = self.flatten('')

        if any(state not in flat_graph for state in self.states):
            return False

        positions = tuple(
            flat_graph.find(state)
            for state in self.states[1:]
        )

        return tuple(sorted(positions)) == positions

    def is_strongly_connected(self):
        states = set(self.states)

        for state in self.states:
            if not any(state in self.arrows[dst]
                       for dst in
                       states.difference(state)):
                return False

        for state in self.states:
            reachable_from_x = set(self.arrows[state]).difference(state)

            for _ in range(len(states.difference(state))):
                reachable_from_x.discard('H')

                reachable = {
                    node
                    for connection in reachable_from_x
                    for node in self.arrows[connection]
                }

                reachable_from_x.update(reachable)

            if not reachable_from_x.issuperset(states):
                return False

        return True
