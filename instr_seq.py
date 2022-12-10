import sys

from tm import Program

def format_sequence(prog: Program) -> None:
    print(f'    "{prog}": {{')

    for partial, step, slot in prog.instr_seq:
        print(f'        "{partial}": ({step : 2d}, \'{slot}\'),')

    print('    },')

if __name__ == '__main__':
    for program in map(Program, sys.stdin):
        format_sequence(program)
