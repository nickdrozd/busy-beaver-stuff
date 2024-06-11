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

########################################

PROGS: Q

RESULTS: dict[str, tuple[int, str]]


class TestTree(TestCase):
    progs: set[str]

    results: dict[str, tuple[int, str]]

    def setUp(self):
        global PROGS, RESULTS  # pylint: disable = global-statement

        PROGS = Queue()

        RESULTS = Manager().dict(  # type: ignore[assignment]
            blanks = (0, ""),
            halted = (0, ""),
            spnout = (0, ""),
            infrul = (0, ""),
        )

    def assert_progs(self, count: int, progfile: str):
        self.progs = queue_to_set(PROGS)

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
                tuple[int, str]],
    ):
        for cat, (exp_step, exp_prog) in expected.items():
            res_step, res_prog = RESULTS[cat]

            self.assertEqual(res_step, exp_step)

            self.assertTrue(
                res_prog == exp_prog)

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


def add_result(prog: str, machine: BasicMachine) -> None:
    if ((blanks := machine.blanks)
            and (res := min(blanks.values()))
            and  res > RESULTS['blanks'][0]):
        RESULTS['blanks'] = res, prog

    if ((spnout := machine.spnout)
            and spnout > RESULTS['spnout'][0]):
        RESULTS['spnout'] = spnout, prog
        return

    if ((und := machine.undfnd)
            and ((step := und[0] + 1) > RESULTS['halted'][0])):
        RESULTS['halted'] = step, prog
        return

    if (machine.infrul
            and (cycles := machine.cycles)
                    > RESULTS['infrul'][0]):
        RESULTS['infrul'] = cycles, prog
        return

########################################

PARAMS_22 = 2, 2
MAXINF_22 = 187

def capture_22(prog: str) -> None:
    machine = Machine(
        prog,
        opt_macro = 20,
        params = PARAMS_22,
    ).run(sim_lim = 1 + MAXINF_22)

    if machine.xlimit is None:
        add_result(prog, machine)
        return

    PROGS.put(prog)

########################################

PARAMS_32 = 3, 2
MAXINF_32 = 4_927

def capture_32(prog: str) -> None:
    machines = run_variations(
        prog, 1 + MAXINF_32,
        lin_rec = 50,
        params = PARAMS_32,
    )

    for machine in machines:
        if machine.xlimit is None:
            add_result(prog, machine)
            return

    PROGS.put(prog)

########################################

PARAMS_23 = 2, 3
MAXINF_23 = 7_395

def capture_23(prog: str) -> None:
    machines = run_variations(
        prog, 400,
        lin_rec = 50,
        params = PARAMS_23,
    )

    for machine in machines:
        if machine.xlimit is None:
            add_result(prog, machine)
            return

    machines = run_variations(
        prog, 1 + MAXINF_23,
        params = PARAMS_23,
    )

    for machine in machines:
        if machine.xlimit is None:
            add_result(prog, machine)
            return

    PROGS.put(prog)

########################################

