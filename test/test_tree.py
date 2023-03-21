from queue import Queue as Q
from typing import Any
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
    queue: Q[str]

    progs: set[str]

    results: Any  # type: ignore[misc]

    def setUp(self):
        self.queue = Queue()

        self.results = Manager().dict(
            blanks = (0, ""),
            halted = (0, ""),
            spnout = (0, ""),
        )

    def assert_progs(self, count: int, progfile: str):
        self.progs = queue_to_set(self.queue)

        self.assertEqual(
            self.progs,
            read_progs(progfile))

        self.assertEqual(
            count,
            len(self.progs))

    def assert_records(self, expected: dict[str, tuple[int, str]]):
        for cat, res in expected.items():
            self.assertEqual(
                self.results[cat], res)

    def assert_cant_terminate(self) -> None:
        for prog in map(Program, self.progs):
            self.assertTrue(
                prog.cant_halt
                and prog.cant_blank
                and prog.cant_spin_out)

    def assert_simple_and_connected(self) -> None:
        for prog in map(Program, self.progs):
            self.assertTrue(
                prog.graph.is_simple
                and prog.graph.is_strongly_connected)

    def add_result(self, prog: str, machine) -> None:
        if ((blanks := machine.blanks)
                and (res := min(blanks.values()))
                and  res > self.results['blanks'][0]):
            self.results['blanks'] = res, prog

        if ((res := machine.spnout)
                and res > self.results['spnout'][0]):
            self.results['spnout'] = res, prog
            return

        if ((res := machine.undfnd)
                and ((step := res[0] + 1)
                       > self.results['halted'][0])):  # no-coverage
            self.results['halted'] = step, prog.replace('...', '1R_')
            return


class Fast(TestTree):
    def test_22(self):
        def capture(prog: str) -> None:
            for machine in run_variations(prog, 190):
                if machine.xlimit is not None:
                    continue

                self.add_result(prog, machine)
                return

            self.queue.put(prog)  # no-coverage

        run_tree_gen(
            states = 2,
            colors = 2,
            steps = 20,
            halt = False,
            output = capture,
        )

        self.assertFalse(
            queue_to_set(self.queue))

        self.assert_records({
            'blanks': (8, "1RB 0RA  1LB 1LA"),
            'spnout': (6, "1RB 1LB  0LB 1LA"),
        })

    def test_32(self):
        def capture(prog: str) -> None:
            for machine in run_variations(prog, 800):
                if machine.xlimit is not None:
                    continue

                self.add_result(prog, machine)
                return

            self.queue.put(prog)

        run_tree_gen(
            states = 3,
            colors = 2,
            steps = 15,
            halt = False,
            output = capture,
        )

        self.assert_progs(
            3,
            'holdouts_32q')

        self.assert_records({
            'blanks': (34, "1RB 1LB  1LA 1LC  1RC 0LC"),
            'spnout': (55, "1RB 0LB  1LA 0RC  1LC 1LA"),
        })

        self.assert_cant_terminate()

    def test_23(self):
        def capture(prog: str) -> None:
            for machine in run_variations(prog, 400):
                if machine.xlimit is not None:
                    continue

                self.add_result(prog, machine)
                return

            for machine in run_variations(prog, 9_600):
                if machine.xlimit is None:
                    return

            self.queue.put(prog)

        run_tree_gen(
            states = 2,
            colors = 3,
            steps = 23,
            halt = False,
            output = capture,
        )

        self.assert_progs(
            9,
            'holdouts_23q')

        self.assert_records({
            'blanks': (77, "1RB 2LA 0RB  1LA 0LB 1RA"),
            'spnout': (59, "1RB 2LB 1LA  2LB 2RA 0RA"),
        })

        self.assertIn(
            "1RB 2LA 1LA  2LA 2RB 0RA",  # wolfram
            self.progs)

        self.assert_simple_and_connected()


class Slow(TestTree):
    def test_42(self):  # no-coverage
        def capture(prog: str) -> None:
            if 'D' not in prog:
                return

            for machine in run_variations(prog, 1000):
                if machine.xlimit is None:
                    self.add_result(prog, machine)
                    return

            for machine in run_variations(
                    prog, 7_000, block_steps = 6_000):
                if machine.xlimit is None:
                    return

            self.queue.put(prog)

        run_tree_gen(
            states = 4,
            colors = 2,
            steps = 35,
            halt = True,
            output = capture,
        )

        self.assert_progs(
            64,
            'holdouts_42h')

        self.assert_records({
            'blanks': (169, "1RB ...  0RC 0LA  1LC 1LD  0RB 0RD"),
            'spnout': (171, "1RB ...  0RC 0LA  1LC 1LD  0RB 0RD"),
            'halted': (159, "1RB 0RD  1LC 0RA  1LA 1LB  1R_ 0RC"),
        })
