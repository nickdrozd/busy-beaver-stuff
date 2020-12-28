# pylint: disable = attribute-defined-outside-init

from unittest import TestCase

from graph.graph import Graph


GRAPHS = {
    # 2 2
    "1RB 1LB 1LA 1RH": ("B B A H", 1, 1),
    "1RB 1LB 1LB 1LA": ("B B B A", 1, 1),

    # 3 2
    "1RB 1RH 1LB 0RC 1LC 1LA": ("B H B C C A", 1, 1),
    "1RC 1RH 1LB 0RC 1LB 1LA": ("C H B C B A", 0, 1),
    "1RB 0LB 1LA 0RC 1LC 1LA": ("B B A C C A", 1, 1),
    "1RB 0LA 1LB 0RC 1LC 1LB": ("B A B C C B", 1, 0),

    # 2 3
    "1RB 2LB 1RH 2LA 2RB 1LB": ("B B H A B B", 1, 1),
    "1RB 2LB 1LA 2LB 2RA 0RA": ("B B A B A A", 1, 1),
    "1RB 2LB 1LA 2LB 2RB 0RB": ("B B A B B B", 1, 0),

    # 4 2
    "1RB 1LB 1LA 0LC 1RH 1LD 1RD 0RA": ("B B A C H D D A", 1, 1),
    "1RC 1LB 1LA 0LC 1RH 1LD 1RD 0RA": ("C B A C H D D A", 0, 1),
    "1RB 1LB 1LA 0LB 1RH 1LC 1RD 0RA": ("B B A B H C D A", 1, 0),
    "1RC 1LB 1LA 0LC 1RH 1LD 1RD 0RD": ("C B A C H D D D", 0, 0),
    "1RB 0LC 1LD 0LA 1RC 1RD 1LA 0LD": ("B C D A C D A D", 1, 1),
    "1RB 0LC 1LC 0LA 1RC 1RB 1LA 0LD": ("B C C A C B A D", 1, 0),
    "1RC 0LB 1LD 0LA 1RC 1RD 1LA 0LD": ("C B D A C D A D", 0, 1),
    "1RC 0LC 1LD 0LA 1RC 1RD 1LA 0LD": ("C C D A C D A D", 0, 0),

    # 2 4
    "1RB 2LA 1RA 1RA 1LB 1LA 3RB 1RH": ("B A A A B A B H", 1, 1),
    "1RA 2LA 1RA 1RA 1LB 1LA 3RB 1RH": ("A A A A B A B H", 1, 0),

    # 3 3
    "1RB 2LB 1LC 1LA 2RB 1RB 1RH 2LA 0LC": ("B B C A B B H A C", 1, 1),
    "1RB 2LB 1LA 1LA 2RB 1RB 1RH 2LA 0LC": ("B B A A B B H A C", 1, 0),
    "1RC 2LB 1LC 1LA 2RB 1RB 1RH 2LA 0LC": ("C B C A B B H A C", 0, 1),
    "1RB 1LC 1RH 1LA 1LC 2RB 1RB 2LC 1RC": ("B C H A C B B C C", 1, 1),

    # 5 2
    "1RB 1LC 1RC 1RB 1RD 0LE 1LA 1LD 1RH 0LA": ("B C C B D E A D H A", 1, 1),
    "1RB 1LC 1RC 1RB 0LE 1RD 1LA 1LD 1RH 0LA": ("B C C B E D A D H A", 0, 1),
    "1RB 1LC 1RC 1RB 1RD 0LC 1LA 1LD 1RH 0LE": ("B C C B D C A D H E", 1, 0),
    "1RB 1LC 1LC 1RA 1LB 0LD 1LA 0RE 1RD 1RE": ("B C C A B D A E D E", 1, 1),
}


class TestGraph(TestCase):
    def assert_flat(self, flat):
        self.assertEqual(
            flat,
            self.graph.flatten())

    def assert_normal(self, norm):
        (self.assertTrue
         if self.graph.is_normal() else
         self.assertFalse)(
             bool(norm))

    def assert_connected(self, conn):
        (self.assertTrue
         if self.graph.is_strongly_connected() else
         self.assertFalse)(
             bool(conn))

    def test_graph(self):
        for prog, (flat, norm, conn) in GRAPHS.items():
            self.graph = Graph(prog)

            self.assert_flat(flat)
            self.assert_normal(norm)
            self.assert_connected(conn)
