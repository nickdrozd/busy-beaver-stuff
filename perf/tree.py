from perf import profile
from tm.tree import worker, prep_branches

PROG_PARAMS = (
    (2, 2, 0),
    (3, 2, 0),
    (2, 3, 0),
    (4, 2, 1),
)

@profile
def main() -> None:
    for states, colors, halt in PROG_PARAMS:
        worker(
            steps = 100,
            halt = bool(halt),
            output = print,
            stack = prep_branches(
                states = states,
                colors = colors,
                halt = bool(halt),
            ),
        )


if __name__ == '__main__':
    main()
