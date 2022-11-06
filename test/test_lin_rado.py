# pylint: disable = attribute-defined-outside-init
from unittest import TestCase

from tm import LinRecMachine
from generate.naive import yield_programs


class TestLinRado(TestCase):
    def assert_progs_equal(self, other: set[str]):
        self.assertEqual(
            self.progs,
            other)

    def assert_progs_count(self, count: int):
        self.assertEqual(
            len(self.progs),
            count)

    def run_lin_rado(
            self,
            states: int,
            colors: int,
            halt: int,
            xlimit: int,
            rejects: list[str] | None = None,
    ):
        self.progs = {
            prog
            for prog in
            yield_programs(
                states,
                colors,
                bool(halt),
                rejects)
            if
            LinRecMachine(prog).run(
                step_lim = xlimit,
                check_rec = 0,
                skip = True,
            ).xlimit is not None
        }

    def test_22(self):
        # h
        self.run_lin_rado(
            2, 2, 1,
            10,
        )

        self.assert_progs_count(
            0)

        # q
        self.run_lin_rado(
            2, 2, 0,
            13,
            rejects = [],
        )

        self.assert_progs_equal(
            {
                "1RB 1LA  0LA 0RB",  # counter
                "1RB 1LA  1LA 1RB",  # xmas classic
                "1RB 1LA  0LA 1RB",  # xmas one-side
                "1RB 0LB  1LA 0RA",  # xmas spaces
            })

        self.assert_progs_count(
            4)

    def test_32h(self):
        self.run_lin_rado(
            3, 2, 1,
            29,
            rejects = NOT_CONNECTED_32,
        )

        self.assert_progs_count(
            40)

        self.assert_progs_equal(
            LIN_HOLDOUTS)

        self.assertEqual(
            LIN_HOLDOUTS,
            BRADY_HOLDOUTS | LR_NOT_BRADY)


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

def lr_convert(rado_string: int) -> str:
    def oct_to_bin(oct_string: int) -> str:
        return f'{oct_string:b}'

    def bin_to_prog(bin_string: str) -> str:
        # pylint: disable = invalid-name
        a0, a1, b0, b1, c0, c1 = map(
            convert_bin_instr,
            (bin_string[i : i + 4]
             for i in range(0, len(bin_string), 4)))

        return f'{a0} {a1}  {b0} {b1}  {c0} {c1}'

    def convert_bin_instr(bin_instr: str) -> str:
        pr: str
        sh: str
        tr: list[str]

        pr, sh, *tr =  bin_instr  # type: ignore

        v_sh = 'L' if int(sh) == 0 else 'R'

        v_tr = (
            '_' if (vtr := int(''.join(tr), 2)) == 0
            else chr(vtr + 64)
        )

        return f'{pr}{v_sh}{v_tr}'

    return bin_to_prog(
        oct_to_bin(
            rado_string))

LIN_HOLDOUTS = set(map(lr_convert, LR_HOLDOUTS))

AB_LOOP = '^1RB ..[AB]  ..[AB] ..[AB]  ... ...'
BC_LOOP = '^1RB ...  ..[BC] ..[BC]  ..[BC] ..[BC]'

NOT_CONNECTED_32 = [
    AB_LOOP,
    BC_LOOP,
]

def read_progs(name: str) -> set[str]:
    with open(f'test/data/{name}.prog') as holdouts:
        return set(
            prog.strip()
            for prog in holdouts.readlines()
        )

LR_NOT_BRADY = read_progs('lr_not_brady')
BRADY_HOLDOUTS = read_progs('brady_holdouts')

LIN_EXAMPLES = {
    "1RB ...  0RC 1LB  1LA 0RB",  # total recurrence (blank)
    "1RB ...  1LB 0LC  1LA 1RA",  # left barrier
    "1RB ...  1LC 1RA  1LA 0LC",  # right barrier
}