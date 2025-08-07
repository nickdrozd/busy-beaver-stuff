# ruff: noqa: F405
import json
import os
import re
from collections import defaultdict
from itertools import product
from math import isclose, log10
from typing import TYPE_CHECKING
from unittest import TestCase, expectedFailure, skip, skipUnless

from test.lin_rec import (
    LinRecSampler,
    StrictLinRecMachine,
    run_loose_linrec_machine,
)
from test.prog_data import *  # noqa: F403
from test.test_num import assert_num_counts, clear_caches
from tm.machine import (
    Machine,
    show_number,
    show_slot,
    term_or_rec,
)
from tm.macro import (
    MacroProg,
    opt_block,
    show_comp,
    tcompile,
)
from tm.reason import (
    BackwardResult,
    cant_blank,
    cant_halt,
    cant_spin_out,
    cps_cant_blank,
    cps_cant_halt,
    cps_cant_spin_out,
    ctl_cant_blank,
    ctl_cant_halt,
    ctl_cant_spin_out,
    segment_cant_blank,
    segment_cant_halt,
    segment_cant_spin_out,
)
from tm.rust_stuff import (
    MachineResult,
    run_quick_machine,
)
from tools import get_params
from tools.graph import Graph
from tools.instr_seq import instr_seq
from tools.normalize import normalize

if TYPE_CHECKING:
    from collections.abc import Mapping
    from typing import Any

    from test.lin_rec import Tapes
    from tm.machine import Count
    from tm.reason import BackwardReasoner as BR

    BasicMachine = (
        Machine
        | MachineResult
        | StrictLinRecMachine
        | LinRecSampler
    )

RUN_SLOW = os.environ.get('RUN_SLOW')


CPS_LIMIT = 11
CTL_LIMIT = 300
REASON_LIMIT = 2_000
SEGMENT_LIMIT = 22


