# pylint: disable = attribute-defined-outside-init

from unittest import TestCase

from generate.graph import Graph

A, B, C, D, E = "A", "B", "C", "D", "E"


GRAPHS = {
    # 2 2
    "1RB 1LB 1LA 1RH": (
        "B B A H",
        1, 1, 1,
        {A: {B}, B: {A}},
        {A: {B}, B: {A}},
    ),
    "1RB 1LB 1LB 1LA": (
        "B B B A",
        1, 1, 0,
        {A: {B}, B: {A, B}},
        {A: {B}, B: {A, B}},
    ),

    # 3 2
    "1RB 1RH 1LB 0RC 1LC 1LA": (
        "B H B C C A",
        1, 1, 0,
        {A: {C}, B: {A, B}, C: {B, C}},
        {A: {B}, B: {B, C}, C: {A, C}},
    ),
    "1RC 1RH 1LB 0RC 1LB 1LA": (
        "C H B C B A",
        0, 1, 0,
        {A: {C}, B: {B, C}, C: {A, B}},
        {A: {C}, B: {B, C}, C: {A, B}},
    ),
    "1RB 0LB 1LA 0RC 1LC 1LA": (
        "B B A C C A",
        1, 1, 0,
        {A: {B, C}, B: {A}, C: {B, C}},
        {A: {B}, B: {A, C}, C: {A, C}},
    ),
    "1RB 0LA 1LB 0RC 1LC 1LB": (
        "B A B C C B",
        1, 0, 0,
        {A: {A}, B: {A, B, C}, C: {B, C}},
        {A: {A, B}, B: {B, C}, C: {B, C}},
    ),

    # 2 3
    "1RB 2LB 1RH 2LA 2RB 1LB": (
        "B B H A B B",
        1, 1, 0,
        {A: {B}, B: {A, B}},
        {A: {B}, B: {A, B}},
    ),
    "1RB 2LB 1LA 2LB 2RA 0RA": (
        "B B A B A A",
        1, 1, 0,
        {A: {A, B}, B: {A, B}},
        {A: {A, B}, B: {A, B}},
    ),
    "1RB 2LB 1LA 2LB 2RB 0RB": (
        "B B A B B B",
        1, 0, 0,
        {A: {A}, B: {A, B}},
        {A: {A, B}, B: {B}},
    ),

    # 4 2
    "1RB 1LB 1LA 0LC 1RH 1LD 1RD 0RA": (
        "B B A C H D D A",
        1, 1, 0,
        {A: {B, D}, B: {A}, C: {B}, D: {C, D}},
        {A: {B}, B: {A, C}, C: {D}, D: {A, D}},
    ),
    "1RC 1LB 1LA 0LC 1RH 1LD 1RC 0RA": (
        "C B A C H D C A",
        0, 1, 1,
        {A: {B, D}, B: {A}, C: {A, B, D}, D: {C}},
        {A: {B, C}, B: {A, C}, C: {D}, D: {A, C}},
    ),
    "1RB 1LB 1LA 0LB 1RH 1LC 1RD 0RA": (
        "B B A B H C D A",
        1, 0, 0,
        {A: {B, D}, B: {A, B}, C: {C}, D: {D}},
        {A: {B}, B: {A, B}, C: {C}, D: {A, D}},
    ),
    "1RC 1LB 1LA 0LC 1RH 1LD 1RD 0RD": (
        "C B A C H D D D",
        0, 0, 0,
        {A: {B}, B: {A}, C: {A, B}, D: {C, D}},
        {A: {B, C}, B: {A, C}, C: {D}, D: {D}},
    ),
    "1RB 0LC 1LD 0LA 1RC 1RD 1LA 0LD": (
        "B C D A C D A D",
        1, 1, 0,
        {A: {B, D}, B: {A}, C: {A, C}, D: {B, C, D}},
        {A: {B, C}, B: {A, D}, C: {C, D}, D: {A, D}},
    ),
    "1RB 0LC 1LC 0LA 1RC 1RB 1LA 0LD": (
        "B C C A C B A D",
        1, 0, 0,
        {A: {B, D}, B: {A, C}, C: {A, B, C}, D: {D}},
        {A: {B, C}, B: {A, C}, C: {B, C}, D: {A, D}},
    ),
    "1RC 0LB 1LD 0LA 1RC 1RD 1LA 0LD": (
        "C B D A C D A D",
        0, 1, 0,
        {A: {B, D}, B: {A}, C: {A, C}, D: {B, C, D}},
        {A: {B, C}, B: {A, D}, C: {C, D}, D: {A, D}},
    ),
    "1RC 0LC 1LD 0LA 1RC 1RD 1LA 0LD": (
        "C C D A C D A D",
        0, 0, 0,
        {A: {B, D}, B: set(), C: {A, C}, D: {B, C, D}},
        {A: {C}, B: {A, D}, C: {C, D}, D: {A, D}},
    ),

    # 2 4
    "1RB 2LA 1RA 1RA 1LB 1LA 3RB 1RH": (
        "B A A A B A B H",
        1, 1, 0,
        {A: {A, B}, B: {A, B}},
        {A: {A, B}, B: {A, B}},
    ),
    "1RA 2LA 1RA 1RA 1LB 1LA 3RB 1RH": (
        "A A A A B A B H",
        1, 0, 0,
        {A: {A, B}, B: {B}},
        {A: {A}, B: {A, B}},
    ),

    # 3 3
    "1RB 2LB 1LC 1LA 2RB 1RB 1RH 2LA 0LC": (
        "B B C A B B H A C",
        1, 1, 0,
        {A: {B, C}, B: {A, B}, C: {A, C}},
        {A: {B, C}, B: {A, B}, C: {A, C}},
    ),
    "1RB 2LB 1LA 1LA 2RB 1RB 1RH 2LA 0LC": (
        "B B A A B B H A C",
        1, 0, 0,
        {A: {A, B, C}, B: {A, B}, C: {C}},
        {A: {A, B}, B: {A, B}, C: {A, C}},
    ),
    "1RC 2LB 1LC 1LA 2RB 1RB 1RH 2LA 0LC": (
        "C B C A B B H A C",
        0, 1, 0,
        {A: {B, C}, B: {A, B}, C: {A, C}},
        {A: {B, C}, B: {A, B}, C: {A, C}},
    ),
    "1RB 1LC 1RH 1LA 1LC 2RB 1RB 2LC 1RC": (
        "B C H A C B B C C",
        1, 1, 0,
        {A: {B}, B: {A, B, C}, C: {A, B, C}},
        {A: {B, C}, B: {A, B, C}, C: {B, C}},
    ),

    # 5 2
    "1RB 1LC 1RC 1RB 1RD 0LE 1LA 1LD 1RH 0LA": (
        "B C C B D E A D H A",
        1, 1, 0,
        {A: {D, E}, B: {A, B}, C: {A, B}, D: {C, D}, E: {C}},
        {A: {B, C}, B: {B, C}, C: {D, E}, D: {A, D}, E: {A}},
    ),
    "1RB 1LC 1RC 1RB 0LE 1RD 1LA 1LD 1RH 0LA": (
        "B C C B E D A D H A",
        0, 1, 0,
        {A: {D, E}, B: {A, B}, C: {A, B}, D: {C, D}, E: {C}},
        {A: {B, C}, B: {B, C}, C: {E, D}, D: {A, D}, E: {A}},
    ),
    "1RB 1LC 1RC 1RB 1RD 0LC 1LA 1LD 1RH 0LE": (
        "B C C B D C A D H E",
        1, 0, 0,
        {A: {D}, B: {A, B}, C: {A, B, C}, D: {C, D}, E: {E}},
        {A: {B, C}, B: {B, C}, C: {C, D}, D: {A, D}, E: {E}},
    ),
    "1RB 1LC 1LC 1RA 1LB 0LD 1LA 0RE 1RD 1RE": (
        "B C C A B D A E D E",
        1, 1, 0,
        {A: {B, D}, B: {A, C}, C: {A, B}, D: {C, E}, E: {D, E}},
        {A: {B, C}, B: {A, C}, C: {B, D}, D: {A, E}, E: {D, E}},
    ),
}


class TestGraph(TestCase):
    def assert_flat(self, flat):
        self.assertEqual(
            flat,
            self.graph.flatten())

    def assert_normal(self, norm):
        (self.assertTrue
         if self.graph.is_normal else
         self.assertFalse)(
             bool(norm))

    def assert_connected(self, conn):
        (self.assertTrue
         if self.graph.is_strongly_connected else
         self.assertFalse)(
             bool(conn))

    def assert_irreflexive(self, conn):
        (self.assertTrue
         if self.graph.is_irreflexive else
         self.assertFalse)(
             bool(conn))

    def assert_entry_points(self, entries):
        self.assertEqual(
            entries,
            self.graph.entry_points)

    def assert_exit_points(self, exits):
        self.assertEqual(
            exits,
            self.graph.exit_points)

    def test_graph(self):
        for prog, (flat, norm, conn, irr, entries, exits) in GRAPHS.items():
            self.graph = Graph(prog)

            self.assert_flat(flat)
            self.assert_normal(norm)
            self.assert_connected(conn)
            self.assert_irreflexive(irr)
            self.assert_entry_points(entries)
            self.assert_exit_points(exits)
