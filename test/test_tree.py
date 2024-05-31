from __future__ import annotations

from unittest import TestCase, skipUnless
from multiprocessing import Queue, Manager
from typing import TYPE_CHECKING

from test.utils import read_progs, RUN_SLOW
from test.lin_rec import run_loose_linrec_machine
from test.prog_data import CANT_BLANK_FALSE_NEGATIVES

from tools.graph import Graph

from tm.tree import run_tree_gen
from tm.machine import Machine, quick_term_or_rec
from tm.reason import (
    cant_halt,
    cant_blank,
    cant_spin_out,
)


if TYPE_CHECKING:
    from collections.abc import Iterator

    from test.lin_rec import QuickMachineResult

    from tm.machine import Params

    BasicMachine = Machine | QuickMachineResult

    Q = Queue[str]


def queue_to_set(queue: Q) -> set[str]:
    out = set()

    while not queue.empty():  # yuck -- pylint: disable = while-used
        out.add(queue.get())

    return out


def run_variations(
        prog: str,
        sim_lim: int,
        params: Params,
        *,
        lin_rec: int | None = None,
        block_steps: int = 1_000,
) -> Iterator[BasicMachine]:
    if lin_rec is not None:
        yield run_loose_linrec_machine(prog, lin_rec)

    yield Machine(
        prog,
        opt_macro = block_steps,
        params = params,
    ).run(
        sim_lim = sim_lim,
    )

    yield Machine(
        prog,
        backsym = 1,
        params = params,
    ).run(
        sim_lim = sim_lim,
    )


class TestTree(TestCase):
    queue: Q

    progs: set[str]

    results: dict[str, tuple[int, str]]

    def setUp(self):
        self.queue = Queue()

        self.results = Manager().dict(  # type: ignore[assignment]
            blanks = (0, ""),
            halted = (0, ""),
            spnout = (0, ""),
            infrul = (0, ""),
        )

    def assert_progs(self, count: int, progfile: str):
        self.progs = queue_to_set(self.queue)

        self.assertEqual(
            self.progs,
            read_progs(progfile))

        self.assertEqual(
            count,
            len(self.progs))

    def assert_records(
            self,
            expected: dict[
                str,
                tuple[int, str | set[str]]],
    ):
        for cat, (exp_step, exp_prog) in expected.items():
            res_step, res_prog = self.results[cat]

            self.assertEqual(res_step, exp_step)

            self.assertTrue(
                res_prog == exp_prog
                    or res_prog in exp_prog)

    def assert_cant_terminate(self) -> None:
        for prog in self.progs:
            self.assertTrue(
                cant_halt(prog))

            if prog not in CANT_BLANK_FALSE_NEGATIVES:
                self.assertTrue(
                    cant_blank(prog))

            self.assertTrue(
                cant_spin_out(prog))

    def assert_simple_and_connected(self) -> None:
        for graph in map(Graph, self.progs):
            self.assertTrue(
                graph.is_simple
                and graph.is_strongly_connected)

    def add_result(self, prog: str, machine: BasicMachine) -> None:
        if ((blanks := machine.blanks)
                and (res := min(blanks.values()))
                and  res > self.results['blanks'][0]):
            self.results['blanks'] = res, prog

        if ((spnout := machine.spnout)
                and spnout > self.results['spnout'][0]):
            self.results['spnout'] = spnout, prog
            return

        if ((und := machine.undfnd)
                and ((step := und[0] + 1) > self.results['halted'][0])):
            self.results['halted'] = step, prog
            return

        if (machine.infrul
                and (cycles := machine.cycles)
                        > self.results['infrul'][0]):
            self.results['infrul'] = cycles, prog
            return


