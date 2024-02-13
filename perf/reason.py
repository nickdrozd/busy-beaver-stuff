from perf import profile, get_holdouts
from tm.reason import BackwardReasoner


@profile
def main() -> None:
    for prog in get_holdouts():
        print(prog)

        reasoner = BackwardReasoner(prog)

        _ = reasoner.cant_halt
        _ = reasoner.cant_blank
        _ = reasoner.cant_spin_out


if __name__ == '__main__':
    main()
