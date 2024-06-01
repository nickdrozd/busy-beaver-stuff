from perf import profile
from tm.tree import worker, init_branches

PROG_PARAMS = (
    ((2, 2), 0),
    ((3, 2), 0),
    ((2, 3), 0),
    ((4, 2), 1),
)

@profile
def main() -> None:
    for params, halt in PROG_PARAMS:
        worker(
            steps = 100,
            halt = bool(halt),
            params = params,
            output = print,
            stack = init_branches(params),
        )


if __name__ == '__main__':
    main()
