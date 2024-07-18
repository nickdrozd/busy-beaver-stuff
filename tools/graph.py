from __future__ import annotations

from typing import TYPE_CHECKING
from functools import cached_property

from tm.show import show_state
from tools import parse

if TYPE_CHECKING:
    from tm.parse import Color, State

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
        return ''.join(
            show_state(dst)
            for conn in self.arrows.values()
            for dst in conn
        )

    def __repr__(self) -> str:
        return repr({
            show_state(state): tuple(map(show_state, conns))
            for state, conns in self.arrows.items()
        })

    @cached_property
    def states(self) -> tuple[State, ...]:
        return tuple(self.arrows)

    @cached_property
    def colors(self) -> tuple[Color, ...]:
        return tuple(range(len(self.arrows[0])))

    @property
    def exit_points(self) -> ConGraph:
        return {
            state: set(
                conn
                for conn in connections
                if conn is not None
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
        flat_graph = str(self)

        if any(show_state(state) not in flat_graph
               for state in self.states[1:]):
            return False

        return (
            positions := tuple(
                flat_graph.find(show_state(state))
                for state in self.states[1:]
            )
        ) == tuple(sorted(positions))

    @cached_property
    def is_connected(self) -> bool:
        all_states = set(self.states)

        exitpoints = {
            state: {
                conn
                for conn in conns
                if conn is not None and conn != state
            }
            for state, conns in self.arrows.items()
        }

        if any(not conns for conns in exitpoints.values()):
            return False

        reached = set()

        todo = exitpoints[0].copy()

        for _ in self.states:  # no-branch
            try:
                state = todo.pop()
            except KeyError:
                break

            reached.add(state)

            if reached == all_states:
                return True

            todo |= exitpoints[state] - reached

        return False

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
    def is_simple(self) -> bool:
        return not bool(self.reduced)

    @cached_property
    def reduced(self) -> ConGraph:
        return reduce_graph(
            self.exit_points,
            len(self.states) * len(self.colors),
        )


def reduce_graph(graph: ConGraph, passes: int) -> ConGraph:
    for _ in range(passes):
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
