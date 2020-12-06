from unittest import TestCase

import lin_rado
from turing import run_bb
from generate.generate import yield_programs


HOLDOUTS_22Q = {
    "1RB 0LB 1LA 0RA",
    "1RB 1LA 0LA 0RB",
    "1RB 1LA 0LA 1RB",
    "1RB 1LA 1LA 1RB",
}

HOLDOUTS_32H = {
    lin_rado.convert(prog)
    for prog in lin_rado.HOLDOUTS
}


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
        # pylint: disable=attribute-defined-outside-init
        self.progs = {
            prog
            for prog in
            yield_programs(
                states,
                colors,
                rejects=rejects,
                halt=halt)
            if
            run_bb(
                prog,
                x_limit=x_limit,
                check_rec=0
            ).final == 'XLIMIT'
        }

    def test_22h(self):
        self.run_lin_rado(2, 2, 1, 50)

        self.assert_progs_count(0)

    def test_22q(self):
        self.run_lin_rado(2, 2, 0, 50)

        self.assert_progs_equal(
            HOLDOUTS_22Q)

    def test_32h(self):
        self.run_lin_rado(
            states=3,
            colors=2,
            halt=1,
            x_limit=50,
            rejects=[
                '^1RB ... ..[BC] ..[BC] ..[BC] ..[BC]',
                '^1RB ..[AB] ..[AB] ..[AB] ... ...',
            ]
        )

        self.assert_progs_equal(
            HOLDOUTS_32H)

        self.assert_progs_count(
            40)

    def test_32q(self):
        self.run_lin_rado(
            states=3,
            colors=2,
            halt=0,
            x_limit=150,
            rejects=[
                # '^1RB ... ..[BC] ..[BC] ..[BC] ..[BC]',
                # '^1RB ..[AB] ..[AB] ..[AB] ... ...',
            ] + [
                prog.replace('1RH', '...')
                for prog in HOLDOUTS_32H
            ],
        )

        self.assert_progs_count(
            1413)
