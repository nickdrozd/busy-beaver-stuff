# pylint: disable = attribute-defined-outside-init
from queue import Empty, Queue as Q
from unittest import TestCase
from multiprocessing import Queue
from collections.abc import Iterator

from tm import Machine
from tm import Graph, BlockMacro, BacksymbolMacro
from tm.macro import MacroProg
from generate.tree  import run_tree_gen


def read_progs(name: str) -> set[str]:
    with open(f'test/data/{name}.prog') as holdouts:
        return set(
            prog.strip()
            for prog in holdouts.readlines()
        )

HOLDOUTS_23H = read_progs('holdouts_23h')
HOLDOUTS_32Q = read_progs('holdouts_32q')
HOLDOUTS_23Q = read_progs('holdouts_23q')

def macro_variations(prog: str) -> Iterator[str | MacroProg]:
    yield prog

    for block in range(2, 9):
        yield BacksymbolMacro(BlockMacro(prog, [block]), [1])

def run_for_none(prog: str, sim_lim: int) -> Iterator[bool]:
    yield from (
        Machine(macro).run(
            sim_lim = sim_lim,
            prover = 49,
        ).xlimit is None
        for macro in macro_variations(prog)
    )

def queue_to_set(queue: Q[str]) -> set[str]:
    out = set()

    while True:  # yuck -- pylint: disable = while-used
        try:
            prog = queue.get(timeout = .5)
        except Empty:
            break

        out.add(prog.replace('...', '1R_'))

    return out

class TestTree(TestCase):
    def assert_counts(self, expected: dict[int, set[str]]):
        for count, cat in expected.items():
            self.assertEqual(len(cat), count)
            self.assertTrue((
                all(Graph(prog).is_strongly_connected
                    for prog in cat)))

    def test_22(self):
        s22q: Q[str] = Queue()

        def capture(prog: str) -> None:
            if any(run_for_none(prog, 115)):
                return

            s22q.put(prog)

        run_tree_gen(
            states = 2,
            colors = 2,
            steps = 40,
            output = capture,
        )

        s22: set[str] = queue_to_set(s22q)

        self.assert_counts({0: s22})

    def test_32(self):
        h32q: Q[str] = Queue()
        q32q: Q[str] = Queue()

        def capture(prog: str) -> None:
            if any(run_for_none(prog, 225)):
                return

            if (dots := prog.count('...')) == 0:
                q32q.put(prog)
            elif dots == 1:
                h32q.put(prog)

        run_tree_gen(
            states = 3,
            colors = 2,
            steps = 126,
            blank = True,
            output = capture,
        )

        h32 = queue_to_set(h32q)
        q32 = queue_to_set(q32q)

        self.assert_counts({
             0: h32,
            40: q32,
        })

        self.assertEqual(
            q32,
            HOLDOUTS_32Q)

    def test_23(self):
        h23q: Q[str] = Queue()
        q23q: Q[str] = Queue()

        def capture(prog: str) -> None:
            if any(run_for_none(prog, 192)):
                return

            if (dots := prog.count('...')) == 0:
                q23q.put(prog)
            elif dots == 1:
                h23q.put(prog.replace('...', '1R_'))

        run_tree_gen(
            states = 2,
            colors = 3,
            steps = 223,
            output = capture,
        )

        h23 = queue_to_set(h23q)
        q23 = queue_to_set(q23q)

        self.assert_counts({
             11: h23,
            123: q23,
        })

        self.assertEqual(
            h23,
            HOLDOUTS_23H)

        self.assertEqual(
            q23,
            HOLDOUTS_23Q)

        self.assertIn(
            "1RB 2LA 1LA  2LA 2RB 0RA",  # wolfram
            q23)
