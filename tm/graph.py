from functools import cached_property

from tm.parse import parse, st_str
from tm.instrs import Color, State

ConGraph = dict[State, set[State]]

class Graph:
    arrows: dict[State, tuple[State | None, ...]]

    def __init__(self, program: str):
        self.arrows  = dict(
            enumerate(
                tuple(
                    instr[2] if instr is not None else None
                    for instr in instrs
                )
                for instrs in parse(program)
            )
        )

    def __str__(self) -> str:
        return self.flatten()

    def __repr__(self) -> str:
        return repr({
            st_str(state): tuple(map(st_str, conns))
            for state, conns in self.arrows.items()
        })

    def flatten(self, sep: str = ' ') -> str:
        return sep.join(
            st_str(dst)
            for conn in self.arrows.values()
            for dst in conn
        )

    @cached_property
    def states(self) -> tuple[State, ...]:
        return tuple(self.arrows)

    @cached_property
    def colors(self) -> tuple[Color, ...]:
        return tuple(range(len(self.arrows[0])))

    @cached_property
    def exit_points(self) -> ConGraph:
        return {
            state: set(
                conn
                for conn in connections
                if conn is not None and conn != -1
            )
            for state, connections in self.arrows.items()
        }

    @cached_property
    def entry_points(self) -> ConGraph:
        entries: ConGraph = {
            state: set()
            for state in self.states
        }

        for state, exits in self.exit_points.items():
            for exit_point in exits:
                entries[exit_point].add(state)

        return entries

    @cached_property
    def is_normal(self) -> bool:
        flat_graph = self.flatten('')

        if any(st_str(state) not in flat_graph
               for state in self.states[1:]):
            return False

        return (
            positions := tuple(
                flat_graph.find(st_str(state))
                for state in self.states[1:]
            )
        ) == tuple(sorted(positions))

    @cached_property
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

    @cached_property
    def reflexive_states(self) -> set[State]:
        return {
            state
            for state, connections in self.arrows.items()
            if state in connections
        }

    @cached_property
    def zero_reflexive_states(self) -> set[State]:
        return {
            state
            for state, connections in self.arrows.items()
            if connections[0] == state
        }

    @cached_property
    def is_irreflexive(self) -> bool:
        return not bool(self.reflexive_states)

    @cached_property
    def is_zero_reflexive(self) -> bool:
        return bool(self.zero_reflexive_states)

    @cached_property
    def entries_dispersed(self) -> bool:
        return all(
            len(entries) == len(self.colors)
            for entries in self.entry_points.values()
        )

    @cached_property
    def exits_dispersed(self) -> bool:
        return all(
            len(exits) == len(self.colors)
            for exits in self.exit_points.values()
        )

    @cached_property
    def is_dispersed(self) -> bool:
        return self.entries_dispersed and self.exits_dispersed

    @cached_property
    def reduced(self) -> ConGraph:
        graph = self.exit_points

        for _ in range(len(self.states) * len(self.colors)):
            if not graph:
                break

            cut_reflexive_arrows(graph)
            inline_single_exit(graph)
            inline_single_entry(graph)

        return {
            state: connections
            for state, connections in graph.items()
            if connections
        }

    @cached_property
    def is_simple(self) -> bool:
        return not bool(self.reduced)


def purge_dead_ends(graph: ConGraph) -> None:
    to_cut = {
        state
        for state, connections in graph.items()
        if not connections
    }

    for state in to_cut:
        for connections in graph.values():
            connections.discard(state)

        del graph[state]


def inline_single_entry(graph: ConGraph) -> None:
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

    purge_dead_ends(graph)


def cut_reflexive_arrows(graph: ConGraph) -> None:
    for state, connections in graph.items():
        if state in connections:
            connections.remove(state)


def inline_single_exit(graph: ConGraph) -> None:
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

    purge_dead_ends(graph)
