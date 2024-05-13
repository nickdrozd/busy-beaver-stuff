# pylint: disable = too-many-lines
from __future__ import annotations

from math import isclose, log10
from typing import TYPE_CHECKING
from unittest import TestCase, expectedFailure, skip, skipUnless

# pylint: disable-next = wildcard-import, unused-wildcard-import
from test.prog_data import *
# pylint: disable-next = line-too-long
from test.test_num import assert_num_counts, clear_caches  # type: ignore[attr-defined]
from test.machine import QuickMachineResult, run_quick_machine
from test.lin_rec import (
    LinRecSampler,
    run_loose_linrec_machine,
    StrictLinRecMachine,
)
from test.utils import get_holdouts, read_holdouts, RUN_SLOW

from tm.parse import tcompile
from tm.show import show_comp
from tm.tree import Program
from tm.macro import opt_block
from tm.reason import (
    cant_halt,
    cant_blank,
    cant_spin_out,
)
from tm.machine import (
    show_slot,
    show_number,
    Machine,
    quick_term_or_rec,
)

from tools.graph import Graph
from tools.instr_seq import instr_seq
from tools.normalize import normalize

if TYPE_CHECKING:
    from typing import Any
    from collections.abc import Mapping

    from test.lin_rec import Tapes
    from tm.machine import Count

    BasicMachine = (
        Machine
        | QuickMachineResult
        | StrictLinRecMachine
        | LinRecSampler
    )


class TuringTest(TestCase):
    machine: BasicMachine

    maxDiff = None

    def assert_marks(self, marks: int):
        assert isinstance(
            self.machine,
            Machine | QuickMachineResult)

        try:
            self.assertEqual(
                actual_marks := self.machine.marks,
                marks)
        except AssertionError:
            try:
                self.assertEqual(
                    # pylint: disable = used-before-assignment
                    actual_marks,
                    marks - 1)
            except AssertionError:
                self.assertEqual(marks, 0)
                self.assertEqual(actual_marks, 1)

    def assert_steps(self, steps: int):
        try:
            self.assertEqual(
                self.machine.steps,
                steps)
        except AssertionError:
            self.assertEqual(
                self.machine.steps,
                steps - 1)

    def assert_cycles(self, cycles: int):
        self.assertEqual(
            self.machine.cycles,
            cycles)

    def assert_undefined(self, expected: tuple[int, str]):
        assert (undfnd := self.machine.undfnd) is not None
        step, slot = undfnd

        self.assertEqual(
            expected,
            (step, show_slot(slot)))

    def assert_normal(self, prog: str):
        self.assertTrue(
            Graph(prog).is_normal,
            prog)

        self.assertTrue(
            prog == normalize(prog)
                or prog.startswith('0')
                or prog == "1RB ...  ... ...",
            prog)

    def assert_connected(self, prog: str):
        self.assertTrue(
            Graph(prog).is_strongly_connected
            or prog in MODULAR
            or 'A' not in prog
            or '...' in prog
        )

    def assert_could_halt(self, prog: str):
        self.assertFalse(
            cant_halt(prog),
            f'halt false positive: {prog}')

    def assert_cant_halt(self, prog: str):
        self.assertTrue(
            cant_halt(prog)
                or prog in CANT_HALT_FALSE_NEGATIVES,
            f'halt false negative: "{prog}"')

    def assert_could_blank(self, prog: str):
        self.assertFalse(
            cant_blank(prog),
            f'blank false positive: "{prog}"')

    def assert_cant_blank(self, prog: str):
        self.assertTrue(
            cant_blank(prog)
                or prog in CANT_BLANK_FALSE_NEGATIVES
                or Machine(prog).run(sim_lim = 10).blanks,
            f'blank false negative: "{prog}"')

    def assert_could_spin_out(self, prog: str):
        self.assertFalse(
            cant_spin_out(prog),
            f'spin out false positive: "{prog}"')

    def assert_cant_spin_out(self, prog: str):
        self.assertTrue(
            cant_spin_out(prog)
                or prog in CANT_SPIN_OUT_FALSE_NEGATIVES,
            f'spin out false negative: "{prog}"')

    def assert_simple(self, prog: str):
        self.assertTrue(
            (graph := Graph(prog)).is_simple
                or prog in SPAGHETTI
                or prog in KERNEL,
            f'not simple: "{prog}": {len(graph.reduced)},')

    def analyze(
            self,
            prog: str,
            normal: bool = True,
            decomp: bool = True,
    ) -> None:
        if normal:
            self.assert_normal(prog)

        self.assert_simple(prog)
        self.assert_connected(prog)

        if decomp and prog != "1RB ...  ... ...":
            self.assertEqual(
                prog,
                show_comp(tcompile(prog)))

        _ = Machine(prog,  blocks = 2).run(sim_lim = 10)
        _ = Machine(prog, backsym = 1).run(sim_lim = 10)


