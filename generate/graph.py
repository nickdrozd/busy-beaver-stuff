from typing import Dict, Set, Tuple

from tm import parse

COLORS = (
    'blue',
    'red',
    'forestgreen',
    'purple',
    'goldenrod',
    'black',
    'brown',
    'deeppink',
)

HALT = '_'

class Graph:
    def __init__(self, program: str):
        self.program = program

        self.arrows = {
            chr(65 + i): connection
            for i, connection in
            enumerate((
                tuple(instr[2] for instr in state)
                for state in parse(program)
            ))
        }

    def __str__(self):
        return self.flatten()

    def __repr__(self):
        return self.flatten()

    @property
    def states(self) -> Tuple[str, ...]:
        return tuple(self.arrows)

    @property
    def colors(self) -> Tuple[int, ...]:
        return tuple(range(len(self.arrows['A'])))

    @property
    def exit_points(self) -> Dict[str, Set[str]]:
        return {
            state: set(connections).difference(HALT).difference('.')
            for state, connections in self.arrows.items()
        }

    @property
    def entry_points(self) -> Dict[str, Set[str]]:
        entries: Dict[str, Set[str]] = {
            state: set()
            for state in self.states
        }

        for state, exits in self.arrows.items():
            for exit_point in exits:
                if exit_point == HALT:
                    continue

                try:
                    entries[exit_point].add(state)
                except KeyError:
                    pass

        return entries

    @property
    def dot(self) -> str:
        prog_format = '\n'.join([
            '  labelloc="t";',
            f'  label="{self.program}";',
            '  fontname="courier"',
        ])

        edges = '\n'.join([
            f'  {node} -> {target} [ color=" {COLORS[i]}" ];'
            for node, targets in self.arrows.items()
            for i, target in enumerate(targets)
            if target != '.'
        ])

        title = prog_format if len(prog_format) < 50 else ''

        return f'digraph NAME {{\n{title}\n\n{edges}\n}}'

    def flatten(self, sep=' ') -> str:
        return sep.join(
            dst
            for state in self.states
            for conn in self.arrows[state]
            for dst in conn
        )

    @property
    def is_normal(self) -> bool:
        flat_graph = self.flatten('')

        if any(state not in flat_graph for state in self.states):
            return False

        positions = tuple(
            flat_graph.find(state)
            for state in self.states[1:]
        )

        return tuple(sorted(positions)) == positions

    @property
    def is_strongly_connected(self) -> bool:
        states = set(self.states)

        for state in self.states:
            if not any(state in self.arrows[dst]
                       for dst in
                       states.difference(state)):
                return False

        for state in self.states:
            reachable_from_x = set(self.arrows[state]).difference(state)

            for _ in range(len(states.difference(state))):
                reachable_from_x.discard(HALT)

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
    def is_irreflexive(self) -> bool:
        return all(
            state not in connections
            for state, connections in self.arrows.items()
        )

    @property
    def entries_dispersed(self) -> bool:
        color_count = len(self.colors)

        return all(
            len(entries) == color_count
            for entries in self.entry_points.values()
        )

    @property
    def exits_dispersed(self) -> bool:
        color_count = len(self.colors)

        return all(
            len(exits) == color_count
            for exits in self.exit_points.values()
        )

    @property
    def is_dispersed(self) -> bool:
        return self.entries_dispersed and self.exits_dispersed
