from queue import Queue as Q
from unittest import TestCase
from multiprocessing import Queue
from collections.abc import Iterator

from tm import Machine, LinRecMachine
from tm import Graph, BlockMacro, BacksymbolMacro
from tm.macro import MacroProg
from generate.tree  import run_tree_gen


def read_progs(name: str) -> set[str]:
    with open(f'test/data/{name}.prog') as holdouts:
        return set(
            prog.strip()
            for prog in holdouts.readlines()
        )


def macro_variations(
        prog: str,
        max_block: int | None,
) -> Iterator[str | MacroProg]:
    yield prog

    if max_block is not None:
        for block in range(2, max_block + 1):
            yield BacksymbolMacro(
                BlockMacro(
                    prog, [block]), [1])

    yield BacksymbolMacro(prog, [1])
    yield BacksymbolMacro(prog, [1, 1])

    yield BlockMacro(
        BacksymbolMacro(
            BlockMacro(
                prog, [2]), [1]), [2])


def run_for_none(
        prog: str,
        sim_lim: int,
        depth: int,
        max_block: int | None,
) -> Iterator[bool]:
    yield LinRecMachine(prog).run(
        step_lim = 50,
        check_rec = 0,
        skip = True,
    ).xlimit is None

    yield from (
        Machine(macro).run(
            sim_lim = sim_lim,
            prover = depth,
        ).xlimit is None
        for macro in macro_variations(prog, max_block)
    )


def queue_to_set(queue: Q[str]) -> set[str]:
    out = set()

    while not queue.empty():  # yuck -- pylint: disable = while-used
        out.add(queue.get())

    return out


class TestTree(TestCase):
    @staticmethod
    def assert_counts(expected: dict[int, set[str]]):
        if (failed := {
                count: len(cat)
                for count, cat in expected.items()
                if len(cat) != count
        }):
            raise AssertionError(failed)

    def assert_progs(self, progs: set[str], progfile: str):
        self.assertEqual(
            progs,
            read_progs(progfile))

    def assert_connected(self, *prog_sets: set[str]):
        for progs in prog_sets:
            self.assertTrue((
                all(Graph(prog).is_strongly_connected
                    for prog in progs)))


class Fast(TestTree):
    def test_22(self):
        s22q: Q[str] = Queue()

        def capture(prog: str) -> None:
            if any(run_for_none(prog, 45, 48, None)):
                return

            s22q.put(prog)  # no-coverage

        run_tree_gen(
            states = 2,
            colors = 2,
            steps = 20,
            output = capture,
        )

        s22: set[str] = queue_to_set(s22q)

        self.assert_counts({0: s22})

    def test_32(self):
        h32q: Q[str] = Queue()
        q32q: Q[str] = Queue()

        def capture(prog: str) -> None:
            if any(run_for_none(prog, 200, 200, 3)):
                return

            (q32q if prog.count('...') == 0 else h32q).put(prog)

        run_tree_gen(
            states = 3,
            colors = 2,
            steps = 15,
            blank = True,
            output = capture,
        )

        h32 = queue_to_set(h32q)
        q32 = queue_to_set(q32q)

        q32 -= {
            prog for prog in q32
            if any(run_for_none(prog, 2130, 100, 2))
        }

        self.assert_counts({
            0: h32,
            3: q32,
        })

        self.assert_connected(q32)

        self.assert_progs(q32, 'holdouts_32q')

    def test_23(self):
        h23q: Q[str] = Queue()
        q23q: Q[str] = Queue()

        def capture(prog: str) -> None:
            if any(run_for_none(prog, 200, 200, 8)):
                return

            (q23q if prog.count('...') == 0 else h23q).put(prog)

        run_tree_gen(
            states = 2,
            colors = 3,
            steps = 23,
            output = capture,
        )

        h23 = queue_to_set(h23q)
        q23 = queue_to_set(q23q)

        h23 -= {
            prog for prog in h23
            if any(run_for_none(prog, 1200, 300, 2))
        }

        q23 -= {
            prog for prog in q23
            if any(run_for_none(prog, 2350, 1400, 2))
        }

        self.assert_counts({
            0: h23,
            9: q23,
        })

        self.assert_connected(h23, q23)

        self.assert_progs(q23, 'holdouts_23q')

        self.assertIn(
            "1RB 2LA 1LA  2LA 2RB 0RA",  # wolfram
            q23)


class Slow(TestTree):
    def test_42(self):  # no-coverage
        hc42q: Q[str] = Queue()
        hd42q: Q[str] = Queue()

        def capture(prog: str) -> None:
            if any(run_for_none(prog, 200, 63, 8)):
                return

            if 'D' not in prog:
                return

            # pylint: disable = line-too-long
            (hc42q if Graph(prog).is_strongly_connected else hd42q).put(prog)

        run_tree_gen(
            states = 4,
            colors = 2,
            steps = 35,
            halt = True,
            output = capture,
        )

        hd42 = queue_to_set(hd42q)
        hc42 = queue_to_set(hc42q)

        hd42 -= {
            prog for prog in hd42
            if any(run_for_none(prog, 1700, 300, 3))
        }

        hc42 -= {
            prog for prog in hc42
            if any(run_for_none(prog, 2000, 500, 3))
        }

        self.assert_counts({
            38: hd42,
            76: hc42,
        })

        self.assert_connected(hc42)

        self.assert_progs(hd42, 'holdouts_42hd')
        self.assert_progs(hc42, 'holdouts_42hc')