def branch_last(prog: str) -> list[str]:
    program = Program(prog)
    assert len(slots := program.open_slots) == 1
    return program.branch(slots[0])


class Reason(TuringTest):
    def test_undefined(self):
        for prog, sequence in UNDEFINED.items():
            self.assertEqual(
                sequence,
                {
                    partial: (step, show_slot(slot))
                    for partial, step, slot in
                    instr_seq(prog)
                },
            )

    def test_mother_of_giants(self):
        mother = "1RB 1LE  0LC 0LB  0LD 1LC  1RD 1RA  ... 0LA"

        for prog in branch_last(mother):
            self.assert_could_blank(prog)
            self.assert_could_spin_out(prog)

    def test_bigfoot(self):
        bigfoot = "1RB 2RA 1LC  2LC 1RB 2RB  ... 2LA 1LA"

        self.assert_could_halt(bigfoot)

        for prog in branch_last(bigfoot):
            self.assert_cant_blank(prog)

        # pylint: disable = line-too-long
        two_color = "1RB 1RC  1RE 1RE  1LD 0RA  1RB 1LG  1LG 1RF  0RE 1RE  1LH 0LH  ... 1LC"

        self.assert_could_halt(two_color)

        for prog in branch_last(two_color):
            self.assert_cant_blank(prog)

    def test_blank(self):
        for prog in DONT_BLANK:
            self.assert_cant_blank(prog)

        for prog in BLANKERS:
            self.assert_simple(prog)
            self.assert_could_blank(prog)

    def test_false_negatives(self):
        for prog in CANT_HALT_FALSE_NEGATIVES:
            self.assertNotIn(prog, HALTERS)
            self.assert_could_halt(prog)

        for prog in CANT_BLANK_FALSE_NEGATIVES:
            self.assertNotIn(prog, BLANKERS)
            self.assert_could_blank(prog)

        for prog in CANT_SPIN_OUT_FALSE_NEGATIVES:
            self.assertNotIn(prog, SPINNERS)
            self.assert_could_spin_out(prog)

    def test_halt(self):
        for prog in HALTERS:
            self.assert_could_halt(prog)

        for prog in SPINNERS | RECURS:
            self.assert_cant_halt(prog)

    def test_spinout(self):
        for prog in SPINNERS:
            self.assert_simple(prog)

            if prog in MACRO_SPINOUT:
                continue

            self.assert_could_spin_out(prog)

        for prog in DONT_SPIN_OUT | HALTERS | RECURS:
            self.assert_cant_spin_out(prog)

    def test_recur(self):
        for prog in RECURS:
            self.assert_cant_halt(prog)
            self.assert_cant_spin_out(prog)

            if prog not in BLANKERS:
                self.assert_cant_blank(prog)

    def test_holdouts(self):
        for cat in ('42h', '24h'):
            for prog in read_holdouts(cat):
                self.assert_could_halt(prog)

        for prog in read_holdouts('42q'):
            self.assert_cant_halt(prog)
            self.assert_could_spin_out(prog)

        for cat in ('32q', '23q'):
            for prog in read_holdouts(cat):
                self.assert_cant_halt(prog)
                self.assert_cant_blank(prog)
                self.assert_cant_spin_out(prog)


