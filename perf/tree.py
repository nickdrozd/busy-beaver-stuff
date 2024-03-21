from perf import profile
from tm.tree import worker, prep_branches


@profile
def main() -> None:
    worker(
        steps = 100,
        halt = False,
        output = print,
        stack = prep_branches(
            states = 2,
            colors = 3,
            halt = False,
        ),
    )


if __name__ == '__main__':
    main()
