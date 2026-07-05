# ruff: noqa: F405
import json
from collections import defaultdict
from typing import TYPE_CHECKING
from unittest import TestCase

from test.prog_data import *  # noqa: F403
from tm.rust_stuff import (
    bkw_cant_blank,
    bkw_cant_halt,
    bkw_cant_spinout,
    bkw_cant_twostep,
    bkw_cant_zloop,
    cps_cant_blank,
    cps_cant_halt,
    cps_cant_quasihalt,
    cps_cant_spinout,
    ctl_cant_blank,
    ctl_cant_halt,
    ctl_cant_spinout,
    far_cant_blank,
    far_cant_halt,
    far_cant_spinout,
    graph_cant_blank,
    graph_cant_halt,
    graph_cant_quasihalt,
    graph_cant_spinout,
    segment_cant_halt,
    segment_cant_spinout,
)
from tools.graph import Graph as GraphPy

if TYPE_CHECKING:
    from collections.abc import Callable

    from tm.rust_stuff import BackwardResult

    type BackwardReasoner = Callable[
        [str, int],
        BackwardResult,
    ]

########################################

CPS_LIMIT = 33
CTL_LIMIT = 3_200
BKW_LIMIT = 3_000
SEG_LIMIT = 22

########################################

def get_holdouts(path: str) -> set[str]:
    holdouts: set[str] = set()

    with open(f'test/data/holdouts/{path}.prog') as progs:
        holdouts.update(prog.strip() for prog in progs)

    return holdouts


HALT_HOLDOUTS = get_holdouts('halt')
BLANK_HOLDOUTS = get_holdouts('blank')
SPINOUT_HOLDOUTS = get_holdouts('spinout')
QUASIHALT_HOLDOUTS = get_holdouts('quasihalt')

########################################

BACKWARD_REASONERS: dict[Goal, BackwardReasoner] = {
    "halt": bkw_cant_halt,
    "blank": bkw_cant_blank,
    "spinout": bkw_cant_spinout,
    "twostep": bkw_cant_twostep,
    "zloop": bkw_cant_zloop,
}

