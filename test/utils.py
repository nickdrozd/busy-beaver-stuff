import os

RUN_SLOW = os.environ.get('RUN_SLOW')


def read_progs(name: str) -> set[str]:
    with open(f'data/{name}.prog') as holdouts:
        return {
            prog.strip()
            for prog in holdouts
        }


def read_holdouts(name: str) -> set[str]:
    return read_progs(
        f'tree/holdouts_{name}')
