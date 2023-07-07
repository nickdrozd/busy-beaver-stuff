import sys

from tm.parse import st_str
from tm.program import Program

def format_sequence(prog: Program) -> None:
    print(f'    "{prog}": {{')

    for partial, step, (state, color) in prog.instr_seq:
        slot = st_str(state) + str(color)
        print(f'        "{partial}": ({step : 2d}, \'{slot}\'),')

    print('    },')

if __name__ == '__main__':
    for program in map(Program, sys.stdin):
        format_sequence(program)
