from perf import profile, HOLDOUTS
from tm.machine import quick_term_or_rec


@profile
def main() -> None:
    for prog in HOLDOUTS:
        print(prog)

        _ = quick_term_or_rec(prog, 10_000)


if __name__ == '__main__':
    main()