class Backward(TestCase):
    def assert_could_halt_backward(self, prog: str):
        self.assertFalse(
            bkw_cant_halt(prog, steps = BKW_LIMIT).is_refuted(),
            f'halt false positive: "{prog}"')

    def assert_could_blank_backward(self, prog: str):
        self.assertFalse(
            bkw_cant_blank(prog, steps = BKW_LIMIT).is_refuted(),
            f'blank false positive: "{prog}"')

    def assert_could_spinout_backward(self, prog: str):
        self.assertFalse(
            bkw_cant_spinout(prog, steps = BKW_LIMIT).is_refuted(),
            f'spinout false positive: "{prog}"')

    def assert_could_twostep_backward(self, prog: str):
        self.assertFalse(
            bkw_cant_twostep(prog, steps = BKW_LIMIT).is_refuted(),
            f'twostep false positive: "{prog}"')

    def assert_could_zloop_backward(self, prog: str):
        self.assertFalse(
            bkw_cant_zloop(prog, steps = BKW_LIMIT).is_refuted(),
            f'zloop false positive: "{prog}"')

    def assert_cant_halt_backward(self, prog: str, depth: int):
        if prog in BACKWARD_CANT_HALT_FALSE_NEGATIVES:
            return

        self.assertTrue(
            bkw_cant_halt(prog, depth).is_refuted(),
            f'halt false negative: "{prog}"')

    def assert_cant_blank_backward(self, prog: str, depth: int):
        if prog in BACKWARD_CANT_BLANK_FALSE_NEGATIVES:
            return

        self.assertTrue(
            bkw_cant_blank(prog, depth).is_refuted(),
            f'blank false negative: "{prog}"')

    def assert_cant_spinout_backward(self, prog: str, depth: int):
        if prog in BACKWARD_CANT_SPINOUT_FALSE_NEGATIVES:
            return

        if 2 < prog.count('...'):
            return

        self.assertTrue(
            bkw_cant_spinout(prog, depth).is_refuted(),
            f'spinout false negative: "{prog}"')

    def assert_cant_twostep_backward(self, prog: str, depth: int):
        if prog in BACKWARD_CANT_TWOSTEP_FALSE_NEGATIVES:
            return

        self.assertTrue(
            bkw_cant_twostep(prog, depth).is_refuted(),
            f'twostep false negative: "{prog}"')

    def assert_cant_zloop_backward(self, prog: str, depth: int):
        if prog in BACKWARD_CANT_ZLOOP_FALSE_NEGATIVES:
            return

        self.assertTrue(
            bkw_cant_zloop(prog, depth).is_refuted(),
            f'zloop false negative: "{prog}"')

    ########################################

    def test_halt(self):
        for prog in HALTERS:
            self.assert_could_halt_backward(prog)

        for prog in NONHALTERS:
            self.assert_cant_halt_backward(prog, 200)

    def test_spinout(self):
        for prog in SPINNERS - MACRO_SPINOUT:
            self.assert_could_spinout_backward(prog)

        for prog in NONSPINNERS:
            self.assert_cant_spinout_backward(prog, 256)

    def test_blank(self):
        for prog in NONBLANKERS:
            self.assert_cant_blank_backward(prog, 1331)

        for prog in BLANKERS:
            self.assert_could_blank_backward(prog)

    def test_twostep(self):
        for prog in HALTERS | SPINNERS:
            self.assert_cant_twostep_backward(prog, 0)

        for prog in RECURS - TWOSTEPPERS:
            self.assert_cant_twostep_backward(prog, 0)

        for prog in TWOSTEPPERS:
            self.assert_could_twostep_backward(prog)

    def test_zloop(self):
        for prog in HALTERS | SPINNERS:
            self.assert_cant_zloop_backward(prog, 2)

        for prog in RECURS - ZLOOPERS - ZLOOPY:
            self.assert_cant_zloop_backward(prog, 4)

        for prog in ZLOOPERS:
            self.assert_could_zloop_backward(prog)

    def test_recur(self):
        for prog in RECURS | INFRUL:
            self.assert_cant_halt_backward(prog, 115)
            self.assert_cant_spinout_backward(prog, 256)

            if prog not in BLANKERS:
                self.assert_cant_blank_backward(prog, 1331)

    def test_holdouts(self):
        for prog in HALT_HOLDOUTS:
            self.assert_could_halt_backward(prog)

        for prog in BLANK_HOLDOUTS:
            self.assert_could_blank_backward(prog)

        for prog in SPINOUT_HOLDOUTS:
            self.assert_could_spinout_backward(prog)

    def test_false_negatives(self):
        results: dict[Goal, BackwardCats] = {
            'halt': defaultdict(set),
            'blank': defaultdict(set),
            'zloop': defaultdict(set),
            'spinout': defaultdict(set),
            'twostep': defaultdict(set),
        }

        for term, cats in BACKWARD_FALSE_NEGATIVES.items():
            term_results = results[term]
            reasoner = BACKWARD_REASONERS[term]

            for progs in cats.values():
                for prog in progs:
                    result = str(reasoner(prog, BKW_LIMIT))

                    term_results[result].add(prog)  # type: ignore[index]  # ty: ignore[invalid-argument-type]

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
                for cat, res in data.items():
                    if progs := res['refuted']:  # type: ignore[index]  # ty: ignore[invalid-argument-type]
                        print(cat)
                        for prog in progs:
                            print(prog)
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

    def test_steps(self):
        for cat, data in BACKWARD_STEPS.items():
            bkw_cant_reach = BACKWARD_REASONERS[cat]

            for prog, steps in data.items():
                self.assertEqual(
                    bkw_cant_reach(prog, BKW_LIMIT).step,
                    steps)


