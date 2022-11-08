from queue import Queue as Q
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


def macro_variations(prog: str) -> Iterator[str | MacroProg]:
    yield prog

    for block in range(2, 9):
        yield BacksymbolMacro(BlockMacro(prog, [block]), [1])

    yield BacksymbolMacro(prog, [1])


def run_for_none(prog: str, sim_lim: int, depth: int) -> Iterator[bool]:
    yield from (
        Machine(macro).run(
            sim_lim = sim_lim,
            prover = depth,
        ).xlimit is None
        for macro in macro_variations(prog)
    )


def queue_to_set(queue: Q[str]) -> set[str]:
    out = set()

    while not queue.empty():  # yuck -- pylint: disable = while-used
        out.add(queue.get().replace('...', '1R_'))

    return out


class TestTree(TestCase):
    def assert_counts(self, expected: dict[int, set[str]]):
        for count, cat in expected.items():
            self.assertEqual(len(cat), count)

    def assert_progs(self, progs: set[str], progfile: str):
        self.assertEqual(
            progs,
            read_progs(progfile))

    def assert_connected(self, *prog_sets: set[str]):
        for progs in prog_sets:
            self.assertTrue((
                all(Graph(prog).is_strongly_connected
                    for prog in progs)))

    def test_22(self):
        s22q: Q[str] = Queue()

        def capture(prog: str) -> None:
            if any(run_for_none(prog, 115, 24)):
                return

            s22q.put(prog)  # no-coverage

        run_tree_gen(
            states = 2,
            colors = 2,
            steps = 1,
            output = capture,
        )

        s22: set[str] = queue_to_set(s22q)

        self.assert_counts({0: s22})

    def test_32(self):
        h32q: Q[str] = Queue()
        q32q: Q[str] = Queue()

        def capture(prog: str) -> None:
            if any(run_for_none(prog, 189, 40)):
                return

            (q32q if not prog.count('...') else h32q).put(prog)

        run_tree_gen(
            states = 3,
            colors = 2,
            steps = 15,
            blank = True,
            output = capture,
        )

        h32 = queue_to_set(h32q)
        q32 = queue_to_set(q32q)

        self.assert_counts({
             0: h32,
            33: q32,
        })

        self.assert_connected(q32)

        self.assert_progs(q32, 'holdouts_32q')

    def test_23(self):
        h23q: Q[str] = Queue()
        q23q: Q[str] = Queue()

        def capture(prog: str) -> None:
            if any(run_for_none(prog, 212, 49)):
                return

            if prog.count('...') == 0:
                q23q.put(prog)
            else:
                h23q.put(prog.replace('...', '1R_'))

        run_tree_gen(
            states = 2,
            colors = 3,
            steps = 18,
            output = capture,
        )

        h23 = queue_to_set(h23q)
        q23 = queue_to_set(q23q)

        self.assert_counts({
             7: h23,
            80: q23,
        })

        self.assert_connected(h23, q23)

        self.assert_progs(h23, 'holdouts_23h')
        self.assert_progs(q23, 'holdouts_23q')

        self.assertIn(
            "1RB 2LA 1LA  2LA 2RB 0RA",  # wolfram
            q23)