class Holdouts(TestCase):
    def test_holdouts(self):
        self.assertEqual(
            len(holdouts := get_holdouts()),
            894)

        for prog in holdouts:
            self.assertFalse(
                quick_term_or_rec(prog, 1_000),
                prog)


class Simple(TuringTest):
    machine: QuickMachineResult

    def run_bb(
            self,
            prog: str,
            normal: bool = True,
            decomp: bool = True,
    ):
        print(prog)

        self.analyze(prog, normal = normal, decomp = decomp)

        self.machine = run_quick_machine(prog)

    def test_halt(self):
        self._test_halt(HALT)

    def test_spinout(self):
        self._test_spinout(SPINOUT)
        self._test_spinout(SPINOUT_BLANK, blank = True)

    def test_undefined(self):
        for sequence in UNDEFINED.values():
            for partial, expected in sequence.items():
                self.run_bb(partial, normal = False, decomp = False)

                self.assert_undefined(expected)

    def _test_simple_terminate(
            self,
            prog_data: Mapping[str, tuple[int | set[str], int]],
            blank: bool,
    ):
        for prog, (marks, steps) in prog_data.items():
            self.run_bb(prog)

            self.assert_steps(steps)

            try:
                self.assertEqual(
                    steps,
                    self.machine.simple_termination)
            except AssertionError:
                self.assertEqual(
                    steps - 1,
                    self.machine.simple_termination)

            blanks = self.machine.blanks

            if not blank:
                assert isinstance(marks, int)

                if marks > 0:
                    self.assert_marks(marks)

                    if prog[0] != '0' and marks > 2:
                        self.assertEqual(blanks, {})
                        self.assert_cant_blank(prog)

            else:
                self.assert_marks(0)
                self.assertEqual(steps, max(blanks.values()))
                self.assertEqual(
                    marks, {chr(blank + 65) for blank in blanks})
                self.assert_could_blank(prog)

            if self.machine.undfnd is not None:
                self.assert_could_halt(prog)
                self.assert_cant_spin_out(prog)

            else:
                self.assert_could_spin_out(prog)
                self.assert_cant_halt(prog)

                self.assertTrue(
                    (graph := Graph(prog)).is_zero_reflexive
                    and not graph.is_irreflexive
                )

    def _test_halt(self, prog_data: BasicTermData):
        self._test_simple_terminate(
            prog_data,
            blank = False,
        )

    def _test_spinout(
            self,
            prog_data: Mapping[str, tuple[int | set[str], int]],
            blank: bool = False,
    ):
        self._test_simple_terminate(
            prog_data,
            blank = blank,
        )


@skipUnless(RUN_SLOW, '')
class SimpleSlow(Simple):
    @skip('')
    def test_halt(self):
        self._test_halt(HALT_SLOW)

    def test_spinout(self):
        self._test_spinout(SPINOUT_SLOW)
        self._test_spinout(SPINOUT_BLANK_SLOW, blank = True)


