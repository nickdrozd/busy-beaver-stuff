from unittest import TestCase

from test.prog_data import GRAPHS, KERNEL, MODULAR, SPAGHETTI
from test.utils import read_progs
from tm.rust_stuff import is_connected
from tm.show import show_state
from tools.graph import Graph


class TestGraph(TestCase):
    graph: Graph

    def assert_flat(self, flat: str):
        self.assertEqual(
            flat,
            str(self.graph))

    def assert_normal(self, norm: int):
        (self.assertTrue
         if self.graph.is_normal else
         self.assertFalse)(
             bool(norm))

    def assert_connected(self, conn: int):
        (self.assertTrue
         if self.graph.is_connected else
         self.assertFalse)(
             bool(conn))

    def assert_irreflexive(self, irr: int):
        (self.assertTrue
         if self.graph.is_irreflexive else
         self.assertFalse)(
             bool(irr))

    def assert_zero_reflexive(self, zrefl: int):
        (self.assertTrue
         if self.graph.is_zero_reflexive else
         self.assertFalse)(
             bool(zrefl))

    def assert_entry_points(self, entries: dict[str, set[str]]):
        self.assertEqual(
            entries,
            {
                show_state(state): set(map(show_state, entries))
                for state, entries in self.graph.entry_points.items()
            },
        )

    def assert_exit_points(self, exits: dict[str, set[str]]):
        self.assertEqual(
            exits,
            {
                show_state(state): set(map(show_state, entries))
                for state, entries in self.graph.exit_points.items()
            },
        )

    def test_graph(self):
        for prog, (flat, norm, conn, irr, zrefl, entries, exits) in GRAPHS.items():
            self.graph = Graph(prog)

            _ =  str(self.graph)
            _ = repr(self.graph)

            self.assert_flat(flat)
            self.assert_normal(norm)
            self.assert_connected(conn)
            self.assert_irreflexive(irr)
            self.assert_zero_reflexive(zrefl)
            self.assert_entry_points(entries)
            self.assert_exit_points(exits)

            self.assertTrue(
                self.graph.zero_reflexive_states
                <= self.graph.reflexive_states
            )

            if len(self.graph.states) == 2:
                self.assertTrue(
                    self.graph.is_simple)

    def test_spaghetti(self):
        for prog in SPAGHETTI:
            graph = Graph(prog)

            self.assertEqual(
                len(graph.reduced),
                len(graph.states),
                prog)

            self.assertTrue(
                graph.is_dispersed
                    or '.' in prog
                    or len(graph.colors) >= 3,
                prog)

    def test_kernel(self):
        for prog, kernel in KERNEL.items():
            graph = Graph(prog)

            self.assertEqual(
                len(graph.reduced),
                kernel,
                prog)

            self.assertFalse(
                graph.is_dispersed and graph.is_irreflexive,
                prog)

    def test_modular(self):
        for prog in MODULAR:
            graph = Graph(prog)

            self.assertFalse(
                graph.is_connected)

            self.assertTrue(
                graph.is_simple)

    def test_connected(self):
        for prog in read_progs('graph/connected'):
            self.assertTrue(
                Graph(prog).is_connected)

            self.assertTrue(
                is_connected(prog))

    def test_unconnected(self):
        for prog in read_progs('graph/unconnected'):
            self.assertFalse(
                Graph(prog).is_connected)

            self.assertFalse(
                is_connected(prog))
