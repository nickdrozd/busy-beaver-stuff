def read_progs(name: str) -> set[str]:
    with open(f'data/{name}.prog') as holdouts:  # noqa: PTH123
        return {
            prog.strip()
            for prog in holdouts
        }