class Recur(TuringTest):
    prog: str
    machine: Machine | StrictLinRecMachine | LinRecSampler

    def assert_quasihalt(self, qsihlt: bool | None):
        assert isinstance(self.machine, StrictLinRecMachine)

        self.assertEqual(
            self.machine.qsihlt,
            qsihlt)

    def run_bb(
            self,
            prog: str,
            *,
            print_prog: bool = True,
            check_rec: int | None = None,
            samples: Tapes | None = None,
            sim_lim: int | None = None,
    ):
        if print_prog:
            print(prog)

        if check_rec is not None:
            self.machine = StrictLinRecMachine(prog).run(
                check_rec = check_rec)
        elif samples is not None:
            assert sim_lim is not None
            self.machine = LinRecSampler(prog).run(
                samples = samples,
                sim_lim = sim_lim)
        else:
            self.machine = Machine(prog).run()

        self.analyze(prog)

    def test_recur(self):
        self._test_recur(
            RECUR_COMPACT
            | RECUR_DIFFUSE
        )

        self._test_recur(
            RECUR_BLANK_IN_PERIOD,
            blank = True,
            qsihlt = None,
        )

        self._test_recur(
            QUASIHALT,  # type: ignore[arg-type]
            qsihlt = True,
        )

    def _test_recur(
            self,
            prog_data: Mapping[
                str,
                tuple[
                    int | None,
                    int | tuple[int, int]]],
            quick: bool = True,
            blank: bool = False,
            qsihlt: bool | None = False,
    ):
        for prog, (steps, period) in prog_data.items():
            if isinstance(period, tuple):
                # pylint: disable = redefined-loop-name
                period, qsihlt_diff = period
            else:
                qsihlt_diff = 0

            self.prog = prog

            self.assertGreater(period, 1)

            self.assert_cant_halt(prog)
            self.assert_cant_spin_out(prog)

            if blank:
                self.assert_could_blank(prog)
            else:
                if prog not in BLANKERS:
                    self.assert_cant_blank(prog)
                assert steps is not None
                self.verify_lin_rec(prog, steps, period)

            if not quick or period > 2000:
                print(prog)

                self.assertTrue(
                    quick_term_or_rec(
                        prog,
                        1_000_000,
                    ),
                    prog)

                continue

            if blank:
                self.run_bb(prog)
            else:
                assert steps is not None
                self.run_bb(
                    prog,
                    check_rec = (
                        0
                        if steps < 256 else
                        steps
                    ) - qsihlt_diff,
                )

            if blank:
                self.assertTrue(
                    self.machine.infrul)

            else:
                assert isinstance(self.machine, StrictLinRecMachine)

                assert self.machine.linrec is not None

                self.assertEqual(
                    period,
                    self.machine.linrec[1])

                self.assert_quasihalt(qsihlt)

            if steps is None or steps < 100_000:
                self.assertIsNotNone(
                    run_loose_linrec_machine(prog, 100_000).infrul)

                self.assertTrue(
                    quick_term_or_rec(prog, 100_000))

    def assert_lin_rec(self, steps: int, recur: int):
        assert isinstance(self.machine, LinRecSampler)
        history = self.machine.history

        self.assertEqual(
            history.states[steps],
            history.states[recur],
        )

        self.assertEqual(
            history.verify_lin_rec(
                steps,
                recur,
            ),
            (steps, recur - steps),
            self.prog,
        )

    def deny_lin_rec(self, steps: int, recur: int):
        assert isinstance(self.machine, LinRecSampler)
        history = self.machine.history

        states = history.states

        if states[steps] == states[recur]:
            self.assertIsNone(
                history.verify_lin_rec(
                    steps,
                    recur,
                ),
                self.prog,
            )

    def verify_lin_rec(self, prog: str, steps: int, period: int):
        recur   = period + steps
        runtime = period + recur

        self.run_bb(
            prog,
            print_prog = False,
            sim_lim = 1 + runtime,
            samples = {
                steps - 1      : None,  # type: ignore[dict-item]
                steps          : None,  # type: ignore[dict-item]
                steps + 1      : None,  # type: ignore[dict-item]
                recur - 1      : None,  # type: ignore[dict-item]
                recur          : None,  # type: ignore[dict-item]
                recur + 1      : None,  # type: ignore[dict-item]
                recur + period : None,  # type: ignore[dict-item]
            },
        )

        self.assert_lin_rec(    steps,          recur)
        self.assert_lin_rec(1 + steps,      1 + recur)
        self.assert_lin_rec(    steps, period + recur)

        assert period > 1

        self.deny_lin_rec(steps, 1 + recur)
        self.deny_lin_rec(steps, recur - 1)

        if steps >= 1:
            self.deny_lin_rec(steps - 1, recur)


