from unittest import TestCase

from rado import convert_rado, HOLDOUTS
from turing import run_bb
from generate.generate import yield_programs


CONVERTED_HOLDOUTS = {
    convert_rado(prog)
    for prog in HOLDOUTS
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

    def test_lin_rado_32h(self):
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
            CONVERTED_HOLDOUTS)

        self.assert_progs_count(
            40)

    def test_lin_rado_32q(self):
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
                for prog in CONVERTED_HOLDOUTS
            ],
        )

        self.assert_progs_count(
            1413)
