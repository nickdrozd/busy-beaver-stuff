# pylint: disable = attribute-defined-outside-init

import re
import json
from queue import Empty
from multiprocessing import Queue
from unittest import TestCase

from tm import run_bb
from generate.graph import Graph
from generate.tree  import run_tree_gen
from generate.naive import yield_programs


class TestNaive(TestCase):
    def yield_programs(self, states, colors, halt):
        self.progs = tuple(
            yield_programs(
                states,
                colors,
                halt))

    def assert_count(self, count):
        self.assertEqual(
            len(self.progs),
            count)

    def test_naive(self):
        expected = {
            (2, 2, 1): 64,
            (2, 2, 0): 256,

            (2, 3, 1): 41472,
            (2, 3, 0): 124416,

            (3, 2, 1): 57024,
            (3, 2, 0): 186624,
        }

        for (states, colors, halt), count in expected.items():
            print(states, colors, halt)
            self.yield_programs(states, colors, halt)
            self.assert_count(count)


class TestLinRado(TestCase):
    def assert_progs_equal(self, other):
        self.assertEqual(
            self.progs,
            other)

    def assert_progs_count(self, count):
        self.assertEqual(
            len(self.progs),
            count)

    def run_lin_rado(self, states, colors, halt, x_limit, rejects=None):
        print(f'{states} {colors} {halt}')

        self.progs = {
            prog
            for prog in
            yield_programs(
                states,
                colors,
                halt,
                rejects=rejects)
            if
            run_bb(
                prog,
                x_limit=x_limit,
                check_rec=0,
                check_blanks=False,
            ).final.xlimit is not None
        }

    def test_22h(self):
        self.run_lin_rado(
            2, 2, 1,
            7,  # 6
        )

        self.assert_progs_count(
            0)

    def test_22q(self):
        self.run_lin_rado(
            2, 2, 0,
            13,
        )

        self.assert_progs_equal(
            HOLDOUTS_22Q)

        self.assert_progs_count(
            4)

    def test_32h(self):
        self.run_lin_rado(
            3, 2, 1,
            29,
            rejects=NOT_CONNECTED_32,
        )

        self.assert_progs_equal(
            HOLDOUTS_32H)

        self.assert_progs_count(
            40)

        self.assertEqual(
            HOLDOUTS_32H,
            BRADY_HOLDOUTS.union(
                LR_NOT_BRADY))

    def test_23h(self):
        self.run_lin_rado(
            2, 3, 1,
            223,  # 220
        )

        self.assert_progs_equal(
            HOLDOUTS_23H)

        self.assert_progs_count(
            304)

    def test_32q(self):
        self.run_lin_rado(
            3, 2, 0,
            126,
            rejects=[AB_LOOP] + [
                prog.replace('1RH', '...')
                for prog in HOLDOUTS_32H
            ],
        )

        self.assert_progs_equal(
            HOLDOUTS_32Q)

        self.assert_progs_count(
            837)

    def test_23q(self):
        self.run_lin_rado(
            2, 3, 0,
            223,  # 220
            rejects=[
                prog.replace('1RH', '...')
                for prog in HOLDOUTS_23H
            ],
        )

        self.assert_progs_equal(
            HOLDOUTS_23Q)

        self.assert_progs_count(
            906)


