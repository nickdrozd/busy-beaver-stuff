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
UNDEFINED = '.'

class Graph:
    def __init__(self, program: str):
        self.program = program

        self.arrows: Dict[str, Tuple[str, ...]] = {
            chr(65 + i): connection
            for i, connection in
            enumerate((
                tuple(instr[2] for instr in state)
                for state in parse(program)
            ))
        }

    def __str__(self) -> str:
        return self.flatten()

    def __repr__(self) -> str:
        return repr(self.arrows)

    def flatten(self, sep: str = ' ') -> str:
        return sep.join(
            dst
            for state in self.states
            for conn in self.arrows[state]
            for dst in conn
        )

    @property
    def dot(self) -> str:
        title = '\n'.join([
            '  labelloc="t";',
            f'  label="{self.program}";',
            '  fontname="courier"',
        ])

        header = title if len(self.program) < 50 else ''

        edges = '\n'.join([
            f'  {node} -> {target} [ color=" {COLORS[i]}" ];'
            for node, targets in self.arrows.items()
            for i, target in enumerate(targets)
            if target != UNDEFINED
        ])

        return f'digraph NAME {{\n{header}\n\n{edges}\n}}'

    @property
    def states(self) -> Tuple[str, ...]:
        return tuple(self.arrows)

    @property
    def colors(self) -> Tuple[int, ...]:
        return tuple(range(len(self.arrows['A'])))

    @property
    def exit_points(self) -> Dict[str, Set[str]]:
        return {
            state: set(connections) - { HALT, UNDEFINED }
            for state, connections in self.arrows.items()
        }

    @property
    def entry_points(self) -> Dict[str, Set[str]]:
        entries: Dict[str, Set[str]] = {
            state: set()
            for state in self.states
        }

        for state, exits in self.exit_points.items():
            for exit_point in exits:
                entries[exit_point].add(state)

        return entries

    @property
    def is_normal(self) -> bool:
        flat_graph = self.flatten('')

        if any(state not in flat_graph for state in self.states[1:]):
            return False

        return (
            positions := tuple(
                flat_graph.find(state)
                for state in self.states[1:]
            )
        ) == tuple(sorted(positions))

    @property
    def is_strongly_connected(self) -> bool:
        for state in self.states:
            reachable_from_x = set(self.arrows[state])

            for _ in self.states:
                reachable_from_x |= {
                    node
                    for connection in reachable_from_x
                    if connection in self.arrows
                    for node in self.arrows[connection]
                }

            if not reachable_from_x >= set(self.states):
                return False

        return True

    @property
    def reflexive_states(self) -> Set[str]:
        return {
            state
            for state, connections in self.arrows.items()
            if state in connections
        }

    @property
    def zero_reflexive_states(self) -> Set[str]:
        return {
            state
            for state, connections in self.arrows.items()
            if connections[0] == state
        }

    @property
    def is_irreflexive(self) -> bool:
        return not bool(self.reflexive_states)

    @property
    def is_zero_reflexive(self) -> bool:
        return bool(self.zero_reflexive_states)

    @property
    def entries_dispersed(self) -> bool:
        return all(
            len(entries) == len(self.colors)
            for entries in self.entry_points.values()
        )

    @property
    def exits_dispersed(self) -> bool:
        return all(
            len(exits) == len(self.colors)
            for exits in self.exit_points.values()
        )

    @property
    def is_dispersed(self) -> bool:
        return self.entries_dispersed and self.exits_dispersed

    @property
    def reduced(self) -> Dict[str, Set[str]]:
        graph = self.exit_points

        for _ in range(len(self.states) * len(self.colors)):
            purge_dead_ends(graph)
            cut_reflexive_arrows(graph)
            inline_single_exit(graph)
            inline_single_entry(graph)

        return {
            state: connections
            for state, connections in graph.items()
            if connections
        }

    @property
    def is_simple(self) -> bool:
        return not bool(self.reduced)


def purge_dead_ends(graph):
    to_cut = {
        state
        for state, connections in graph.items()
        if not connections
    }

    for state in to_cut:
        for connections in graph.values():
            connections.discard(state)

        del graph[state]


def inline_single_entry(graph):
    for _ in range(len(graph)):
        for dst in set(graph.keys()):
            entries = {
                src
                for src in graph
                if dst in graph[src]
            }

            if len(entries) != 1:
                continue

            entry_point = entries.pop()

            for out in graph[dst]:
                graph[entry_point].add(out)

            graph[entry_point].remove(dst)
            del graph[dst]

            break
        else:
            break


def cut_reflexive_arrows(graph):
    for state, connections in graph.items():
        if state in connections:
            connections.remove(state)


def inline_single_exit(graph):
    for state, connections in graph.items():
        if state in connections:
            connections.remove(state)
            break

        if not connections:
            continue

        if len(connections) > 1:
            continue

        exit_point = connections.pop()

        for con_rep in graph.values():
            if state in con_rep:
                con_rep.remove(state)
                con_rep.add(exit_point)