class Segment(TestCase):
    def assert_could_halt_segment(self, prog: str):
        self.assertFalse(
            segment_cant_halt(prog, segs = SEG_LIMIT).is_refuted(),
            f'segment halt false positive: "{prog}"')

    def assert_cant_halt_segment(self, prog: str, segs: int):
        if prog in SEGMENT_FALSE_NEGATIVES['halt']:
            return

        self.assertTrue(
            segment_cant_halt(prog, segs).is_settled(),
            f'segment halt false negative: "{prog}"')

    def assert_could_spinout_segment(self, prog: str):
        self.assertFalse(
            segment_cant_spinout(prog, segs = SEG_LIMIT).is_refuted(),
            f'segment spinout false positive: "{prog}"')

    def assert_cant_spinout_segment(self, prog: str, segs: int):
        if prog in SEGMENT_FALSE_NEGATIVES['spinout']:
            return

        self.assertTrue(
            segment_cant_spinout(prog, segs).is_settled(),
            f'segment spinout false negative: "{prog}"')

    def test_halt(self):
        for prog in HALTERS:
            self.assert_could_halt_segment(prog)

        for prog in NONHALTERS:
            self.assert_cant_halt_segment(prog, SEG_LIMIT)

    def test_spinout(self):
        for prog in SPINNERS - MACRO_SPINOUT:
            self.assert_could_spinout_segment(prog)

        for prog in NONSPINNERS:
            self.assert_cant_spinout_segment(prog, 26)

    def test_holdouts(self):
        for prog in HALT_HOLDOUTS:
            self.assert_could_halt_segment(prog)

        for prog in SPINOUT_HOLDOUTS:
            self.assert_could_spinout_segment(prog)

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
            self.assert_could_spinout_segment(prog)

    def test_steps(self):
        bkw_cant_reaches = {
            "halt": segment_cant_halt,
            "spinout": segment_cant_spinout,
        }

        for cat, data in SEGMENT_STEPS.items():
            bkw_cant_reach = bkw_cant_reaches[cat]

            for prog, steps in data.items():
                self.assertEqual(
                    steps,
                    bkw_cant_reach(prog, steps).step)

                self.assertFalse(
                    bkw_cant_reach(prog, steps - 1).is_settled())

########################################

class Cps(TestCase):
    def assert_could_halt_cps(self, prog: str):
        self.assertFalse(
            cps_cant_halt(prog, CPS_LIMIT))

    def assert_cant_halt_cps(self, prog: str, segs: int):
        if prog in CPS_FALSE_NEGATIVES['halt']:
            return

        self.assertTrue(
            cps_cant_halt(prog, segs))

    def assert_could_blank_cps(self, prog: str):
        self.assertFalse(
            cps_cant_blank(prog, CPS_LIMIT))

    def assert_cant_blank_cps(self, prog: str, segs: int):
        if prog in CPS_FALSE_NEGATIVES['blank']:
            return

        self.assertTrue(
            cps_cant_blank(prog, segs))

    def assert_could_spinout_cps(self, prog: str):
        self.assertFalse(
            cps_cant_spinout(prog, CPS_LIMIT))

    def assert_cant_spinout_cps(self, prog: str, segs: int):
        if prog in CPS_FALSE_NEGATIVES['spinout']:
            return

        if 2 < prog.count('...'):
            return

        self.assertTrue(
            cps_cant_spinout(prog, segs), prog)

    def assert_could_quasihalt_cps(self, prog: str):
        self.assertFalse(
            cps_cant_quasihalt(prog, CPS_LIMIT),
            f'cps quasihalt false positive: "{prog}"')

    def assert_cant_quasihalt_cps(self, prog: str, segs: int):
        if prog in CPS_QUASIHALT_FALSE_NEGATIVES:
            self.assertFalse(
                cps_cant_quasihalt(prog, CPS_LIMIT),
                f'unexpected cps quasihalt positive: "{prog}"')

            return

        self.assertTrue(
            cps_cant_quasihalt(prog, segs),
            f'cps quasihalt false negative: "{prog}"')

    def test_halt(self):
        for prog in HALTERS:
            self.assert_could_halt_cps(prog)

        for prog in NONHALTERS:
            self.assert_cant_halt_cps(prog, 7)

    def test_blank(self):
        for prog in BLANKERS:
            self.assert_could_blank_cps(prog)

        for prog in NONBLANKERS:
            self.assert_cant_blank_cps(prog, 33)

    def test_spinout(self):
        for prog in SPINNERS - MACRO_SPINOUT:
            self.assert_could_spinout_cps(prog)

        for prog in NONSPINNERS:
            self.assert_cant_spinout_cps(prog, 19)

    def test_quasihalt(self):
        for prog in QUASIHALT_HOLDOUTS:
            self.assert_could_quasihalt_cps(prog)

        for prog in (RECURS | INFRUL) - QUASIHALT:
            self.assert_cant_quasihalt_cps(prog, 30)

        for prog in QUASIHALT:
            self.assert_could_quasihalt_cps(prog)

    def test_holdouts(self):
        for prog in BLANK_HOLDOUTS:
            self.assert_could_blank_cps(prog)

        for prog in SPINOUT_HOLDOUTS:
            self.assert_could_spinout_cps(prog)

        for prog in HALT_HOLDOUTS:
            self.assert_could_halt_cps(prog)

    def test_false_negatives(self):
        counts = {
            cat: len(progs)
            for cat, progs in CPS_FALSE_NEGATIVES.items()
        }

        self.assertEqual(
            CPS_FALSE_NEGATIVE_COUNTS,
            counts,
            json.dumps(counts, indent = 4))

        cats: dict[Goal, tuple[set[str], Callable[[str], None]]] = {
            'halt': (set(), self.assert_could_halt_cps),
            'blank': (set(), self.assert_could_blank_cps),
            'spinout': (set(), self.assert_could_spinout_cps),
        }

        for cat, (pos, cps_check) in cats.items():
            for prog in CPS_FALSE_NEGATIVES[cat]:
                try:
                    cps_check(prog)
                except AssertionError:
                    pos.add(prog)

        true_pos = {
            cat: sorted(pos, key = len)
            for cat, (pos, _) in cats.items()
        }

        if any(pos for pos in true_pos.values()):
            print(json.dumps(true_pos, indent = 4))
            raise AssertionError

