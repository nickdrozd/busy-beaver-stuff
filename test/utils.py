import os


RUN_SLOW = os.environ.get('RUN_SLOW')


def read_progs(name: str) -> set[str]:
    with open(f'test/data/{name}.prog') as holdouts:
        return set(
            prog.strip()
            for prog in holdouts.readlines()
        )


def read_holdouts(name: str) -> set[str]:
    return read_progs(
        f'holdouts_{name}')
