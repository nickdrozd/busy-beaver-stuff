from functools import cached_property

from tm.instrs import Color, State

from tm.rust_stuff import parse, st_str, reduce_graph

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
            st_str(dst)
            for conn in self.arrows.values()
            for dst in conn
        )

    def __repr__(self) -> str:
        return repr({
            st_str(state): tuple(map(st_str, conns))
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
        flat_graph = str(self)

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
    def is_simple(self) -> bool:
        return not bool(self.reduced)

    @cached_property
    def reduced(self) -> ConGraph:
        return reduce_graph(
            self.exit_points,
            len(self.states) * len(self.colors),
        )
