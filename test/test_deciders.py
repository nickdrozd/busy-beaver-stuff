# ruff: noqa: F405
import json
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
    far_cant_blank,
    far_cant_halt,
    far_cant_spinout,
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
BKW_LIMIT = 3_000

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

class DeciderTest(TestCase):
    false_negatives: dict[Goal, set[str]]

########################################

BACKWARD_REASONERS: dict[Goal, BackwardReasoner] = {
    "halt": bkw_cant_halt,
    "blank": bkw_cant_blank,
    "spinout": bkw_cant_spinout,
    "twostep": bkw_cant_twostep,
    "zloop": bkw_cant_zloop,
}

class Backward(DeciderTest):
    false_negatives = FALSE_NEGATIVES['bkw']

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
        if prog in self.false_negatives['halt'] | RECUR_FAST:
            return

        self.assertTrue(
            bkw_cant_halt(prog, depth).is_refuted(),
            f'halt false negative: "{prog}"')

    def assert_cant_blank_backward(self, prog: str, depth: int):
        if prog in self.false_negatives['blank'] | RECUR_FAST:
            return

        self.assertTrue(
            bkw_cant_blank(prog, depth).is_refuted(),
            f'blank false negative: "{prog}"')

    def assert_cant_spinout_backward(self, prog: str, depth: int):
        if prog in self.false_negatives['spinout'] | RECUR_FAST:
            return

        if 2 < prog.count('...'):
            return

        self.assertTrue(
            bkw_cant_spinout(prog, depth).is_refuted(),
            f'spinout false negative: "{prog}"')

    def assert_cant_twostep_backward(self, prog: str, depth: int):
        if prog in self.false_negatives['twostep']:
            return

        self.assertTrue(
            bkw_cant_twostep(prog, depth).is_refuted(),
            f'twostep false negative: "{prog}"')

    def assert_cant_zloop_backward(self, prog: str, depth: int):
        if prog in self.false_negatives['zloop']:
            return

        self.assertTrue(
            bkw_cant_zloop(prog, depth).is_refuted(),
            f'zloop false negative: "{prog}"')

    ########################################

    def test_true_positives(self):
        for prog in NONHALTERS:
            self.assert_cant_halt_backward(prog, 200)

        for prog in NONSPINNERS:
            self.assert_cant_spinout_backward(prog, 256)

        for prog in NONBLANKERS:
            self.assert_cant_blank_backward(prog, 1331)

        for prog in HALTERS | SPINNERS | (RECURS - TWOSTEPPERS):
            self.assert_cant_twostep_backward(prog, 0)

        for prog in HALTERS | SPINNERS | (RECURS - ZLOOPERS - ZLOOPY):
            self.assert_cant_zloop_backward(prog, 4)

    def test_true_negatives(self):
        for prog in HALTERS:
            self.assert_could_halt_backward(prog)

        for prog in SPINNERS:
            self.assert_could_spinout_backward(prog)

        for prog in BLANKERS:
            self.assert_could_blank_backward(prog)

        for prog in TWOSTEPPERS:
            self.assert_could_twostep_backward(prog)

        for prog in ZLOOPERS:
            self.assert_could_zloop_backward(prog)

    def test_holdouts(self):
        for prog in HALT_HOLDOUTS:
            self.assert_could_halt_backward(prog)

        for prog in BLANK_HOLDOUTS:
            self.assert_could_blank_backward(prog)

        for prog in SPINOUT_HOLDOUTS:
            self.assert_could_spinout_backward(prog)

    def test_false_negatives(self):
        solved: dict[Goal, set[str]] = {
            goal: set()
            for goal in self.false_negatives
        }

        for goal, progs in self.false_negatives.items():
            for prog in progs:
                result = BACKWARD_REASONERS[goal](prog, BKW_LIMIT)

                if result.is_refuted():
                    solved[goal].add(prog)

        if any(progs for progs in solved.values()):
            for goal, progs in solved.items():
                print(goal)
                for prog in progs:
                    print(prog)

            raise AssertionError

    def test_steps(self):
        for cat, data in BACKWARD_STEPS.items():
            bkw_cant_reach = BACKWARD_REASONERS[cat]

            for prog, steps in data.items():
                self.assertEqual(
                    bkw_cant_reach(prog, BKW_LIMIT).step,
                    steps)