class Fast(TestTree):
    def test_22(self):
        run_tree_gen(
            steps = 20,
            halt = False,
            params = PARAMS_22,
            output = capture_22,
        )

        self.assertFalse(
            queue_to_set(PROGS))

        self.assertIn(
            result := RESULTS['blanks'], {
                (8, "1RB 0RA  1LB 1LA"),
                (7, '1RB 0RA  0LB 1LA'),
            },
            result)

        self.assertIn(
            result := RESULTS['infrul'], {
                (MAXINF_22, "1RB 1LA  0LA 0RB"),
                (39, '1RB 0LB  1LA 1RB'),
                (39, '1RB 1LA  1LA 0RA'),
            },
            result)

        self.assertIn(
            result := RESULTS['spnout'], {
                (6, "1RB 0LB  0LB 1LA"),
                (6, "1RB 1LB  0LB 1LA"),
                (6, "1RB 1LB  1LB 1LA"),
                (6, '1RB 0LB  1LB 1LA'),
            },
            result)

    def test_32(self):
        run_tree_gen(
            steps = 15,
            halt = False,
            params = PARAMS_32,
            output = capture_32,
        )

        self.assert_progs(
            3,
            'holdouts_32q')

        self.assert_records({
            'blanks': (34, "1RB 1LB  1LA 1LC  1RC 0LC"),
            'spnout': (55, "1RB 0LB  1LA 0RC  1LC 1LA"),
            'infrul': (MAXINF_32, "1RB 1LA  0LB 1RC  1LA 0RB"),
        })

        self.assert_cant_terminate()
        self.assert_simple_and_connected()

    def test_23(self):
        run_tree_gen(
            steps = 23,
            halt = False,
            params = PARAMS_23,
            output = capture_23,
        )

        self.assert_progs(
            9,
            'holdouts_23q')

        self.assert_records({
            'blanks': (77, "1RB 2LA 0RB  1LA 0LB 1RA"),
            'spnout': (59, "1RB 2LB 1LA  2LB 2RA 0RA"),
            'infrul': (MAXINF_23, "1RB 2LA 0RB  1LB 1LA 1RA"),
        })

        self.assertIn(
            "1RB 2LA 1LA  2LA 2RB 0RA",  # wolfram
            self.progs)

        self.assert_cant_terminate()
        self.assert_simple_and_connected()

########################################

PARAMS_42 = 4, 2
MAXINF_42H = 13_690

def capture_42h(prog: str) -> None:
    if 'D' not in prog:
        return

    if quick_term_or_rec(prog, 50):
        return

    for machine in run_variations(prog, 1000, params = PARAMS_42):
        if machine.xlimit is None:
            add_result(prog, machine)
            return

    machines = run_variations(
        prog, 1 + MAXINF_42H,
        block_steps = 6_000,
        params = PARAMS_42,
    )

    for machine in machines:
        if machine.xlimit is None:
            add_result(prog, machine)
            return

    if cant_halt(prog):
        return

    PROGS.put(prog)

########################################

PARAMS_24 = 2, 4

def capture_24(prog: str) -> None:
    if '3' not in prog:
        return

    if quick_term_or_rec(prog, 1_000):
        return

    machines = run_variations(
        prog, 15_000,
        block_steps = 6_000,
        params = PARAMS_24,
    )

    for machine in machines:
        if machine.xlimit is None:
            return

    if cant_halt(prog):
        return

    if quick_term_or_rec(prog, 10_000):
        return

    PROGS.put(prog)

########################################

def capture_42q(prog: str) -> None:
    if 'D' not in prog:
        return

    if cant_spin_out(prog):
        return

    if quick_term_or_rec(prog, 1_000):
        return

    machine = Machine(
        prog,
        opt_macro = 1_000,
        params = PARAMS_42,
    ).run(10_000)

    if machine.simple_termination or machine.infrul:
        return

    if quick_term_or_rec(prog, 40_000):
        return

    PROGS.put(prog)

########################################

@skipUnless(RUN_SLOW, '')
class Slow(TestTree):
    def test_42h(self):
        run_tree_gen(
            steps = 35,
            halt = True,
            params = PARAMS_42,
            output = capture_42h,
        )

        self.assert_progs(
            8,
            'holdouts_42h')

        self.assert_records({
            'blanks': (169, "1RB ...  0RC 0LA  1LC 1LD  0RB 0RD"),
            'spnout': (171, "1RB ...  0RC 0LA  1LC 1LD  0RB 0RD"),
            'halted': (107, "1RB 1LB  1LA 0LC  ... 1LD  1RD 0RA"),
            # pylint: disable = line-too-long
            'infrul': (MAXINF_42H, "1RB 0LD  1LC 1RA  ... 1LA  0RA 1LD"),
        })

        self.assert_simple_and_connected()

    def test_24(self):
        run_tree_gen(
            steps = 100,
            halt = True,
            params = PARAMS_24,
            output = capture_24,
        )

        self.assert_progs(
            759,
            'holdouts_24h')

        self.assert_simple_and_connected()

    def test_42q(self):
        run_tree_gen(
            steps = 200,
            halt = False,
            params = PARAMS_42,
            output = capture_42q,
        )

        self.assert_progs(
            115,
            'holdouts_42q')
