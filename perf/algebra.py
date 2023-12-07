from test.prog_data import ALGEBRA
from tm.machine import Machine
from perf import profile


@profile
def main() -> None:
    for data in ALGEBRA.values():
        for prog in data.keys():
            print(prog)
            _ = Machine(prog, opt_macro = 2_000).run()


if __name__ == '__main__':
    main()
