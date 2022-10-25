# pylint: disable = attribute-defined-outside-init

from unittest import TestCase

from analyze import Graph

A, B, C, D, E = "A", "B", "C", "D", "E"

# flat, norm, conn, irr, zrefl, entries, exits

GRAPHS: dict[
    str,
    tuple[
        str,
        int, int, int, int,
        dict[str, set[str]],
        dict[str, set[str]],
    ],
] = {
    # 2 2
    "1RB 1LB  1LA 1R_": (
        "B B A _",
        1, 1, 1, 0,
        {A: {B}, B: {A}},
        {A: {B}, B: {A}},
    ),
    "1RB 1LB  1LB 1LA": (
        "B B B A",
        1, 1, 0, 1,
        {A: {B}, B: {A, B}},
        {A: {B}, B: {A, B}},
    ),

    # 3 2
    "1RB 0LB  1LA 0RA  ... ...": (
        'B B A A . .',
        0, 0, 1, 0,
        {A: {B}, B: {A}, C: set()},
        {A: {B}, B: {A}, C: set()},
    ),
    "1RB 1LA  0LA 0RB  ... ...": (
        'B A A B . .',
        0, 0, 0, 0,
        {A : {A, B}, B: {A, B}, C: set()},
        {A : {A, B}, B: {A, B}, C: set()},
    ),
    "1RB ...  0LB 1RC  1LB 0RC": (
        'B . B C B C',
        1, 0, 0, 1,
        {A: set(), B: {A, B, C}, C: {B, C}},
        {A: {B}, B: {B, C}, C: {B, C}},
    ),
    "1RB 1R_  1LB 0RC  1LC 1LA": (
        "B _ B C C A",
        1, 1, 0, 1,
        {A: {C}, B: {A, B}, C: {B, C}},
        {A: {B}, B: {B, C}, C: {A, C}},
    ),
    "1RC 1R_  1LB 0RC  1LB 1LA": (
        "C _ B C B A",
        0, 1, 0, 1,
        {A: {C}, B: {B, C}, C: {A, B}},
        {A: {C}, B: {B, C}, C: {A, B}},
    ),
    "1RB 0LB  1LA 0RC  1LC 1LA": (
        "B B A C C A",
        1, 1, 0, 1,
        {A: {B, C}, B: {A}, C: {B, C}},
        {A: {B}, B: {A, C}, C: {A, C}},
    ),
    "1RB 0LA  1LB 0RC  1LC 1LB": (
        "B A B C C B",
        1, 0, 0, 1,
        {A: {A}, B: {A, B, C}, C: {B, C}},
        {A: {A, B}, B: {B, C}, C: {B, C}},
    ),

    # 2 3
    "1RB 2LB 1R_  2LA 2RB 1LB": (
        "B B _ A B B",
        1, 1, 0, 0,
        {A: {B}, B: {A, B}},
        {A: {B}, B: {A, B}},
    ),
    "1RB 2LB 1LA  2LB 2RA 0RA": (
        "B B A B A A",
        1, 1, 0, 1,
        {A: {A, B}, B: {A, B}},
        {A: {A, B}, B: {A, B}},
    ),
    "1RB 2LB 1LA  2LB 2RB 0RB": (
        "B B A B B B",
        1, 0, 0, 1,
        {A: {A}, B: {A, B}},
        {A: {A, B}, B: {B}},
    ),

    # 4 2
    "1RB 1LB  1LA 0LC  1R_ 1LD  1RD 0RA": (
        "B B A C _ D D A",
        1, 1, 0, 1,
        {A: {B, D}, B: {A}, C: {B}, D: {C, D}},
        {A: {B}, B: {A, C}, C: {D}, D: {A, D}},
    ),
    "1RC 1LB  1LA 0LC  1R_ 1LD  1RC 0RA": (
        "C B A C _ D C A",
        0, 1, 1, 0,
        {A: {B, D}, B: {A}, C: {A, B, D}, D: {C}},
        {A: {B, C}, B: {A, C}, C: {D}, D: {A, C}},
    ),
    "1RB 1LB  1LA 0LB  1R_ 1LC  1RD 0RA": (
        "B B A B _ C D A",
        1, 0, 0, 1,
        {A: {B, D}, B: {A, B}, C: {C}, D: {D}},
        {A: {B}, B: {A, B}, C: {C}, D: {A, D}},
    ),
    "1RC 1LB  1LA 0LC  1R_ 1LD  1RD 0RD": (
        "C B A C _ D D D",
        0, 0, 0, 1,
        {A: {B}, B: {A}, C: {A, B}, D: {C, D}},
        {A: {B, C}, B: {A, C}, C: {D}, D: {D}},
    ),
    "1RB 0LC  1LD 0LA  1RC 1RD  1LA 0LD": (
        "B C D A C D A D",
        1, 1, 0, 1,
        {A: {B, D}, B: {A}, C: {A, C}, D: {B, C, D}},
        {A: {B, C}, B: {A, D}, C: {C, D}, D: {A, D}},
    ),
    "1RB 0LC  1LC 0LA  1RC 1RB  1LA 0LD": (
        "B C C A C B A D",
        1, 0, 0, 1,
        {A: {B, D}, B: {A, C}, C: {A, B, C}, D: {D}},
        {A: {B, C}, B: {A, C}, C: {B, C}, D: {A, D}},
    ),
    "1RC 0LB  1LD 0LA  1RC 1RD  1LA 0LD": (
        "C B D A C D A D",
        0, 1, 0, 1,
        {A: {B, D}, B: {A}, C: {A, C}, D: {B, C, D}},
        {A: {B, C}, B: {A, D}, C: {C, D}, D: {A, D}},
    ),
    "1RC 0LC  1LD 0LA  1RC 1RD  1LA 0LD": (
        "C C D A C D A D",
        0, 0, 0, 1,
        {A: {B, D}, B: set(), C: {A, C}, D: {B, C, D}},
        {A: {C}, B: {A, D}, C: {C, D}, D: {A, D}},
    ),

    # 2 4
    "1RB 2LA 1RA 1RA  1LB 1LA 3RB 1R_": (
        "B A A A B A B _",
        1, 1, 0, 1,
        {A: {A, B}, B: {A, B}},
        {A: {A, B}, B: {A, B}},
    ),
    "1RA 2LA 1RA 1RA  1LB 1LA 3RB 1R_": (
        "A A A A B A B _",
        1, 0, 0, 1,
        {A: {A, B}, B: {B}},
        {A: {A}, B: {A, B}},
    ),

    # 3 3
    "1RB 2LB 1LC  1LA 2RB 1RB  1R_ 2LA 0LC": (
        "B B C A B B _ A C",
        1, 1, 0, 0,
        {A: {B, C}, B: {A, B}, C: {A, C}},
        {A: {B, C}, B: {A, B}, C: {A, C}},
    ),
    "1RB 2LB 1LA  1LA 2RB 1RB  1R_ 2LA 0LC": (
        "B B A A B B _ A C",
        1, 0, 0, 0,
        {A: {A, B, C}, B: {A, B}, C: {C}},
        {A: {A, B}, B: {A, B}, C: {A, C}},
    ),
    "1RC 2LB 1LC  1LA 2RB 1RB  1R_ 2LA 0LC": (
        "C B C A B B _ A C",
        0, 1, 0, 0,
        {A: {B, C}, B: {A, B}, C: {A, C}},
        {A: {B, C}, B: {A, B}, C: {A, C}},
    ),
    "1RB 1LC 1R_  1LA 1LC 2RB  1RB 2LC 1RC": (
        "B C _ A C B B C C",
        1, 1, 0, 0,
        {A: {B}, B: {A, B, C}, C: {A, B, C}},
        {A: {B, C}, B: {A, B, C}, C: {B, C}},
    ),

    # 5 2
    "1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  1R_ 0LA": (
        "B C C B D E A D _ A",
        1, 1, 0, 0,
        {A: {D, E}, B: {A, B}, C: {A, B}, D: {C, D}, E: {C}},
        {A: {B, C}, B: {B, C}, C: {D, E}, D: {A, D}, E: {A}},
    ),
    "1RB 1LC  1RC 1RB  0LE 1RD  1LA 1LD  1R_ 0LA": (
        "B C C B E D A D _ A",
        0, 1, 0, 0,
        {A: {D, E}, B: {A, B}, C: {A, B}, D: {C, D}, E: {C}},
        {A: {B, C}, B: {B, C}, C: {E, D}, D: {A, D}, E: {A}},
    ),
    "1RB 1LC  1RC 1RB  1RD 0LC  1LA 1LD  1R_ 0LE": (
        "B C C B D C A D _ E",
        1, 0, 0, 0,
        {A: {D}, B: {A, B}, C: {A, B, C}, D: {C, D}, E: {E}},
        {A: {B, C}, B: {B, C}, C: {C, D}, D: {A, D}, E: {E}},
    ),
    "1RB 1LC  1LC 1RA  1LB 0LD  1LA 0RE  1RD 1RE": (
        "B C C A B D A E D E",
        1, 1, 0, 0,
        {A: {B, D}, B: {A, C}, C: {A, B}, D: {C, E}, E: {D, E}},
        {A: {B, C}, B: {A, C}, C: {B, D}, D: {A, E}, E: {D, E}},
    ),
}

class TestGraph(TestCase):
    def assert_flat(self, flat: str):
        self.assertEqual(
            flat,
            self.graph.flatten())

    def assert_normal(self, norm: int):
        (self.assertTrue
         if self.graph.is_normal else
         self.assertFalse)(
             bool(norm),
             self.graph.program)

    def assert_connected(self, conn: int):
        (self.assertTrue
         if self.graph.is_strongly_connected else
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
            self.graph.entry_points)

    def assert_exit_points(self, exits: dict[str, set[str]]):
        self.assertEqual(
            exits,
            self.graph.exit_points)

    def test_graph(self):
        # pylint: disable = line-too-long
        for prog, (flat, norm, conn, irr, zrefl, entries, exits) in GRAPHS.items():
            self.graph = Graph(prog)

            print(self.graph)
            _ = repr(self.graph)

            self.assert_flat(flat)
            self.assert_normal(norm)
            self.assert_connected(conn)
            self.assert_irreflexive(irr)
            self.assert_zero_reflexive(zrefl)
            self.assert_entry_points(entries)
            self.assert_exit_points(exits)

            self.assertTrue((
                self.graph.zero_reflexive_states
                <= self.graph.reflexive_states
            ))