########################################

class Ctl(TestCase):
    def assert_could_halt_ctl(self, prog: str):
        self.assertFalse(
            ctl_cant_halt(prog, CTL_LIMIT))

    def assert_cant_halt_ctl(self, prog: str, segs: int):
        if prog in CTL_FALSE_NEGATIVES['halt']:
            return

        self.assertTrue(
            ctl_cant_halt(prog, segs), prog)

    def assert_could_blank_ctl(self, prog: str):
        self.assertFalse(
            ctl_cant_blank(prog, CTL_LIMIT))

    def assert_cant_blank_ctl(self, prog: str, segs: int):
        if prog in CTL_FALSE_NEGATIVES['blank']:
            return

        self.assertTrue(
            ctl_cant_blank(prog, segs))

    def assert_could_spinout_ctl(self, prog: str):
        self.assertFalse(
            ctl_cant_spinout(prog, CTL_LIMIT))

    def assert_cant_spinout_ctl(self, prog: str, segs: int):
        if prog in CTL_FALSE_NEGATIVES['spinout']:
            return

        self.assertTrue(
            ctl_cant_spinout(prog, segs))

    def test_halt(self):
        for prog in HALTERS:
            self.assert_could_halt_ctl(prog)

        for prog in NONHALTERS:
            self.assert_cant_halt_ctl(prog, CTL_LIMIT)

    def test_blank(self):
        for prog in BLANKERS:
            self.assert_could_blank_ctl(prog)

        for prog in NONBLANKERS:
            self.assert_cant_blank_ctl(prog, CTL_LIMIT)

    def test_spinout(self):
        for prog in SPINNERS - MACRO_SPINOUT:
            self.assert_could_spinout_ctl(prog)

        for prog in NONSPINNERS:
            self.assert_cant_spinout_ctl(prog, CTL_LIMIT)

    def test_holdouts(self):
        for prog in BLANK_HOLDOUTS:
            self.assert_could_blank_ctl(prog)

        for prog in SPINOUT_HOLDOUTS:
            self.assert_could_spinout_ctl(prog)

        for prog in HALT_HOLDOUTS:
            self.assert_could_halt_ctl(prog)

    def test_false_negatives(self):
        counts = {
            cat: len(progs)
            for cat, progs in CTL_FALSE_NEGATIVES.items()
        }

        self.assertEqual(
            CTL_FALSE_NEGATIVE_COUNTS,
            counts,
            json.dumps(counts, indent = 4))

        cats: dict[Goal, tuple[set[str], Callable[[str], None]]] = {
            'halt': (set(), self.assert_could_halt_ctl),
            'blank': (set(), self.assert_could_blank_ctl),
            'spinout': (set(), self.assert_could_spinout_ctl),
        }

        for cat, (pos, ctl_check) in cats.items():
            for prog in CTL_FALSE_NEGATIVES[cat]:
                try:
                    ctl_check(prog)
                except AssertionError:
                    pos.add(prog)

        true_pos = {
            cat: sorted(pos, key = len)
            for cat, (pos, _) in cats.items()
        }

        if any(pos for pos in true_pos.values()):
            print(json.dumps(true_pos, indent = 4))
            raise AssertionError

########################################