@skipUnless(RUN_SLOW, '')
class RecurSlow(Recur):
    def test_recur(self):
        self._test_recur(RECUR_SLOW, quick = False)


class Prover(TuringTest):
    machine: Machine

    def assert_spinout(self) -> None:
        self.assertIsNotNone(
            self.machine.spnout)

    def assert_macro_cells(self, cells: int):
        assert not isinstance(macro := self.machine.program, dict)

        self.assertEqual(
            macro.cells,  # type: ignore[attr-defined]
            cells)

    def assert_mult_rules(self) -> None:
        self.assertTrue(any(
            isinstance(diff, tuple)
            for rules in self.machine.prover.rules.values()
            for _, rule in rules
            for diff in rule.values()))

    def assert_close(
            self,
            this: int,
            that: int | float,
            rel_tol: float,
    ):
        self.assertTrue(
            isclose(
                this,
                that,
                rel_tol = rel_tol,
            )
        )

    def run_bb(  # pylint: disable = too-many-arguments
        self,
        prog: str,
        *,
        print_prog: bool = True,
        analyze: bool = True,
        normal: bool = True,
        blocks: int | None = None,
        backsym: int | None = None,
        opt_macro: int | None = None,
        **opts,
    ):
        self.machine = Machine(
            prog,
            blocks = blocks,
            backsym = backsym,
            opt_macro = opt_macro,
        )

        if print_prog:
            print(self.machine.prog_str)

        self.machine.run(**opts)

        if analyze:
            self.analyze(prog, normal = normal)

    def test_prover(self):
        self._test_prover_est(
            PROVER_HALT
            | PROVER_SPINOUT
        )

        self._test_prover(
            HALT
            | HALT_SLOW
            | SPINOUT
        )

        self._test_prover(
            SPINOUT_BLANK,
            blank = True,
        )

        self._test_prover(
            RECUR_COMPACT
            | QUASIHALT,
            simple_term = False,
        )

        self.run_bb(
            "1RB 0LC  1RD 1RA  ... 0LD  1LA 0LB",
            analyze = False,
        )

        self.run_bb(
            "1RB 2RA 2RC  1LC ... 1LA  1RA 2LB 1LC",
        )

        self.run_bb(
            "1RB 2LA 1RA 1RA  1LB 1LA 3RB ...",
            backsym = 2,
        )

    def _test_prover(  # type: ignore[misc]
            self,
            prog_data: Mapping[str, Any],
            blank: bool = False,
            simple_term: bool = True,
    ):
        for prog in prog_data:
            if prog == "1RB 2LB 1LC  1LA 2RB 1RB  ... 2LA 0LC":  # SIAB
                continue

            self.run_bb(
                prog,
                opt_macro = 10_000,
            )

            if simple_term:
                self.assertIsNotNone(
                    self.machine.simple_termination)

                if not isinstance(self.machine.program, dict):
                    continue

                self.assert_marks(
                    0 if blank else prog_data[prog][0])
            else:
                self.assertTrue(
                    self.machine.infrul
                    or self.machine.cfglim
                    or self.machine.spnout is not None)

            self.assertIsInstance(
                self.machine.rulapp,
                int)

    def _test_prover_est(self, prog_data: ProverEst):
        champ_2_5 = "1RB 2LB 4LB 3LA ...  1LA 3RA 3LB 0LB 0RA"

        for prog, marks in prog_data.items():
            if prog in PROVER_FAILURES:
                continue

            if prog in PROVER_HALT_TOO_SLOW and not RUN_SLOW:
                continue

            self.run_bb(
                prog,
                sim_lim = 10 ** 8,
                opt_macro = 3_000,
                backsym = (
                    2
                    if prog == champ_2_5 else
                    None
                ),
                normal = False,
            )

            if not isinstance(self.machine.rulapp, int):
                assert prog in ALGEBRA_PROGS, prog

            self.assertIsNotNone(
                self.machine.simple_termination)

            result: Count = self.machine.marks

            if not isinstance(result, int):
                self.assert_mult_rules()

            if is_macro := (
                    not isinstance(
                        macro := self.machine.program, dict)):
                result *= macro.cells  # type: ignore[attr-defined]

            if isinstance(marks, int):
                self.assertEqual(result, marks)
            elif isinstance(marks, str):
                self.assertTrue(
                    str(result).startswith(marks))
            else:
                digits, exp = marks

                if exp < 100_000:
                    self.assert_close(
                        int(result) / 10 ** exp,
                        digits,
                        rel_tol = .54,
                    )
                else:
                    magnitude = (
                        int(log10(result))
                        if isinstance(result, int) else
                        result.digits()
                    )

                    self.assertEqual(exp, magnitude)

            if result < 5:
                self.assert_could_blank(prog)
            else:
                self.assert_cant_blank(prog)

            if self.machine.undfnd is not None:
                self.assert_cant_spin_out(prog)
                self.assert_could_halt(prog)
            else:
                self.assert_cant_halt(prog)

                try:
                    self.assert_could_spin_out(prog)
                except AssertionError:
                    self.assertTrue(is_macro)

            if prog in SUSPECTED_RULES:
                self.assertIsNotNone(
                    self.machine.susrul)

            if self.machine.susrul is not None:
                self.assertIn(
                    prog,
                    SUSPECTED_RULES)

    def test_macro_undefined(self):
        self.run_bb(
            "1RB 0RA 1LB  2LA 2RB 0LA",
            blocks = 3,
            backsym = 1,
        )

        self.assertEqual(
            0,
            self.machine.infrul)

        self.run_bb(
            "1RB ...  1LB 0RC  1RB 1RD  0RE 0LD  1LF 1RA  1LC ...",
            opt_macro = 2_000,
        )

        self.assert_undefined((4470, 'A1'))

        self.run_bb(
            "1RB 0LC  0RD 1RA  ... 0LD  1LE 1LA  0LF 1LA  0RE 1LF",
            opt_macro = 200)

        self.assertEqual(
            self.machine.infrul,
            -1)

    def test_rule_limit(self):
        for prog, reason in RULE_LIMIT.items():
            self.run_bb(
                prog,
                print_prog = prog not in PROVER_FAILURES,
                normal = False,
                opt_macro = 1600,
            )

            self.assertTrue(
                str(self.machine.limrul).startswith(reason))

    def test_backsymbol_not_required(self):
        prog = "1RB 0LC  1LC 0RC  1LA 0LC"

        self.run_bb(
            prog,
            sim_lim = 100,
        )

        self.assertEqual(
            84,
            self.machine.infrul)

        self.run_bb(
            prog,
            backsym = 1,
            sim_lim = 100,
        )

        self.assertIsNotNone(
            self.machine.infrul)

    def test_show_state_overflow(self):
        self.run_bb(
            "1RB 2LA 3RA 0LA  1LA 2RA 0RB ...",
            opt_macro = 4_000)

        self.assert_macro_cells(41)

        with self.assertRaises(OverflowError):
            self.run_bb(
                "1RB 2LA 3RA 0LA  1LA 2RA 0RB ...",
                opt_macro = 4_000,
                watch_tape = True)

    def test_prover_false_positive(self):
        self.run_bb(
            "1RB 1LD ...  1RC 2LB 2LD  1LC 2RA 0RD  1RC 1LA 0LA",
            analyze = False)

        self.assert_marks(237)
        self.assert_cycles(546)

        ########################################

        prog = "1RB 0RD  1LC 0RA  1LA 1LB  ... 0RC"

        self.run_bb(
            prog,
            normal = False,
        )

        self.assertIsNotNone(
            self.machine.undfnd)

        self.run_bb(
            prog,
            backsym = 1,
            normal = False,
        )

        self.assertIsNone(
            self.machine.undfnd)

        ########################################

        prog = "1RB 0LD  1RC 0RF  1LC 1LA  0LE ...  1LA 0RB  0RC 0RE"

        for backsym in range(0, 7):
            self.run_bb(
                prog,
                backsym = backsym or None,
                normal = False,
            )

            if backsym in {0, 3, 4, 6}:
                self.assertNotIsInstance(
                    self.machine.rulapp,
                    int)
            else:
                assert backsym in {1, 2, 5}

                self.assertIsInstance(
                    self.machine.rulapp,
                    int)

    @expectedFailure
    def test_wrong_block(self):
        prog = "1RB 0LA  1RC ...  1LD 0RC  0LA 1LD"

        self.run_bb(
            prog,
            blocks = 2,
        )

        self.assertIsNotNone(
            self.machine.infrul)

        blocks = {
            7_130: 1,
            7_131: 4,
        }

        for steps, block in blocks.items():
            self.assertEqual(
                block,
                opt_block(prog, steps))

            self.run_bb(
                prog,
                blocks = block,
                sim_lim = 8806,
            )

            self.assertIsNotNone(
                self.machine.xlimit)

        self.run_bb(
            prog,
            blocks = 4,
            sim_lim = 8807,
        )

        self.assertIsNotNone(
            self.machine.infrul)

        ########################################

    def test_block_mult(self):
        prog = "1RB 1RC 0RC  1RC 0LA 1LB  2LC 2RA 1LB"

        self.run_bb(
            prog,
            blocks = 4,
            analyze = False)

        self.assert_marks(9899724)
        self.assert_cycles(5441)
        self.assert_spinout()

        self.run_bb(
            prog,
            opt_macro = 1_000,
            sim_lim = 1,
            analyze = False)

        self.assert_macro_cells(2)

    def test_algebra(self):
        clear_caches()

        show = False

        for term, progs in ALGEBRA.items():
            if show:
                print(f'    "{term}": {{')

            for prog, (cycles, est, string, rulapp) in progs.items():
                self.run_bb(
                    prog,
                    opt_macro = 2000,
                    analyze = False,
                    backsym = 1 if term == 'times-depth' else None,
                    print_prog = not show,
                )

                if self.machine.undfnd is not None:
                    self.assertEqual(term, 'halt', prog)

                    self.assertIn(
                        prog,
                        PROVER_HALT)

                elif self.machine.spnout is not None:
                    self.assertEqual(term, 'spinout', prog)

                    self.assertIn(
                        prog,
                        PROVER_SPINOUT)

                elif self.machine.infrul is not None:
                    self.assertEqual(term, 'infrul', prog)

                else:
                    self.assertEqual(
                        term,
                        self.machine.limrul,
                        prog)

                marks = self.machine.marks

                estimate = (
                    marks
                    if isinstance(marks, int) else
                    marks.estimate()
                )

                show_rulapp = show_number(self.machine.rulapp)

                if show:
                    print('\n'.join([
                        f'        "{prog}": (',
                        f'            {self.machine.cycles},',
                        f'            "{estimate}",',
                        f'            "{marks}",',
                        f'            "{show_rulapp}",',
                        '        ),',
                    ]))

                    continue

                self.assert_cycles(cycles)

                self.assertEqual(
                    string,
                    show_number(marks),
                    prog)

                self.assertEqual(
                    est,
                    show_number(
                        estimate))

                self.assertEqual(
                    rulapp,
                    show_rulapp,
                    prog)

            if show:
                print('    },\n')

        assert_num_counts({
            "adds": 48034,
            "divs": 13574,
            "exps": 12614,
            "muls": 12166,
            "totl": 86388,
        })
