# pylint: disable = line-too-long, wildcard-import, unused-wildcard-import

from math import isclose, log10
from typing import Any
from unittest import skip
from itertools import product
from collections.abc import Mapping

from test.prog_data import *
from test.test_program import BackwardReasoning

from tm.tape import Tape
from tm.parse import str_st
from tm.utils import opt_block
from tm.instrs import GetInstr
from tm.program import Program
from tm.machine import Machine, LinRecMachine
from tm.macro import BlockMacro, BacksymbolMacro


class TuringTest(BackwardReasoning):
    prog: str
    tape: Tape
    machine: Machine

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

    def assert_marks(self, marks: int):
        self.assertEqual(
            self.machine.marks,
            marks)

    def assert_steps(self, steps: int):
        self.assertEqual(
            self.machine.steps,
            steps)

    def assert_quasihalt(self, qsihlt: bool | None):
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

    def assert_lin_recurrence(self, steps: int, recurrence: int):
        assert isinstance(self.machine, LinRecMachine)
        history = self.machine.history

        self.assertEqual(
            history.states[steps],
            history.states[recurrence],
        )

        self.assertEqual(
            history.verify_lin_recurrence(
                steps,
                recurrence,
            ),
            (steps, recurrence - steps),
            self.prog,
        )

    def deny_lin_recurrence(self, steps: int, recurrence: int):
        assert isinstance(self.machine, LinRecMachine)
        history = self.machine.history

        states = history.states

        if states[steps] == states[recurrence]:
            self.assertIsNone(
                history.verify_lin_recurrence(
                    steps,
                    recurrence,
                ),
                self.prog,
            )

    def verify_lin_recurrence(self, prog: str, steps: int, period: int):
        recurrence = period + steps
        runtime    = period + recurrence

        self.run_bb(
            prog,
            print_prog = False,
            lin_rec = True,
            step_lim = 1 + runtime,
            samples = {
                steps - 1           : None,
                steps               : None,
                steps + 1           : None,
                recurrence - 1      : None,
                recurrence          : None,
                recurrence + 1      : None,
                recurrence + period : None,
            },
        )

        self.assert_lin_recurrence(    steps,     recurrence)
        self.assert_lin_recurrence(1 + steps, 1 + recurrence)
        self.assert_lin_recurrence(steps, period + recurrence)

        assert period > 1

        self.deny_lin_recurrence(steps, 1 + recurrence)
        self.deny_lin_recurrence(steps, recurrence - 1)

        if steps >= 1:
            self.deny_lin_recurrence(steps - 1, recurrence)

    def run_bb(
            self,
            prog: str | GetInstr,
            print_prog: bool = True,
            analyze: bool = True,
            normal: bool = True,
            lin_rec: bool = False,
            **opts,
    ):
        if print_prog:
            print(prog)

        if lin_rec:
            assert isinstance(prog, str)
            self.machine = LinRecMachine(prog).run(**opts)
        else:
            self.machine = Machine(prog).run(**opts)

        self.tape = self.machine.tape

        if not analyze or not isinstance(prog, str):
            return

        if normal:
            self.assert_normal(prog)

        self.assert_simple(prog)
        self.assert_connected(prog)

        _ = Machine(     BlockMacro(prog, [2])).run(sim_lim = 10)
        _ = Machine(BacksymbolMacro(prog, [1])).run(sim_lim = 10)

    def _test_simple_terminate(
            self,
            prog_data: Mapping[str, tuple[int | set[str], int]],
            blank: bool,
    ):
        for prog, (marks, steps) in prog_data.items():
            self.run_bb(prog)

            self.assert_steps(steps)

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

            self.assert_quasihalt(True)

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
            prog_data: Mapping[str, tuple[int | None, int]],
            quick: bool = True,
            blank: bool = False,
            qsihlt: bool | None = False,
    ):
        for prog, (steps, period) in prog_data.items():
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
                self.verify_lin_recurrence(prog, steps, period)

            if not quick or period > 2000:  # no-coverage
                print(prog)
                continue

            if blank:
                self.run_bb(prog)
                assert self.machine.linrec is not None
                self.assertEqual(
                    period,
                    self.machine.linrec[1])
                self.assert_quasihalt(qsihlt)
            else:
                assert steps is not None
                self.run_bb(
                    prog,
                    lin_rec = True,
                    check_rec = 0 if steps < 256 else steps,
                )

                assert self.machine.linrec is not None
                self.assertEqual(
                    period,
                    self.machine.linrec[1])

                self.assert_quasihalt(qsihlt)

    def _test_prover(  # type: ignore[misc]
            self,
            prog_data: Mapping[str, Any],
            blank: bool = False,
            simple_term: bool = True,
    ):
        for prog in prog_data:
            if prog == "1RB 2LB 1LC  1LA 2RB 1RB  1R_ 2LA 0LC":  # SIAB
                continue

            program: str | BlockMacro = (
                prog
                if (opt := opt_block(prog, steps = 10_000)) == 1 else
                BlockMacro(prog, [opt])
            )

            self.run_bb(
                program,
                prover = True,
            )

            if simple_term:
                self.assertIsNotNone(
                    self.machine.simple_termination)

                if not isinstance(program, str):
                    continue

                self.assert_marks(
                    0 if blank else prog_data[prog][0])
            else:
                self.assertTrue(
                    self.machine.infrul
                    or self.machine.spnout is not None)

    def _test_prover_est(self, prog_data: ProverEst):
        # pylint: disable = redefined-variable-type
        for prog, marks in prog_data.items():
            program: str | BlockMacro = prog

            if (opt := opt_block(prog, steps = 3_000)) > 1:
                program = BlockMacro(prog, [opt])

            self.run_bb(
                program,
                sim_lim = 10 ** 8,
                prover = True,
                normal = False,
            )

            self.assertIsNotNone(
                self.machine.simple_termination)

            result: int = self.machine.marks * opt

            if isinstance(marks, int):
                self.assertEqual(result, marks)
            else:
                digits, exp = marks

                if exp < 100_000:
                    self.assert_close(
                        result / 10 ** exp,
                        digits,
                        rel_tol = .54,
                    )
                else:
                    self.assertEqual(
                        exp,
                        int(log10(result)))

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

    def _test_macro_cycles(self, prog_data: MacroCycles):
        def macro_variations(base: str):
            # pylint: disable = invalid-name
            return (
                base,
                (k2 := BlockMacro(base, [2])),
                (k3 := BlockMacro(base, [3])),
                (bk := BacksymbolMacro(base, [1])),
                BacksymbolMacro(k2, [1]),
                BacksymbolMacro(k3, [1]),
                BacksymbolMacro(base, [1, 1]),
                BlockMacro(bk, [2]),
                BlockMacro(bk, [3]),
            )

        for program, cycleses in prog_data.items():
            if isinstance(program, tuple):
                prog, opt = program
                sim_lim = opt
            else:
                assert isinstance(program, str)
                prog, opt, sim_lim = program, 0, None

            self.assertEqual(
                len(cycleses),
                len(macros := macro_variations(prog)))

            for cycles, macro in zip(cycleses, macros):
                if cycles is not None and cycles > 10_000_000:  # no-coverage
                    continue

                self.run_bb(
                    macro,
                    sim_lim = (
                        20_000 if opt is None else
                        sim_lim  if sim_lim is not None else
                        10 ** 10),
                )

                self.assertEqual(
                    cycles,
                    None
                    if (self.machine.simple_termination is None
                        and opt is None) else
                    self.machine.cycles
                    if sim_lim is None else
                    self.machine.steps)

                if sim_lim is None and not isinstance(macro, str):
                    self.assertTrue(
                        len(macro) <= 60,
                        (len(macro), str(macro)))

    def _test_block_macro_steps(
            self,
            wraps: int,
            cells: int,
            rel_tol: float = .001,
            jump: int | None = None,
    ):
        for prog, steps in BLOCK_MACRO_STEPS.items():
            for wrap, cell in product(range(1, wraps), range(1, cells)):
                self.run_bb(
                    BlockMacro(prog, [cell] * wrap),
                )

                assert self.machine.simple_termination is not None

                self.assert_close(
                    self.machine.simple_termination,
                    steps / (cell ** wrap),
                    rel_tol = rel_tol,
                )

            if jump is None:                        # no-coverage
                continue

            for cell in range(jump, jump + cells):  # no-coverage
                self.run_bb(
                    BlockMacro(prog, [cell]),
                )

                assert self.machine.simple_termination is not None

                self.assert_close(
                    self.machine.simple_termination,
                    steps / cell,
                    rel_tol = rel_tol,
                )


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
            QUASIHALT,
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
            prover = True,
            analyze = False,
        )

        self.run_bb(
            "1RB 2RA 2RC  1LC 1R_ 1LA  1RA 2LB 1LC",
            prover = True,
        )

        self.run_bb(
            BacksymbolMacro("1RB 2LA 1RA 1RA  1LB 1LA 3RB 1R_", [2]),
            prover = True,
        )

    def test_undefined(self):
        for sequence in UNDEFINED.values():
            for partial, (step, slot) in sequence.items():
                self.run_bb(partial, normal = False)

                self.assertEqual(
                    (step, (str_st(slot[0]), int(slot[1]))),
                    self.machine.undfnd)

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
                    BacksymbolMacro(prog, [back] * back))

                self.assertIsNotNone(
                    self.machine.simple_termination)

    def test_backsymbol_required(self):
        prog = "1RB 0LC  1LC 0RC  1LA 0LC"

        self.run_bb(
            prog,
            sim_lim = 100,
            prover = True,
        )

        self.assertIsNone(
            self.machine.infrul)

        self.run_bb(
            BacksymbolMacro(prog, [1]),
            sim_lim = 100,
            prover = True,
        )

        self.assertIsNotNone(
            self.machine.infrul)

    def test_prover_false_positive(self):
        self.run_bb(
            "1RB 0RD  1LC 0RA  1LA 1LB  1R_ 0RC",
            prover = True,
            normal = False,
        )

        self.assertEqual(
            self.machine.halted,
            159)

        self.run_bb(
            BacksymbolMacro(
                "1RB 0RD  1LC 0RA  1LA 1LB  1R_ 0RC", [1]),
            prover = True,
        )

        self.assertIsNone(
            self.machine.halted)

    def test_wrong_block(self):
        prog = "1RB 0LA  1RC ...  1LD 0RC  0LA 1LD"

        self.run_bb(
            BlockMacro(prog, [2]),
            prover = True,
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
                BlockMacro(prog, [block]),
                prover = True,
                sim_lim = 8806,
            )

            self.assertIsNotNone(
                self.machine.xlimit)

        self.run_bb(
            BlockMacro(prog, [4]),
            prover = True,
            sim_lim = 8807,
        )

        self.assertIsNotNone(
            self.machine.infrul)


class Slow(TuringTest):  # no-coverage
    @skip('')
    def test_halt(self):
        self._test_halt(HALT_SLOW)

    def test_spinout(self):
        for prog in CANT_SPIN_OUT_SLOW:
            self.assert_cant_spin_out(prog)

        self._test_spinout(SPINOUT_SLOW)
        self._test_spinout(SPINOUT_BLANK_SLOW, blank = True)

    def test_recur(self):
        self._test_recur(RECUR_SLOW, quick = False)

    def test_block_macro_steps(self):
        self._test_block_macro_steps(
            wraps = 8,
            cells = 9,
            rel_tol = 1.0,
            jump = 2_000,
        )

    def test_macro_cycles(self):
        self._test_macro_cycles(MACRO_CYCLES_SLOW)