class Graph(TestCase):
    def test_halt(self):
        for prog in HALTERS:
            self.assertFalse(
                graph_cant_halt(prog))

        for prog in NONHALTERS:
            if not graph_cant_halt(prog):
                self.assertIn(prog, GRAPH_FALSE_NEGATIVES['halt'])

        for prog in GRAPH_FALSE_NEGATIVES['halt']:
            self.assertFalse(
                graph_cant_halt(prog))

    def test_blank(self):
        for prog in BLANKERS:
            self.assertFalse(
                graph_cant_blank(prog))

        for prog in NONBLANKERS:
            if not graph_cant_blank(prog):
                self.assertIn(prog, GRAPH_FALSE_NEGATIVES['blank'])

        for prog in GRAPH_FALSE_NEGATIVES['blank']:
            self.assertFalse(
                graph_cant_blank(prog))

    def test_spinout(self):
        for prog in SPINNERS - MACRO_SPINOUT:
            self.assertFalse(
                graph_cant_spinout(prog))

        for prog in NONSPINNERS:
            if not graph_cant_spinout(prog):
                self.assertIn(prog, GRAPH_FALSE_NEGATIVES['spinout'])

        for prog in GRAPH_FALSE_NEGATIVES['spinout']:
            self.assertFalse(
                graph_cant_spinout(prog))

    def test_strict_cycle(self):
        for prog in STRICT_CYCLE:
            self.assertTrue(
                GraphPy(prog).is_strict_cycle)

        for prog in QUASIHALT:
            self.assertFalse(
                GraphPy(prog).is_strict_cycle)

        for prog in (RECURS | INFRUL) - QUASIHALT:
            if GraphPy(prog).is_strict_cycle:
                self.assertIn(prog, STRICT_CYCLE)

    def test_quasihalt(self):
        for prog in QUASIHALT_HOLDOUTS:
            self.assertFalse(
                graph_cant_quasihalt(prog))

        for prog in QUASIHALT:
            self.assertFalse(
                graph_cant_quasihalt(prog))

        for prog in GRAPH_CANT_QUASIHALT | STRICT_CYCLE:
            self.assertTrue(
                graph_cant_quasihalt(prog))

        for prog in (RECURS | INFRUL) - QUASIHALT:
            if graph_cant_quasihalt(prog):
                self.assertIn(prog, GRAPH_CANT_QUASIHALT | STRICT_CYCLE)

    def test_holdouts(self):
        for prog in BLANK_HOLDOUTS:
            self.assertFalse(
                graph_cant_blank(prog))

        for prog in SPINOUT_HOLDOUTS:
            self.assertFalse(
                graph_cant_spinout(prog))

        for prog in HALT_HOLDOUTS:
            self.assertFalse(
                graph_cant_halt(prog))

########################################

class Far(TestCase):
    def test_true_positives(self):
        for prog in NONHALTERS:
            if not far_cant_halt(prog, 3):
                self.assertIn(prog, FAR_FALSE_NEGATIVES['halt'])

        for prog in NONBLANKERS:
            if not far_cant_blank(prog, 3):
                self.assertIn(prog, FAR_FALSE_NEGATIVES['blank'])

        for prog in NONSPINNERS:
            if not far_cant_spinout(prog, 3):
                self.assertIn(prog, FAR_FALSE_NEGATIVES['spinout'])

    def test_true_negatives(self):
        for prog in HALTERS:
            self.assertFalse(
                far_cant_halt(prog, 3))

        for prog in BLANKERS:
            self.assertFalse(
                far_cant_blank(prog, 3))

        for prog in SPINNERS:
            self.assertFalse(
                far_cant_spinout(prog, 3))

    def test_false_negatives(self):
        new_solved = False

        print('halt')
        for prog in FAR_FALSE_NEGATIVES['halt']:
            if far_cant_halt(prog, 3):
                print(prog)
                new_solved = True

        print('blank')
        for prog in FAR_FALSE_NEGATIVES['blank']:
            if far_cant_blank(prog, 3):
                print(prog)
                new_solved = True

        print('spinout')
        for prog in FAR_FALSE_NEGATIVES['spinout']:
            if far_cant_spinout(prog, 3):
                print(prog)
                new_solved = True

        self.assertFalse(new_solved)

    def test_holdouts(self):
        new_solved = False

        print('blank')
        for prog in BLANK_HOLDOUTS:
            if far_cant_blank(prog, 3):
                print(prog)
                new_solved = True

        print('spinout')
        for prog in SPINOUT_HOLDOUTS:
            if far_cant_spinout(prog, 3):
                print(prog)
                new_solved = True

        print('halt')
        for prog in HALT_HOLDOUTS:
            if far_cant_halt(prog, 3):
                print(prog)
                new_solved = True

        self.assertFalse(new_solved)
