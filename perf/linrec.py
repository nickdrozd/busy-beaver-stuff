from perf import get_holdouts
from tm.machine import quick_term_or_rec


def main() -> None:
    for prog in get_holdouts():
        print(prog)

        _ = quick_term_or_rec(prog, 10_000)


if __name__ == '__main__':
    main()
