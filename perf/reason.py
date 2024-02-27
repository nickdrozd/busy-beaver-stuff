from perf import profile, get_holdouts
from tm.reason import cant_halt, cant_blank, cant_spin_out


@profile
def main() -> None:
    for prog in get_holdouts():
        print(prog)

        _ = cant_halt(prog)
        _ = cant_blank(prog)
        _ = cant_spin_out(prog)


if __name__ == '__main__':
    main()
