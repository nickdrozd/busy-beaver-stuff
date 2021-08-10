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
            prog = (
                prog.split()
                if ' ' in prog else
                tuple(prog)
            )

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

        self.prog = prog

    def __str__(self):
        return self.flatten()

    def __repr__(self):
        return self.flatten()

    @property
    def states(self):
        return tuple(self.arrows)

    @property
    def colors(self):
        return tuple(range(len(self.arrows['A'])))

    @property
    def exit_points(self):
        return {
            state: set(connections).difference('H').difference('.')
            for state, connections in self.arrows.items()
        }

    @property
    def entry_points(self):
        entries = {state: set() for state in self.states}

        for state, exits in self.arrows.items():
            for exit_point in exits:
                if exit_point == 'H':
                    continue

                try:
                    entries[exit_point].add(state)
                except KeyError:
                    pass

        return entries

    @property
    def dot(self):
        prog = ' '.join(self.prog)

        title = f'labelloc="t"; label="{prog}";'

        edges = '\n'.join([
            f'  {node} -> {target} [ color=" {COLORS[i]}" ];'
            for node, targets in self.arrows.items()
            for i, target in enumerate(targets)
        ])

        return f'digraph NAME {{{title}\n{edges}\n}}'

    def flatten(self, sep=' '):
        return sep.join(
            dst
            for state in self.states
            for conn in self.arrows[state]
            for dst in conn
        )

    @property
    def is_normal(self):
        flat_graph = self.flatten('')

        if any(state not in flat_graph for state in self.states):
            return False

        positions = tuple(
            flat_graph.find(state)
            for state in self.states[1:]
        )

        return tuple(sorted(positions)) == positions

    @property
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
                    if connection in self.arrows
                    for node in self.arrows[connection]
                }

                reachable_from_x.update(reachable)

            if not reachable_from_x.issuperset(states):
                return False

        return True

    @property
    def is_irreflexive(self):
        return all(
            state not in connections
            for state, connections in self.arrows.items()
        )

    @property
    def entries_dispersed(self):
        color_count = len(self.colors)

        return all(
            len(entries) == color_count
            for entries in self.entry_points.values()
        )

    @property
    def exits_dispersed(self):
        color_count = len(self.colors)

        return all(
            len(exits) == color_count
            for exits in self.exit_points.values()
        )

    @property
    def is_dispersed(self):
        return self.entries_dispersed and self.exits_dispersed