class TestTree(TestCase):
    def run_tree_gen(self, states):
        self.q22 = Queue()
        self.h32 = Queue()
        self.q32 = Queue()

        def capture(prog):
            if (dots := prog.count('...')) == 2:
                self.q22.put(prog[:15])
            elif dots == 1:
                if not re.match(BC_LOOP, prog):
                    self.h32.put(prog)
            else:
                self.q32.put(prog)

        run_tree_gen(
            states,
            output = capture,
        )

        def queue_to_set(queue):
            out = set()

            while True:  # yuck -- pylint: disable = while-used
                try:
                    prog = queue.get(timeout=.5)
                    out.add(prog.replace('...', '1RH'))
                except Empty:
                    break

            return out

        # pylint: disable = redefined-variable-type
        self.q22 = queue_to_set(self.q22)
        self.h32 = queue_to_set(self.h32)
        self.q32 = queue_to_set(self.q32)

    def assert_sconn_count(self, cat, count):
        sconn = {
            prog
            for prog in cat
            if Graph(prog).is_strongly_connected
        }

        self.assertEqual(
            count,
            len(sconn))

    def test_tree(self):
        self.run_tree_gen(3)

        expected = {
             (4, 4): self.q22,
             (40, 40): self.h32,
             (609, 609): self.q32,
        }

        for (count, sconn), cat in expected.items():
            self.assertEqual(len(cat), count)
            self.assert_sconn_count(cat, sconn)

        self.assertEqual(
            self.q22,
            HOLDOUTS_22Q)

        self.assertEqual(
            self.h32,
            HOLDOUTS_32H)

        self.assertTrue(
            BRADY_HOLDOUTS <= self.h32
        )

        self.assertTrue(
            self.q32 <= HOLDOUTS_32Q
        )


HOLDOUTS_22Q = {
    "1RB 1LA 1LA 1RB",  # xmas classic
    "1RB 1LA 0LA 1RB",  # xmas one-side
    "1RB 0LB 1LA 0RA",  # xmas spaces
    "1RB 1LA 0LA 0RB",  # counter
}

LR_HOLDOUTS = {
    # Lot 1
    0o73037233,
    0o73137233,
    0o73137123,
    0o73136523,
    0o73133271,
    0o73133251,
    0o73132742,
    0o73132542,
    0o73032532,
    0o73032632,
    0o73033132,
    0o73033271,
    0o73073271,
    0o73075221,
    # Lot 2
    0o73676261,
    0o73736122,
    0o71536037,
    0o73336333,
    0o71676261,
    0o73336133,
    0o73236333,
    0o73236133,
    # Lot 3
    0o70537311,
    0o70636711,
    0o70726711,
    0o72737311,
    0o71717312,
    0o72211715,
    0o72237311,
    0o72311715,
    0o72317716,
    0o72331715,
    0o72337311,
    0o72337315,
    # Lot 4
    0o70513754,
    0o70612634,
    0o70712634,
    0o72377034,
    0o72377234,
    0o72613234,
}

def lr_convert(rado_string):
    # pylint: disable = invalid-name
    def oct_to_bin(oct_string):
        return f'{oct_string:b}'

    def bin_to_prog(bin_string):
        a0, a1, b0, b1, c0, c1 = map(
            convert_bin_instr,
            (bin_string[i : i + 4]
             for i in range(0, len(bin_string), 4)))

        return f'{a0} {a1} {b0} {b1} {c0} {c1}'

    def convert_bin_instr(bin_instr):
        pr, sh, *tr =  bin_instr

        v_sh = 'L' if int(sh) == 0 else 'R'

        v_tr = (
            'H' if (tr := int(''.join(tr), 2)) == 0
            else chr(tr + 64)
        )

        return f'{pr}{v_sh}{v_tr}'

    return bin_to_prog(
        oct_to_bin(
            rado_string))

HOLDOUTS_32H = set(map(lr_convert, LR_HOLDOUTS))

AB_LOOP = '^1RB ..[AB] ..[AB] ..[AB] ... ...'
BC_LOOP = '^1RB ... ..[BC] ..[BC] ..[BC] ..[BC]'

NOT_CONNECTED_32 = [
    AB_LOOP,
    BC_LOOP,
]

def read_progs(name):
    with open(f'test/data/{name}') as holdouts:
        return set(json.loads(holdouts.read()))

HOLDOUTS_23H = read_progs('holdouts_23h')
HOLDOUTS_32Q = read_progs('holdouts_32q')
HOLDOUTS_23Q = read_progs('holdouts_23q')

LR_NOT_BRADY = read_progs('lr_not_brady')
BRADY_HOLDOUTS = read_progs('brady_holdouts')
