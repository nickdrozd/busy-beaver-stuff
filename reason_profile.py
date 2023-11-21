from test.utils import read_progs
from tm.reason import BackwardReasoner
from perf import profile


PROGS = (
    read_progs('holdouts_32q')
    | read_progs('holdouts_23q')
    | read_progs('holdouts_42h')
    | read_progs('holdouts_24h')
)


@profile
def main() -> None:
    for prog in PROGS:
        print(prog)

        reasoner = BackwardReasoner(prog)

        _ = reasoner.cant_halt
        _ = reasoner.cant_blank
        _ = reasoner.cant_spin_out


if __name__ == '__main__':
    main()
