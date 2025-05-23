import sys

from tm.show import show_slot
from tools.instr_seq import instr_seq


def format_sequence(prog: str) -> None:
    print(f'    "{prog.strip()}": {{')

    for partial, step, slot in instr_seq(prog):
        print(
            "        \"{}\": ({:2d}, '{}'),".format(  # noqa: UP032
                partial,
                step,
                show_slot(slot),
            )
        )

    print('    },')

if __name__ == '__main__':
    for program in sys.stdin:
        format_sequence(program)