class TuringTest(TestCase):
    machine: BasicMachine

    maxDiff = None

    ########################################

    def assert_marks(self, marks: int):
        assert isinstance(
            self.machine,
            Machine | MachineResult)

        actual_marks = self.machine.marks

        try:
            self.assertEqual(actual_marks, marks)
        except AssertionError:
            try:
                self.assertEqual(
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

    def assert_no_init_blank(self) -> None:
        self.assertNotIn(0, self.machine.blanks)

    def assert_undefined(self, expected: tuple[int, str]):
        assert (undfnd := self.machine.undfnd) is not None
        step, slot = undfnd

        self.assertEqual(
            expected,
            (step, show_slot(slot)))

    ########################################

    def assert_normal(self, prog: str):
        if re.match(MOTHER, prog):
            return

        self.assertTrue(
            Graph(prog).is_normal,
            prog)

        self.assertTrue(
            prog == normalize(prog)
                or prog.startswith('0'),
            prog)

    def assert_connected(self, prog: str):
        self.assertTrue(
            Graph(prog).is_connected
                or prog in MODULAR,
            prog)

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

        if decomp and prog not in NONNORMAL:
            self.assertEqual(
                prog,
                show_comp(tcompile(prog)))

        _ = Machine(prog,  blocks = 2).run(sim_lim = 10)
        _ = Machine(prog, backsym = 1).run(sim_lim = 10)

    ########################################

    def assert_could_halt(self, prog: str):
        self.assert_could_halt_backward(prog)
        self.assert_could_halt_segment(prog)

    def assert_could_halt_backward(self, prog: str):
        if prog == "1RB ...  ... ...":
            return

        self.assertFalse(
            cant_halt(prog, depth = REASON_LIMIT).is_refuted(),
            f'halt false positive: "{prog}"')

    def assert_cant_halt_backward(self, prog: str, depth: int):
        if prog in CANT_HALT_FALSE_NEGATIVES:
            return

        self.assertTrue(
            cant_halt(prog, depth).is_refuted(),
            f'halt false negative: "{prog}"')

    def assert_could_halt_segment(self, prog: str):
        if prog == "1RB ...  ... ...":
            return

        self.assertFalse(
            segment_cant_halt(prog, segs = SEGMENT_LIMIT).is_refuted(),
            f'segment halt false positive: "{prog}"')

    def assert_cant_halt_segment(self, prog: str, segs: int):
        if prog in SEGMENT_FALSE_NEGATIVES['halt']:
            return

        self.assertTrue(
            segment_cant_halt(prog, segs).is_settled(),
            f'segment halt false negative: "{prog}"')

    def assert_could_halt_cps(self, prog: str):
        if prog == '1RB ...  ... ...':
            return

        self.assertFalse(
            cps_cant_halt(prog, CPS_LIMIT))

    def assert_cant_halt_cps(self, prog: str, segs: int):
        if prog in CPS_FALSE_NEGATIVES['halt']:
            return

        self.assertTrue(
            cps_cant_halt(prog, segs))

    def assert_could_halt_ctl(self, prog: str):
        if prog == '1RB ...  ... ...':
            return

        self.assertFalse(
            ctl_cant_halt(prog, CTL_LIMIT))

    def assert_cant_halt_ctl(self, prog: str, segs: int):
        if prog in CTL_FALSE_NEGATIVES['halt']:
            return

        self.assertTrue(
            ctl_cant_halt(prog, segs), prog)

    ########################################

    def assert_could_blank(self, prog: str):
        self.assert_could_blank_backward(prog)
        self.assert_could_blank_segment(prog)

    def assert_could_blank_backward(self, prog: str):
        self.assertFalse(
            cant_blank(prog, depth = REASON_LIMIT).is_refuted(),
            f'blank false positive: "{prog}"')

    def assert_could_blank_segment(self, prog: str):
        self.assertFalse(
            segment_cant_blank(prog, segs = SEGMENT_LIMIT).is_refuted(),
            f'segment blank false positive: "{prog}"')

    def assert_cant_blank_backward(self, prog: str, depth: int):
        if (prog in CANT_BLANK_FALSE_NEGATIVES
                or Machine(prog).run(sim_lim = 30).blanks):
            return

        self.assertTrue(
            cant_blank(prog, depth).is_refuted(),
            f'blank false negative: "{prog}"')

    def assert_cant_blank_segment(self, prog: str, segs: int):
        self.assertTrue(
            segment_cant_blank(prog, segs).is_settled(),
            f'segment blank false negative: "{prog}"')

    def assert_could_blank_cps(self, prog: str):
        self.assertFalse(
            cps_cant_blank(prog, CPS_LIMIT))

    def assert_cant_blank_cps(self, prog: str, segs: int):
        if prog in CPS_FALSE_NEGATIVES['blank']:
            return

        self.assertTrue(
            cps_cant_blank(prog, segs))

    def assert_could_blank_ctl(self, prog: str):
        self.assertFalse(
            ctl_cant_blank(prog, CTL_LIMIT))

    def assert_cant_blank_ctl(self, prog: str, segs: int):
        if prog in CTL_FALSE_NEGATIVES['blank']:
            return

        self.assertTrue(
            ctl_cant_blank(prog, segs))

    ########################################

    def assert_could_spin_out(self, prog: str):
        self.assert_could_spin_out_backward(prog)
        self.assert_could_spin_out_segment(prog)

    def assert_could_spin_out_backward(self, prog: str):
        self.assertFalse(
            cant_spin_out(prog, depth = REASON_LIMIT).is_refuted(),
            f'spin out false positive: "{prog}"')

    def assert_could_spin_out_segment(self, prog: str):
        self.assertFalse(
            segment_cant_spin_out(prog, segs = SEGMENT_LIMIT).is_refuted(),
            f'segment spin out false positive: "{prog}"')

    def assert_cant_spin_out_backward(self, prog: str, depth: int):
        if prog in CANT_SPIN_OUT_FALSE_NEGATIVES:
            return

        self.assertTrue(
            cant_spin_out(prog, depth).is_refuted(),
            f'spin out false negative: "{prog}"')

    def assert_cant_spin_out_segment(self, prog: str, segs: int):
        if prog in SEGMENT_FALSE_NEGATIVES['spinout']:
            assert prog not in SEGMENT_STEPS['spinout']
            return

        self.assertTrue(
            segment_cant_spin_out(prog, segs).is_settled(),
            f'segment spin out false negative: "{prog}"')

    def assert_could_spin_out_cps(self, prog: str):
        self.assertFalse(
            cps_cant_spin_out(prog, CPS_LIMIT))

    def assert_cant_spin_out_cps(self, prog: str, segs: int):
        if prog in CPS_FALSE_NEGATIVES['spinout']:
            return

        self.assertTrue(
            cps_cant_spin_out(prog, segs))

    def assert_could_spin_out_ctl(self, prog: str):
        self.assertFalse(
            ctl_cant_spin_out(prog, CTL_LIMIT))

    def assert_cant_spin_out_ctl(self, prog: str, segs: int):
        if prog in CTL_FALSE_NEGATIVES['spinout']:
            return

        self.assertTrue(
            ctl_cant_spin_out(prog, segs))

########################################

BACKWARD_REASONERS: dict[str, BR] = {
    "halt": cant_halt,
    "blank": cant_blank,
    "spinout": cant_spin_out,
}

class Reason(TuringTest):
    def test_halt(self):
        for prog in HALTERS:
            self.assert_could_halt_backward(prog)

        for prog in NONHALTERS:
            self.assert_cant_halt_backward(prog, 200)

    def test_spinout(self):
        for prog in SPINNERS - MACRO_SPINOUT:
            self.assert_could_spin_out_backward(prog)

        for prog in NONSPINNERS:
            self.assert_cant_spin_out_backward(prog, 256)

    def test_blank(self):
        for prog in NONBLANKERS:
            self.assert_cant_blank_backward(prog, 1331)

        for prog in BLANKERS:
            self.assert_could_blank_backward(prog)

    def test_recur(self):
        for prog in RECURS | INFRUL:
            self.assert_cant_halt_backward(prog, 115)
            self.assert_cant_spin_out_backward(prog, 256)

            if prog not in BLANKERS:
                self.assert_cant_blank_backward(prog, 1331)

    def test_false_negatives(self):
        results: dict[Goal, BackwardCats] = {
            'halt': defaultdict(set),
            'blank': defaultdict(set),
            'spinout': defaultdict(set),
        }

        for term, cats in BACKWARD_FALSE_NEGATIVES.items():
            term_results = results[term]
            reasoner = BACKWARD_REASONERS[term]

            for progs in cats.values():
                for prog in progs:
                    result = str(reasoner(prog, REASON_LIMIT))

                    term_results[result].add(prog)  # type: ignore[index]

        def assert_counts(data: dict[Goal, BackwardCats]):
            counts = {
                term: {
                    cat: len(progs)
                    for cat, progs in cats.items()
                }
                for term, cats in data.items()
            }

            if counts != BACKWARD_FALSE_NEGATIVES_COUNTS:
                print(json.dumps(counts, indent = 4))
                raise AssertionError

        assert_counts(results)

        if results != BACKWARD_FALSE_NEGATIVES:
            dump = {
                term: {
                    cat: sorted(sorted(progs), key = len)
                    for cat, progs in cats.items()
                }
                for term, cats in results.items()
            }

            print(json.dumps(dump, indent = 4))

            raise AssertionError

        assert_counts(BACKWARD_FALSE_NEGATIVES)

        _ = self

    def test_cryptids(self):
        for cryptid in CRYPTIDS:
            self.assert_could_halt(cryptid)

        for ext in branch_last(MOTHER):
            self.assert_could_blank_backward(ext)
            self.assert_could_spin_out_backward(ext)

            if ext in INFRUL:
                self.assert_cant_blank_segment(ext, 14)
                self.assert_cant_spin_out_segment(ext, 14)

            if ext in BLANKERS:
                self.assert_could_blank_segment(ext)
            elif ext in SPINNERS:
                self.assert_could_spin_out_segment(ext)
                self.assert_cant_blank_segment(ext, 14)

        for bigfoot in BIGFOOT:
            for ext in branch_last(bigfoot):
                self.assert_cant_blank_backward(ext, 7)

        for hydra in HYDRA:
            self.assert_could_blank(hydra)
            self.assert_cant_spin_out_backward(hydra, 0)

        for hydra in ANTIHYDRA:
            self.assert_cant_blank_backward(hydra, 1)
            self.assert_cant_spin_out_backward(hydra, 0)

    def test_steps(self):
        for cat, data in BACKWARD_STEPS.items():
            cant_reach = BACKWARD_REASONERS[cat]

            for prog, steps in data.items():
                self.assertEqual(
                    cant_reach(prog, steps).step,
                    steps)

                self.assertIsInstance(
                    cant_reach(prog, steps - 1),
                    BackwardResult.step_limit)

    def test_reason_only(self):
        for prog in REASON_ONLY:
            self.assert_cant_halt_backward(prog, REASON_LIMIT)
            self.assert_cant_blank_backward(prog, REASON_LIMIT)
            self.assert_cant_spin_out_backward(prog, REASON_LIMIT)

            self.assertNotIn(prog, INFRUL)

    def test_omnireasonable(self):
        assert set(OMNIREASONABLE) <= INFRUL | RECURS

        for prog, (halt, blank, spin) in OMNIREASONABLE.items():
            self.assertEqual(
                1 + halt,
                cant_halt(prog, REASON_LIMIT).step)

            self.assertEqual(
                1 + blank,
                cant_blank(prog, REASON_LIMIT).step)

            self.assertEqual(
                1 + spin,
                cant_spin_out(prog, REASON_LIMIT).step)

    def test_simple(self):
        for prog in HALTERS | SPINNERS | RECURS | INFRUL:
            self.assert_simple(prog)


class Segment(TuringTest):
    def test_halt(self):
        for prog in HALTERS:
            self.assert_could_halt_segment(prog)

        for prog in NONHALTERS:
            self.assert_cant_halt_segment(prog, SEGMENT_LIMIT)

    def test_spinout(self):
        for prog in SPINNERS - MACRO_SPINOUT:
            self.assert_could_spin_out_segment(prog)

        for prog in NONSPINNERS:
            self.assert_cant_spin_out_segment(prog, 26)

    def test_false_negatives(self):
        counts = {
            cat: len(progs)
            for cat, progs in SEGMENT_FALSE_NEGATIVES.items()
        }

        self.assertEqual(
            SEGMENT_FALSE_NEGATIVE_COUNTS,
            counts,
            json.dumps(counts, indent = 4))

        for prog in SEGMENT_FALSE_NEGATIVES['halt']:
            self.assert_could_halt_segment(prog)

        for prog in SEGMENT_FALSE_NEGATIVES['spinout']:
            self.assert_could_spin_out_segment(prog)

    def test_steps(self):
        cant_reaches = {
            "halt": segment_cant_halt,
            "spinout": segment_cant_spin_out,
        }

        for cat, data in SEGMENT_STEPS.items():
            cant_reach = cant_reaches[cat]

            for prog, steps in data.items():
                self.assertEqual(
                    steps,
                    cant_reach(prog, steps).step)

                self.assertFalse(
                    cant_reach(prog, steps - 1).is_settled())

    def test_omnireasonable(self):
        for prog in OMNIREASONABLE:
            self.assert_cant_halt_segment(prog, SEGMENT_LIMIT)
            self.assert_cant_spin_out_segment(prog, SEGMENT_LIMIT)


def branch_last(prog: str) -> list[str]:
    comp = tcompile(prog)

    states, colors = get_params(prog)

    all_slots = set(product(range(states), range(colors)))

    open_slots = all_slots - set(comp.keys())

    assert len(open_slots) == 1

    open_slot = next(iter(open_slots))

    all_instrs = product(
        range(colors),
        (True, False),
        range(states),
    )

    return [
        show_comp(comp | { open_slot : instr })
        for instr in all_instrs
    ]

########################################

class Cps(TuringTest):
    def test_halt(self):
        for prog in HALTERS:
            self.assert_could_halt_cps(prog)

        for prog in NONHALTERS:
            self.assert_cant_halt_cps(prog, 11)

    def test_blank(self):
        for prog in BLANKERS:
            self.assert_could_blank_cps(prog)

        for prog in NONBLANKERS:
            self.assert_cant_blank_cps(prog, 11)

    def test_spinout(self):
        for prog in SPINNERS - MACRO_SPINOUT:
            self.assert_could_spin_out_cps(prog)

        for prog in NONSPINNERS:
            self.assert_cant_spin_out_cps(prog, 11)

    def test_false_negatives(self):
        counts = {
            cat: len(progs)
            for cat, progs in CPS_FALSE_NEGATIVES.items()
        }

        self.assertEqual(
            CPS_FALSE_NEGATIVE_COUNTS,
            counts,
            json.dumps(counts, indent = 4))

        for prog in CPS_FALSE_NEGATIVES['halt']:
            self.assert_could_halt_cps(prog)

        for prog in CPS_FALSE_NEGATIVES['blank']:
            self.assert_could_blank_cps(prog)

        for prog in CPS_FALSE_NEGATIVES['spinout']:
            self.assert_could_spin_out_cps(prog)

########################################

class Ctl(TuringTest):
    def test_halt(self):
        for prog in HALTERS:
            self.assert_could_halt_ctl(prog)

        for prog in NONHALTERS:
            self.assert_cant_halt_ctl(prog, 163)

    def test_blank(self):
        for prog in BLANKERS:
            self.assert_could_blank_ctl(prog)

        for prog in NONBLANKERS:
            self.assert_cant_blank_ctl(prog, 61)

    def test_spinout(self):
        for prog in SPINNERS - MACRO_SPINOUT:
            self.assert_could_spin_out_ctl(prog)

        for prog in NONSPINNERS:
            self.assert_cant_spin_out_ctl(prog, 118)

    def test_false_negatives(self):
        counts = {
            cat: len(progs)
            for cat, progs in CTL_FALSE_NEGATIVES.items()
        }

        self.assertEqual(
            CTL_FALSE_NEGATIVE_COUNTS,
            counts,
            json.dumps(counts, indent = 4))

        for prog in CTL_FALSE_NEGATIVES['halt']:
            self.assert_could_halt_ctl(prog)

        for prog in CTL_FALSE_NEGATIVES['blank']:
            self.assert_could_blank_ctl(prog)

        for prog in CTL_FALSE_NEGATIVES['spinout']:
            self.assert_could_spin_out_ctl(prog)

########################################

class Simple(TuringTest):
    machine: MachineResult

    def run_bb(
            self,
            prog: str,
            sim_lim: int = 100_000_000,
            analyze: bool = True,
    ):
        print(prog)

        if analyze:
            self.analyze(prog, normal = True, decomp = True)

        self.machine = run_quick_machine(prog, sim_lim)

    def test_halt(self):
        self._test_halt(HALT)

    def test_spinout(self):
        self._test_spinout(SPINOUT)
        self._test_spinout(SPINOUT_BLANK, blank = True)

    def test_init_blank(self):
        for prog, steps in INIT_BLANK.items():
            self.run_bb(prog, sim_lim = steps, analyze = False)
            self.assertEqual(self.machine.blanks[0], steps)

    def test_instr_seqs(self):
        self._test_instr_seqs(INSTR_SEQS)

    def test_blank_after_tree(self):
        self._test_instr_seqs(BLANK_AFTER_TREE)

        for prog in BLANK_AFTER_TREE:
            self.run_bb(prog, sim_lim = 100)

            self.assertTrue(
                self.machine.blanks)

    def _test_instr_seqs(self, instr_seqs: InstrSeqs):
        for sequence in instr_seqs.values():
            for partial, expected in sequence.items():
                self.run_bb(partial, analyze = False)

                self.assert_undefined(expected)

        for prog, sequence in instr_seqs.items():
            self.assertEqual(
                sequence,
                {
                    partial: (step, show_slot(slot))
                    for partial, step, slot in
                    instr_seq(prog)
                },
            )

    def test_limited(self):
        for goal, limited in LIMITED.items():
            for instrs, progs in limited.items():
                for prog, steps in progs.items():
                    self.assertEqual(instrs, len(tcompile(prog)))

                    self.run_bb(prog, analyze = False)

                    match goal:
                        case 'blank':
                            self.assertEqual(
                                steps,
                                min(self.machine.blanks.values()))

                        case 'halt':
                            assert (undfnd := self.machine.undfnd)

                            self.assertEqual(
                                steps,
                                undfnd[0])

                        case 'spinout':
                            self.assertEqual(
                                steps,
                                self.machine.spnout)

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
                        self.assert_cant_blank_backward(prog, 19)

            else:
                self.assert_marks(0)
                self.assertEqual(steps, max(blanks.values()))
                self.assertEqual(
                    marks, {chr(blank + 65) for blank in blanks})
                self.assert_could_blank(prog)
                self.assert_no_init_blank()

            if self.machine.undfnd is not None:
                self.assert_could_halt(prog)
                self.assert_cant_spin_out_backward(prog, 6)

            else:
                self.assert_could_spin_out(prog)
                self.assert_cant_halt_backward(prog, 46)

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

########################################

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

    def test_lr_negatives(self):
        steps = 1_000

        for prog in LR_NEGATIVES:
            self.assertFalse(
                term_or_rec(prog, steps),
                prog)

            self.assertIsNotNone(
                run_loose_linrec_machine(prog, steps).xlimit)

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
                period, qsihlt_diff = period  # noqa: PLW2901
            else:
                qsihlt_diff = 0

            self.prog = prog

            self.assertGreater(period, 1)

            if blank:
                self.assert_could_blank(prog)
            else:
                assert steps is not None
                self.verify_lin_rec(prog, steps, period)

            if not quick or period > 2000:
                print(prog)

                self.assertTrue(
                    term_or_rec(
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

            self.assert_no_init_blank()

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

            self.assertIsNotNone(
                run_loose_linrec_machine(prog, 1_000_000).infrul)

            self.assertTrue(
                term_or_rec(prog, 1_000_000), prog)

    def test_infrul(self):
        for prog in INFRUL - BLANKERS:
            self.assertFalse(term_or_rec(prog, 10_000))

        assert not INFRUL & RECURS

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
                steps - 1      : None,
                steps          : None,
                steps + 1      : None,
                recur - 1      : None,
                recur          : None,
                recur + 1      : None,
                recur + period : None,
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

########################################

class RunProver(TuringTest):
    machine: Machine

    def assert_spinout(self) -> None:
        self.assertIsNotNone(
            self.machine.spnout)

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


class Macro(RunProver):
    def assert_macro_cells(self, cells: int):
        assert isinstance(
            macro := self.machine.program,
            MacroProg)

        self.assertEqual(
            macro.cells,
            cells)

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

        self.run_bb(
            "1RB 2LA 3RA 0LA  1LA 2RA 0RB ...",
            opt_macro = 4_000,
            watch_tape = True)

        with self.assertRaises(OverflowError):
            self.run_bb(
                "1RB 2LA 2RB 3RA  1LB 1RA 3LB 0RB",
                opt_macro = 300,
                watch_tape = True)

    def test_backsym_overflow(self):
        self.run_bb(
            "1RB 1LA ... 0RB  2LA 3RB 1LB 0RA",
            backsym = 1,
            sim_lim = 2422,
        )

        self.assertEqual(
            self.machine.infrul,
            133219)

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

    def test_backsym_no_help(self):
        self.run_bb(
            "1RB 1LA  0LB 1RC  0LA 0RC",
            backsym = 1,
            sim_lim = 3,
        )

        self.assertIsNotNone(
            self.machine.xlimit)


class Prover(RunProver):
    machine: Machine

    def assert_mult_rules(self) -> None:
        self.assertTrue(any(
            isinstance(diff, tuple)
            for rules in self.machine.prover.rules.values()
            for _, rule in rules
            for diff in rule.values()))

    def assert_close(
            self,
            this: int,
            that: int | float,  # noqa: PYI041
            rel_tol: float,
    ):
        self.assertTrue(
            isclose(
                this,
                that,
                rel_tol = rel_tol,
            )
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
            "1RB 2RA 2RC  1LC ... 1LA  1RA 2LB 1LC",
        )

        self.run_bb(
            "1RB 2LA 1RA 1RA  1LB 1LA 3RB ...",
            backsym = 2,
        )

    def _test_prover(  # type: ignore[explicit-any]
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
                analyze = False,
            )

            if simple_term:
                self.assertIsNotNone(
                    self.machine.simple_termination)

                if not isinstance(self.machine.program, dict):
                    continue

                self.assert_marks(
                    0 if blank else prog_data[prog][0])

                if blank:
                    self.assert_no_init_blank()
            else:
                self.assertTrue(
                    self.machine.infrul
                    or self.machine.cfglim
                    or self.machine.spnout is not None)

            self.assertIsInstance(
                self.machine.rulapp,
                int)

    def _test_prover_est(self, prog_data: ProverEst):
        for prog, marks in prog_data.items():
            if prog in FAILURES:
                continue

            if prog in PROVER_HALT_TOO_SLOW and not RUN_SLOW:
                continue

            self.run_bb(
                prog,
                sim_lim = 10 ** 8,
                opt_macro = 3_000,
                backsym = REQUIRES_BACKSYM.get(prog),
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
                    isinstance(
                        macro := self.machine.program,
                        MacroProg)):
                result *= macro.cells

            if isinstance(marks, int):
                self.assertEqual(result, marks)
            elif isinstance(marks, str):
                self.assertTrue(
                    str(result).startswith(marks),
                    f'    "{prog}": "{result}",')
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
                self.assert_no_init_blank()
            else:
                self.assert_cant_blank_backward(prog, 14)

            if self.machine.undfnd is not None:
                self.assert_cant_spin_out_backward(prog, 5)
                self.assert_could_halt(prog)
            else:
                self.assert_cant_halt_backward(prog, 0)

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

    def test_infrul(self):
        for prog in INFRUL:
            self.run_bb(
                prog,
                normal = False,
                **(
                    {'opt_macro': 3_000}  # type: ignore[arg-type]
                    if (blocks := MACRO_FAILURES.get(prog)) is None else
                    {'blocks': blocks}
                ),
            )

            self.assertIsNotNone(
                self.machine.infrul)

        for prog in REASON_ONLY:
            self.run_bb(
                prog,
                opt_macro = 2_400,
                sim_lim = 10_000,
                normal = False)

            self.assertIsNone(
                self.machine.infrul)

    def _test_reason_steps(self, steps: dict[Goal, dict[str, int]]):
        for cat, progs in steps.items():
            for prog in progs:
                if prog in PROVER_FAILURES:
                    continue

                self.run_bb(
                    prog,
                    sim_lim = 2_000,
                    analyze = False)

                match cat:
                    case 'halt':
                        self.assertIsNone(
                            self.machine.undfnd,
                            f'"{prog}"')
                    case 'spinout':
                        self.assertIsNone(
                            self.machine.spnout,
                            f'"{prog}"')
                    case 'blank':
                        self.assertFalse(
                            self.machine.blanks,
                            f'"{prog}"')

    def test_backwards_steps(self):
        self._test_reason_steps(BACKWARD_STEPS)

    def test_segment_steps(self):
        self._test_reason_steps(SEGMENT_STEPS)

    def test_rule_limit(self):
        for prog, reason in RULE_LIMIT.items():
            self.run_bb(
                prog,
                print_prog = prog not in FAILURES,
                normal = False,
                opt_macro = 1600,
            )

            self.assertTrue(
                str(self.machine.limrul).startswith(reason))

    def test_bad_number(self):
        self.run_bb(
            "1RB 2LC 1RA  1LA 1LC 2RB  0LB 2LA 0RC")

        with self.assertRaises(ValueError):
            print(self.machine)

    def test_prover_false_positive(self):
        prog = "1RB 0LD  1RC 0RF  1LC 1LA  0LE ...  1LA 0RB  0RC 0RE"

        for backsym in range(0, 7):  # noqa: PIE808
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

    def test_algebra(self):
        clear_caches()

        show = False

        for term, progs in ALGEBRA.items():
            if show:
                print(f'    "{term}": {{')

            for prog, (cycles, est, string, rulapp) in progs.items():
                self.run_bb(
                    prog,
                    opt_macro = 4000,
                    analyze = False,
                    backsym = REQUIRES_BACKSYM.get(prog),
                    print_prog = not show,
                )

                assert self.machine.is_algebraic

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

        assert_num_counts(
            ALGEBRA_NUM_COUNTS)