class Fast(TestTree):
    def test_22(self):
        params = 2, 2

        max_inf = 187

        def capture(prog: str) -> None:
            machine = Machine(
                prog,
                opt_macro = 20,
                params = params,
            ).run(sim_lim = 1 + max_inf)

            if machine.xlimit is None:
                self.add_result(prog, machine)
                return

            self.queue.put(prog)

        run_tree_gen(
            steps = 20,
            halt = False,
            params = params,
            output = capture,
        )

        self.assertFalse(
            queue_to_set(self.queue))

        try:
            self.assert_records({
                'blanks': (8, "1RB 0RA  1LB 1LA"),
                'spnout': (6, {
                    "1RB 1LB  0LB 1LA",
                    "1RB 1LB  1LB 1LA",
                    "1RB 0LB  0LB 1LA",
                }),
                'infrul': (max_inf, "1RB 1LA  0LA 0RB"),
            })
        except AssertionError:
            self.assert_records({
                'blanks': (7, '1RB 0RA  0LB 1LA'),
                'spnout': (6, {
                    "1RB 1LB  0LB 1LA",
                    "1RB 1LB  1LB 1LA",
                    "1RB 0LB  0LB 1LA",
                }),
                'infrul': (187, "1RB 1LA  0LA 0RB"),
            })

    def test_32(self):
        params = 3, 2

        max_inf = 4_927

        def capture(prog: str) -> None:
            machines = run_variations(
                prog, 1 + max_inf,
                lin_rec = 50,
                params = params,
            )

            for machine in machines:
                if machine.xlimit is None:
                    self.add_result(prog, machine)
                    return

            self.queue.put(prog)

        run_tree_gen(
            steps = 15,
            halt = False,
            params = params,
            output = capture,
        )

        self.assert_progs(
            3,
            'holdouts_32q')

        self.assert_records({
            'blanks': (34, "1RB 1LB  1LA 1LC  1RC 0LC"),
            'spnout': (55, "1RB 0LB  1LA 0RC  1LC 1LA"),
            'infrul': (max_inf, "1RB 1LA  0LB 1RC  1LA 0RB"),
        })

        self.assert_cant_terminate()
        self.assert_simple_and_connected()

    def test_23(self):
        params = 2, 3

        max_inf = 7_395

        def capture(prog: str) -> None:
            machines = run_variations(
                prog, 400,
                lin_rec = 50,
                params = params,
            )

            for machine in machines:
                if machine.xlimit is None:
                    self.add_result(prog, machine)
                    return

            machines = run_variations(
                prog, 1 + max_inf,
                params = params,
            )

            for machine in machines:
                if machine.xlimit is None:
                    self.add_result(prog, machine)
                    return

            self.queue.put(prog)

        run_tree_gen(
            steps = 23,
            halt = False,
            params = params,
            output = capture,
        )

        self.assert_progs(
            9,
            'holdouts_23q')

        self.assert_records({
            'blanks': (77, "1RB 2LA 0RB  1LA 0LB 1RA"),
            'spnout': (59, "1RB 2LB 1LA  2LB 2RA 0RA"),
            'infrul': (max_inf, "1RB 2LA 0RB  1LB 1LA 1RA"),
        })

        self.assertIn(
            "1RB 2LA 1LA  2LA 2RB 0RA",  # wolfram
            self.progs)

        self.assert_cant_terminate()
        self.assert_simple_and_connected()


@skipUnless(RUN_SLOW, '')
class Slow(TestTree):
    def test_42h(self):
        params = 4, 2

        max_inf = 13_690

        def capture(prog: str) -> None:
            if 'D' not in prog:
                return

            if quick_term_or_rec(prog, 50):
                return

            for machine in run_variations(prog, 1000, params = params):
                if machine.xlimit is None:
                    self.add_result(prog, machine)
                    return

            machines = run_variations(
                prog, 1 + max_inf,
                block_steps = 6_000,
                params = params
            )

            for machine in machines:
                if machine.xlimit is None:
                    self.add_result(prog, machine)
                    return

            if cant_halt(prog):
                return

            self.queue.put(prog)

        run_tree_gen(
            steps = 35,
            halt = True,
            params = params,
            output = capture,
        )

        self.assert_progs(
            8,
            'holdouts_42h')

        self.assert_records({
            'blanks': (169, "1RB ...  0RC 0LA  1LC 1LD  0RB 0RD"),
            'spnout': (171, "1RB ...  0RC 0LA  1LC 1LD  0RB 0RD"),
            'halted': (107, "1RB 1LB  1LA 0LC  ... 1LD  1RD 0RA"),
            'infrul': (max_inf, "1RB 0LD  1LC 1RA  ... 1LA  0RA 1LD"),
        })

        self.assert_simple_and_connected()

    def test_24(self):
        params = 2, 4

        def capture(prog: str) -> None:
            if '3' not in prog:
                return

            if quick_term_or_rec(prog, 1_000):
                return

            machines = run_variations(
                prog, 15_000,
                block_steps = 6_000,
                params = params,
            )

            for machine in machines:
                if machine.xlimit is None:
                    return

            if cant_halt(prog):
                return

            if quick_term_or_rec(prog, 10_000):
                return

            self.queue.put(prog)

        run_tree_gen(
            steps = 100,
            halt = True,
            params = params,
            output = capture,
        )

        self.assert_progs(
            759,
            'holdouts_24h')

        self.assert_simple_and_connected()

    def test_42q(self):
        params = 4, 2

        def capture(prog: str) -> None:
            if 'D' not in prog:
                return

            if cant_spin_out(prog):
                return

            if quick_term_or_rec(prog, 1_000):
                return

            machine = Machine(
                prog,
                opt_macro = 1_000,
                params = params,
            ).run(10_000)

            if machine.simple_termination or machine.infrul:
                return

            if quick_term_or_rec(prog, 40_000):
                return

            self.queue.put(prog)

        run_tree_gen(
            steps = 200,
            halt = False,
            params = params,
            output = capture,
        )

        self.assert_progs(
            115,
            'holdouts_42q')
