import sys

from tm.machine import Machine, show_number


def run(prog: str) -> None:
    machine = Machine(prog, opt_macro = 2_000).run()

    print('infrul' if machine.infrul else machine.limrul)

    assert not isinstance(marks := machine.marks, int)

    print('\n'.join([
        f'        "{prog.strip()}": (',
        f'            {machine.cycles},',
        f'            "{marks.estimate()}",',
        f'            "{marks}",',
        f'            "{show_number(machine.rulapp)}",',
        '        ),',
    ]))


if __name__ == '__main__':
    for PROG in sys.stdin:
        run(PROG)
