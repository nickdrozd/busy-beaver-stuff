from queue import Queue as Q
from unittest import TestCase
from multiprocessing import Queue
from collections.abc import Iterator

from tm import Machine, LinRecMachine
from tm import BlockMacro, BacksymbolMacro
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
        max_block: int,
        back_wrap: int,
) -> Iterator[str | MacroProg]:
    yield prog

    for block in range(2, 1 + max_block):
        yield BacksymbolMacro(
            BlockMacro(
                prog, [block]), [1])

    for wrap in range(1, 1 + back_wrap):
        yield BacksymbolMacro(prog, [1] * wrap)


def run_for_none(
        prog: str,
        sim_lim: int,
        depth: int,
        max_block: int = 1,
        back_wrap: int = 0,
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
        for macro in macro_variations(
                prog, max_block, back_wrap)
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

    def assert_complete(self, prog: str):
        self.assertNotIn('...', prog, f'"{prog}"')


class Fast(TestTree):
    def test_22(self):
        def capture(prog: str) -> None:
            self.assertTrue(
                any(run_for_none(prog, 45, 48, 2))
            )

        run_tree_gen(
            states = 2,
            colors = 2,
            steps = 20,
            output = capture,
        )

    def test_32(self):
        q32q: Q[str] = Queue()

        def capture(prog: str) -> None:
            if any(run_for_none(prog, 200, 200, 3, 1)):
                return

            self.assert_complete(prog)

            if any(run_for_none(prog, 2130, 100, 2)):
                return

            q32q.put(prog)

        run_tree_gen(
            states = 3,
            colors = 2,
            steps = 15,
            blank = True,
            output = capture,
        )

        q32 = queue_to_set(q32q)

        self.assert_counts({
            3: q32,
        })

        self.assert_progs(q32, 'holdouts_32q')

    def test_23(self):
        q23q: Q[str] = Queue()

        def capture(prog: str) -> None:
            if any(run_for_none(prog, 200, 200, 8, 1)):
                return

            if any(run_for_none(prog, 2350, 1400, 2, 1)):
                return

            self.assert_complete(prog)

            q23q.put(prog)

        run_tree_gen(
            states = 2,
            colors = 3,
            steps = 23,
            output = capture,
        )

        q23 = queue_to_set(q23q)

        self.assert_counts({
            9: q23,
        })

        self.assert_progs(q23, 'holdouts_23q')

        self.assertIn(
            "1RB 2LA 1LA  2LA 2RB 0RA",  # wolfram
            q23)


class Slow(TestTree):
    def test_42(self):  # no-coverage
        h42q: Q[str] = Queue()

        def capture(prog: str) -> None:
            if 'D' not in prog:
                return

            if any(run_for_none(prog, 400, 100, 10, 1)):
                return

            if any(run_for_none(prog, 2150, 500, 3, 2)):
                return

            h42q.put(prog)

        run_tree_gen(
            states = 4,
            colors = 2,
            steps = 35,
            halt = True,
            output = capture,
        )

        h42 = queue_to_set(h42q)

        self.assert_counts({
            70: h42,
        })

        self.assert_progs(h42, 'holdouts_42h')
