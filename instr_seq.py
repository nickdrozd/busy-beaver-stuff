import sys

from tm.show import show_slot
from tm.reason import BackwardReasoner

def format_sequence(prog: BackwardReasoner) -> None:
    print(f'    "{prog}": {{')

    for partial, step, slot in prog.instr_seq:
        # pylint: disable-next = line-too-long
        print(f'        "{partial}": ({step : 2d}, \'{show_slot(slot)}\'),')

    print('    },')

if __name__ == '__main__':
    for program in map(BackwardReasoner, sys.stdin):
        format_sequence(program)