########################################

class Cps(DeciderTest):
    false_negatives = FALSE_NEGATIVES['cps']

    def assert_could_halt_cps(self, prog: str):
        self.assertFalse(
            cps_cant_halt(prog, CPS_LIMIT))

    def assert_cant_halt_cps(self, prog: str, segs: int):
        if prog in self.false_negatives['halt']:
            return

        self.assertTrue(
            cps_cant_halt(prog, segs))

    def assert_could_blank_cps(self, prog: str):
        self.assertFalse(
            cps_cant_blank(prog, CPS_LIMIT))

    def assert_cant_blank_cps(self, prog: str, segs: int):
        if prog in self.false_negatives['blank']:
            return

        self.assertTrue(
            cps_cant_blank(prog, segs))

    def assert_could_spinout_cps(self, prog: str):
        self.assertFalse(
            cps_cant_spinout(prog, CPS_LIMIT))

    def assert_cant_spinout_cps(self, prog: str, segs: int):
        if prog in self.false_negatives['spinout']:
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

    def test_true_positives(self):
        for prog in NONHALTERS:
            self.assert_cant_halt_cps(prog, 7)

        for prog in NONBLANKERS:
            self.assert_cant_blank_cps(prog, 33)

        for prog in NONSPINNERS:
            self.assert_cant_spinout_cps(prog, 19)

    def test_true_negatives(self):
        for prog in HALTERS:
            self.assert_could_halt_cps(prog)

        for prog in BLANKERS:
            self.assert_could_blank_cps(prog)

        for prog in SPINNERS:
            self.assert_could_spinout_cps(prog)

    def test_quasihalt(self):
        for prog in QUASIHALT_HOLDOUTS:
            self.assert_could_quasihalt_cps(prog)

        for prog in (RECURS | INFRUL) - QUASIHALT - RECUR_FAST:
            self.assert_cant_quasihalt_cps(prog, 30)

        for prog in QUASIHALT:
            try:
                self.assert_could_quasihalt_cps(prog)
            except AssertionError:
                assert 'A' not in prog

    def test_holdouts(self):
        for prog in BLANK_HOLDOUTS:
            self.assert_could_blank_cps(prog)

        for prog in SPINOUT_HOLDOUTS:
            self.assert_could_spinout_cps(prog)

        for prog in HALT_HOLDOUTS:
            self.assert_could_halt_cps(prog)

    def test_false_negatives(self):
        cats: dict[Goal, tuple[set[str], Callable[[str], None]]] = {
            'halt': (set(), self.assert_could_halt_cps),
            'blank': (set(), self.assert_could_blank_cps),
            'spinout': (set(), self.assert_could_spinout_cps),
        }

        for cat, (pos, cps_check) in cats.items():
            for prog in self.false_negatives[cat]:
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

class Graph(DeciderTest):
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

########################################

class Far(DeciderTest):
    false_negatives = FALSE_NEGATIVES['far']

    def test_true_positives(self):
        for prog in NONHALTERS:
            if not far_cant_halt(prog, 3):
                self.assertIn(prog, self.false_negatives['halt'])

        for prog in NONBLANKERS:
            if not far_cant_blank(prog, 3):
                self.assertIn(prog, self.false_negatives['blank'] | RECUR_FAST)

        for prog in NONSPINNERS:
            if not far_cant_spinout(prog, 3):
                self.assertIn(prog, self.false_negatives['spinout'] | RECUR_FAST)

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
        for prog in self.false_negatives['halt']:
            if far_cant_halt(prog, 3):
                print(prog)
                new_solved = True

        print('blank')
        for prog in self.false_negatives['blank']:
            if far_cant_blank(prog, 3):
                print(prog)
                new_solved = True

        print('spinout')
        for prog in self.false_negatives['spinout']:
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
