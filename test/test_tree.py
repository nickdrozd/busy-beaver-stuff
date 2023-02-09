from queue import Queue as Q
from unittest import TestCase
from multiprocessing import Queue, Manager

from tm.program import Program
from tm.utils import run_variations
from generate.tree import run_tree_gen


def read_progs(name: str) -> set[str]:
    with open(f'test/data/{name}.prog') as holdouts:
        return set(
            prog.strip()
            for prog in holdouts.readlines()
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

    def assert_records(
            self,
            actual,
            expected: dict[str, tuple[int, str]],
    ):
        for cat, res in expected.items():
            self.assertEqual(
                actual[cat], res)

    def assert_cant_terminate(self, progs: set[str]):
        for prog in map(Program, progs):
            self.assertTrue(
                prog.cant_halt
                and prog.cant_blank
                and prog.cant_spin_out)

    def assert_simple_and_connected(self, progs: set[str]):
        for prog in map(Program, progs):
            self.assertTrue(
                prog.graph.is_simple
                and prog.graph.is_strongly_connected)


class Fast(TestTree):
    def test_22(self):
        def capture(prog: str) -> None:
            for machine in run_variations(prog, 190, 2):
                if machine.xlimit is not None:
                    continue

                if ((res := machine.spnout)
                        and res > results['spnout'][0]):
                    results['spnout'] = res, prog

                if ((blanks := machine.blanks)
                        and (res := min(blanks.values()))
                        and  res > results['blanks'][0]):
                    results['blanks'] = res, prog

                return

            q22q.put(prog)  # no-coverage

        q22q: Q[str] = Queue()

        results = Manager().dict(
            blanks = (0, ""),
            spnout = (0, ""),
        )

        run_tree_gen(
            states = 2,
            colors = 2,
            steps = 20,
            output = capture,
        )

        self.assertFalse(
            queue_to_set(q22q))

        self.assert_records(
            results,
            {
                'blanks': (8, "1RB 0RA  1LB 1LA"),
                'spnout': (6, "1RB 1LB  0LB 1LA"),
            })

    def test_32(self):
        q32q: Q[str] = Queue()

        def capture(prog: str) -> None:
            for machine in run_variations(prog, 800, 3):
                if machine.xlimit is None:
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
            for machine in run_variations(prog, 400, 8):
                if machine.xlimit is None:
                    return

            for machine in run_variations(prog, 9_600, 2):
                if machine.xlimit is None:
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

        self.assert_simple_and_connected(q23)


class Slow(TestTree):
    def test_42(self):  # no-coverage
        h42q: Q[str] = Queue()

        def capture(prog: str) -> None:
            if 'D' not in prog:
                return

            for machine in run_variations(prog, 1000, 10):
                if machine.xlimit is None:
                    return

            for machine in run_variations(prog, 10_275):
                if machine.xlimit is None:
                    return

            for machine in run_variations(prog, 6_000, 2):
                if machine.xlimit is None:
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
            66,
            queue_to_set(h42q),
            'holdouts_42h')
