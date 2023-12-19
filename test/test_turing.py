# pylint: disable = too-many-lines
from __future__ import annotations

from math import isclose, log10
from typing import TYPE_CHECKING
from itertools import product
from unittest import TestCase, skip, expectedFailure

# pylint: disable-next = wildcard-import, unused-wildcard-import
from test.prog_data import *
from test.test_num import assert_num_counts, clear_caches
from test.machine import QuickMachine
from test.lin_rec import (
    LinRecSampler,
    LooseLinRecMachine,
    StrictLinRecMachine,
)

from tm.reason import (
    Program,
    cant_halt,
    cant_blank,
    cant_spin_out,
)
from tm.machine import (
    show_slot,
    show_number,
    opt_block,
    Machine,
    quick_term_or_rec,
)

from tools.instr_seq import instr_seq

if TYPE_CHECKING:
    from typing import Any
    from collections.abc import Mapping

    from test.lin_rec import Tapes
    from tm.machine import Count, GetInstr

    BasicMachine = (
        Machine
        | QuickMachine
        | StrictLinRecMachine
        | LinRecSampler
    )


class TuringTest(TestCase):
    prog: str
    machine: BasicMachine

    maxDiff = None

    def assert_marks(self, marks: int):
        self.assertEqual(
            self.machine.marks,  # type: ignore[union-attr]
            marks)

    def assert_steps(self, steps: int):
        self.assertEqual(
            self.machine.steps,
            steps)

    def assert_cycles(self, cycles: int):
        self.assertEqual(
            self.machine.cycles,
            cycles)

    def assert_spinout(self) -> None:
        self.assertIsNotNone(
            self.machine.spnout)

    def assert_macro_cells(self, cells: int):
        self.assertEqual(
            self.machine.program.cells,  # type: ignore
            cells)

    def assert_quasihalt(self, qsihlt: bool | None):
        assert isinstance(self.machine, StrictLinRecMachine)

        self.assertEqual(
            self.machine.qsihlt,
            qsihlt)

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

    def assert_normal(self, prog: str):
        self.assertTrue(
            Program(prog).graph.is_normal,
            prog)

        self.assertTrue(
            prog == Program(prog).normalize()
            or prog.startswith('0')
        )

    def assert_connected(self, prog: str):
        self.assertTrue(
            Program(prog).graph.is_strongly_connected
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
            Program(prog).graph.is_simple
            or prog in SPAGHETTI
            or prog in KERNEL
        )

    def assert_mult_rules(self) -> None:
        assert isinstance(self.machine, Machine)

        self.assertTrue(any(
            isinstance(diff, tuple)
            for rules in self.machine.prover.rules.values()
            for _, rule in rules
            for diff in rule.values()))

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

    def run_bb(  # pylint: disable = too-many-arguments
            self,
            prog: str | GetInstr,
            *,
            print_prog: bool = True,
            analyze: bool = True,
            normal: bool = True,
            blocks: int | list[int] | None = None,
            backsym: int | list[int] | None = None,
            opt_macro: int | None = None,
            prover: bool = True,
            check_rec: int | None = None,
            samples: Tapes | None = None,
            **opts,
    ):
        if check_rec is not None:
            assert isinstance(prog, str)
            self.machine = StrictLinRecMachine(prog)
        elif samples is not None:
            assert isinstance(prog, str)
            self.machine = LinRecSampler(prog)
        elif not prover:
            self.machine = QuickMachine(
                prog
                if (not blocks and not opt_macro and not backsym) else
                Machine(
                    prog,
                    blocks = blocks,
                    backsym = backsym,
                    opt_macro = opt_macro,
                ).program
            )
        else:
            self.machine = Machine(
                prog,
                blocks = blocks,
                backsym = backsym,
                opt_macro = opt_macro,
            )

        if print_prog:
            print(self.machine.program)

        if check_rec is not None:
            assert isinstance(self.machine, StrictLinRecMachine)
            self.machine.run(check_rec = check_rec, **opts)
        elif samples is not None:
            assert isinstance(self.machine, LinRecSampler)
            # pylint: disable = unexpected-keyword-arg
            self.machine.run(samples = samples, **opts)
        else:
            self.machine.run(**opts)

        if not analyze or not isinstance(prog, str):
            return

        if normal:
            self.assert_normal(prog)

        self.assert_simple(prog)
        self.assert_connected(prog)

        _ = Machine(prog,  blocks = 2).run(sim_lim = 10)
        _ = Machine(prog, backsym = 1).run(sim_lim = 10)

    def _test_simple_terminate(
            self,
            prog_data: Mapping[str, tuple[int | set[str], int]],
            blank: bool,
    ):
        for prog, (marks, steps) in prog_data.items():
            self.run_bb(prog, prover = False)

            self.assert_steps(steps)

            assert isinstance(self.machine, QuickMachine)

            self.assertEqual(
                steps,
                self.machine.simple_termination)

            blanks = self.machine.blanks

            if not blank:
                assert isinstance(marks, int)

                if marks > 0:
                    self.assert_marks(marks)

                    if prog[0] != '0':
                        self.assertEqual(blanks, {})
                        self.assert_cant_blank(prog)

            else:
                self.assert_marks(0)
                self.assertEqual(steps, max(blanks.values()))
                self.assertEqual(
                    marks, {chr(blank + 65) for blank in blanks})
                self.assert_could_blank(prog)

            if '_' in prog:
                self.assert_could_halt(prog)
                self.assert_cant_spin_out(prog)

            else:
                self.assert_could_spin_out(prog)
                self.assert_cant_halt(prog)

                self.assertTrue(
                    (graph := Program(prog).graph).is_zero_reflexive
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
                self.assertTrue(
                    LooseLinRecMachine(prog).run(
                        100_000
                    ).infrul)

                self.assertTrue(
                    quick_term_or_rec(prog, 100_000))

    def _test_prover(  # type: ignore[misc]
            self,
            prog_data: Mapping[str, Any],
            blank: bool = False,
            simple_term: bool = True,
    ):
        for prog in prog_data:
            if prog == "1RB 2LB 1LC  1LA 2RB 1RB  1R_ 2LA 0LC":  # SIAB
                continue

            self.run_bb(
                prog,
                opt_macro = 10_000,
            )

            assert isinstance(self.machine, Machine)

            if simple_term:
                self.assertIsNotNone(
                    self.machine.simple_termination)

                if not isinstance(self.machine.program, str):
                    continue

                self.assert_marks(
                    0 if blank else prog_data[prog][0])
            else:
                self.assertTrue(
                    self.machine.infrul
                    or self.machine.cfglim
                    or self.machine.spnout is not None)

    def _test_prover_est(self, prog_data: ProverEst):
        champ_2_5 = "1RB 2LB 4LB 3LA 1R_  1LA 3RA 3LB 0LB 0RA"

        for prog, marks in prog_data.items():
            if prog in PROVER_FAILURES:
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

            assert isinstance(self.machine, Machine)

            self.assertIsNotNone(
                self.machine.simple_termination)

            result: Count = self.machine.marks

            if not isinstance(result, int):
                self.assert_mult_rules()

            if not isinstance(macro := self.machine.program, str):
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

            if '_' in prog:
                self.assert_cant_spin_out(prog)
                self.assert_could_halt(prog)
            else:
                self.assert_cant_halt(prog)
                self.assert_could_spin_out(prog)

            if prog in SUSPECTED_RULES:
                self.assertIsNotNone(
                    self.machine.susrul)

            if self.machine.susrul is not None:
                self.assertIn(
                    prog,
                    SUSPECTED_RULES)

    def _test_macro_cycles(self, prog_data: MacroCycles):
        for program, cycleses in prog_data.items():
            if isinstance(program, tuple):
                prog, opt = program
                sim_lim = opt
            else:
                assert isinstance(program, str)
                prog, opt, sim_lim = program, 0, None

            macro_params = (
                (None, None),
                (2, None),
                (3, None),
                (None, 1),
                (2, 1),
                (3, 1),
                (None, [1, 1]),
                (None, 1, 2),
                (None, 1, 3),
            )

            self.assertEqual(
                len(cycleses),
                len(macro_params))

            run_lim = (
                20_000 if opt is None else
                sim_lim  if sim_lim is not None else
                10 ** 10
            )

            for cycles, params in zip(cycleses, macro_params):
                if (cycles is not None and cycles > 10_000_000):
                    continue

                match params:
                    case (blocks, backsym):
                        assert isinstance(blocks, int | None)
                        assert isinstance(backsym, int | list | None)

                        self.run_bb(
                            prog,
                            blocks = blocks,
                            backsym = backsym,
                            sim_lim = run_lim,
                            prover = False,
                        )

                    case (_, backsym, blocks):
                        self.run_bb(
                            Machine(prog, backsym = backsym).program,
                            blocks = blocks,
                            sim_lim = run_lim,
                            prover = False,
                        )

                assert isinstance(self.machine, QuickMachine)

                self.assertEqual(
                    cycles,
                    None
                    if (self.machine.simple_termination is None
                        and opt is None) else
                    self.machine.cycles
                    if sim_lim is None else
                    self.machine.steps)

                if (sim_lim is None
                        and not isinstance(
                            macro := self.machine.program,
                            str)):
                    self.assertTrue(
                        len(macro) <= 60,
                        (len(macro), str(macro)))

    def _test_block_macro_steps(
            self,
            wraps: int,
            cells: int,
            rel_tol: float = .001,
    ):
        for prog, steps in BLOCK_MACRO_STEPS.items():
            for wrap, cell in product(range(1, wraps), range(1, cells)):
                self.run_bb(
                    prog,
                    blocks = [cell] * wrap,
                    prover = False,
                )

                assert isinstance(self.machine, QuickMachine)

                assert isinstance(
                    term := self.machine.simple_termination,
                    int)

                self.assert_close(
                    term,
                    steps / (cell ** wrap),
                    rel_tol = rel_tol,
                )


class Reasoner(TuringTest):
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

        for prog in Program(mother).branch_read('E0'):
            self.assert_could_blank(prog)
            self.assert_could_spin_out(prog)

    def test_bigfoot(self):
        bigfoot = "1RB 2RA 1LC  2LC 1RB 2RB  ... 2LA 1LA"

        self.assert_could_halt(bigfoot)

        for prog in Program(bigfoot).branch_read('C0'):
            self.assert_cant_blank(prog)

        # pylint: disable = line-too-long
        two_color = "1RB 1RC  1RE 1RE  1LD 0RA  1RB 1LG  1LG 1RF  0RE 1RE  1LH 0LH  ... 1LC"

        self.assert_could_halt(two_color)

        for prog in Program(two_color).branch_read('H0'):
            self.assert_cant_blank(prog)

    def test_blank(self):
        for prog in DONT_BLANK:
            self.assert_cant_blank(prog)

        for prog in BLANKERS:
            self.assert_simple(prog)
            self.assert_could_blank(prog)

    def test_false_negatives(self):
        for prog in CANT_HALT_FALSE_NEGATIVES:
            self.assert_could_halt(prog)
            self.assert_could_halt(prog.replace('...', '1R_'))

        for prog in CANT_BLANK_FALSE_NEGATIVES:
            self.assertNotIn(prog, BLANKERS)
            self.assert_could_blank(prog)

        for prog in CANT_SPIN_OUT_FALSE_NEGATIVES:
            self.assertNotIn(
                prog,
                SPINOUT
                 | SPINOUT_SLOW
                 | SPINOUT_BLANK
                 | SPINOUT_BLANK_SLOW)

            self.assert_could_spin_out(prog)

    def test_halt(self):
        for prog in DO_HALT | set(HALT_SLOW):
            self.assert_could_halt(prog)

    def test_spinout(self):
        for prog in DO_SPIN_OUT | set(SPINOUT_SLOW):
            self.assert_simple(prog)
            self.assert_could_spin_out(prog)

        for prog in DONT_SPIN_OUT:
            self.assert_cant_spin_out(prog)

    def test_recur(self):
        for prog in RECUR_COMPACT | RECUR_DIFFUSE | RECUR_TOO_SLOW:
            self.assert_cant_halt(prog)
            self.assert_cant_blank(prog)
            self.assert_cant_spin_out(prog)


class Fast(TuringTest):
    def test_halt(self):
        self._test_halt(HALT)

    def test_spinout(self):
        self._test_spinout(SPINOUT)
        self._test_spinout(SPINOUT_BLANK, blank = True)

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
            "1RB 2RA 2RC  1LC 1R_ 1LA  1RA 2LB 1LC",
        )

        self.run_bb(
            "1RB 2LA 1RA 1RA  1LB 1LA 3RB 1R_",
            backsym = 2,
        )

    def test_undefined(self):
        for sequence in UNDEFINED.values():
            for partial, expected in sequence.items():
                self.run_bb(partial, prover = False, normal = False)

                assert (undfnd := self.machine.undfnd) is not None
                step, slot = undfnd

                self.assertEqual(
                    expected,
                    (step, show_slot(slot)))

    def test_block_macro_steps(self):
        self._test_block_macro_steps(4, 5)

    def test_macro_cycles(self):
        self._test_macro_cycles(MACRO_CYCLES_FAST)

    def test_macro_multi_backsymbol(self):
        for prog in HALT | SPINOUT:
            if len(prog) > 35:
                continue

            for back in range(1, 5):
                self.run_bb(
                    prog,
                    backsym = [back] * back)

                assert isinstance(self.machine, Machine)

                self.assertIsNotNone(
                    self.machine.simple_termination)

    def test_rule_limit(self):
        for prog, reason in RULE_LIMIT.items():
            self.run_bb(
                prog,
                print_prog = prog not in PROVER_FAILURES,
                normal = False,
                opt_macro = 1600,
            )

            assert isinstance(self.machine, Machine)

            self.assertTrue(
                str(self.machine.limrul).startswith(reason))

    def test_backsymbol_required(self):
        prog = "1RB 0LC  1LC 0RC  1LA 0LC"

        self.run_bb(
            prog,
            sim_lim = 100,
        )

        self.assertIsNone(
            self.machine.infrul)

        self.run_bb(
            prog,
            backsym = 1,
            sim_lim = 100,
        )

        self.assertIsNotNone(
            self.machine.infrul)

    def test_prover_false_positive(self):
        self.run_bb(
            "1RB 1LD 1R_  1RC 2LB 2LD  1LC 2RA 0RD  1RC 1LA 0LA",
            analyze = False)

        self.assert_marks(237)
        self.assert_cycles(546)

        ########################################

        prog = "1RB 0RD  1LC 0RA  1LA 1LB  1R_ 0RC"

        self.run_bb(
            prog,
            normal = False,
        )

        self.assertIsNotNone(
            self.machine.halted)

        self.run_bb(
            prog,
            backsym = 1,
            normal = False,
        )

        self.assertIsNone(
            self.machine.halted)

        ########################################

        prog = "1RB 0LD  1RC 0RF  1LC 1LA  0LE 1R_  1LA 0RB  0RC 0RE"

        for backsym in range(0, 7):
            self.run_bb(
                prog,
                backsym = backsym or None,
                normal = False,
            )

            assert isinstance(self.machine, Machine)

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
                    print_prog = not show,
                )

                assert isinstance(self.machine, Machine)

                if self.machine.halted is not None:
                    self.assertEqual(term, 'halt', prog)

                    self.assertIn(
                        prog,
                        PROVER_HALT)

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
            "adds": 38345,
            "divs": 8394,
            "exps": 10214,
            "muls": 10309,
            "totl": 67262,
        })


class Slow(TuringTest):
    @skip('')
    def test_halt(self):
        self._test_halt(HALT_SLOW)

    def test_spinout(self):
        self._test_spinout(SPINOUT_SLOW)
        self._test_spinout(SPINOUT_BLANK_SLOW, blank = True)

    def test_recur(self):
        self._test_recur(RECUR_SLOW, quick = False)

    def test_macro_cycles(self):
        self._test_macro_cycles(MACRO_CYCLES_SLOW)
