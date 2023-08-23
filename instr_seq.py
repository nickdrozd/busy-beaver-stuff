import sys

from tm.show import show_slot
from tm.reason import instr_seq

def format_sequence(prog: str) -> None:
    print(f'    "{prog}": {{')

    for partial, step, slot in instr_seq(prog):
        # pylint: disable-next = line-too-long
        print(f'        "{partial}": ({step : 2d}, \'{show_slot(slot)}\'),')

    print('    },')

if __name__ == '__main__':
    for program in sys.stdin:
        format_sequence(program)
