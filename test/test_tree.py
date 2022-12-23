from queue import Queue as Q
from unittest import TestCase
from multiprocessing import Queue
from collections.abc import Iterator

from tm import Machine, LinRecMachine
from tm import BlockMacro, BacksymbolMacro, Program
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
    def assert_progs(
            self,
            count: int,
            progs: set[str],
            progfile: str,
    ):
        self.assertEqual(
            progs,
            read_progs(progfile))

        self.assertEqual(
            count,
            len(progs))

    def assert_cant_terminate(self, progs: set[str]):
        for prog in map(Program, progs):
            self.assertTrue(
                prog.cant_halt
                and prog.cant_blank
                and prog.cant_spin_out)


class Fast(TestTree):
    def test_22(self):
        q22q: Q[str] = Queue()

        def capture(prog: str) -> None:
            if not any(run_for_none(prog, 45, 48, 2)):  # no-coverage
                q22q.put(prog)

        run_tree_gen(
            states = 2,
            colors = 2,
            steps = 20,
            output = capture,
        )

        self.assertFalse(queue_to_set(q22q))

    def test_32(self):
        q32q: Q[str] = Queue()

        def capture(prog: str) -> None:
            if any(run_for_none(prog, 200, 200, 3, 1)):
                return

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

        self.assert_progs(
            3,
            q32 := queue_to_set(q32q),
            'holdouts_32q')

        self.assert_cant_terminate(q32)

    def test_23(self):
        q23q: Q[str] = Queue()

        def capture(prog: str) -> None:
            if any(run_for_none(prog, 200, 200, 8, 1)):
                return

            if any(run_for_none(prog, 2350, 1400, 2, 1)):
                return

            q23q.put(prog)

        run_tree_gen(
            states = 2,
            colors = 3,
            steps = 23,
            output = capture,
        )

        self.assert_progs(
            9,
            (q23 := queue_to_set(q23q)),
            'holdouts_23q')

        self.assertIn(
            "1RB 2LA 1LA  2LA 2RB 0RA",  # wolfram
            q23)

        self.assert_cant_terminate(q23)


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

        self.assert_progs(
            70,
            queue_to_set(h42q),
            'holdouts_42h')
