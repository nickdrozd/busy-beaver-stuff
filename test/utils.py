from functools import reduce


def read_progs(name: str) -> set[str]:
    with open(f'test/data/{name}.prog') as holdouts:
        return set(
            prog.strip()
            for prog in holdouts.readlines()
        )


def get_holdouts() -> list[str]:
    return sorted(
        reduce(
            lambda t, s: t | read_progs(f'holdouts_{s}'), # type: ignore
            ('32q', '23q', '42h', '42q', '24h'),
            set(),
        )
    )
