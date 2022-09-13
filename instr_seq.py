import sys

from analyze import Program

def format_sequence(prog):
    print(f'    "{prog}": {{')

    for partial, step, slot in prog.instruction_sequence:
        print(f'        "{partial}": ({step : 2d}, \'{slot}\'),')

    print('    },')

if __name__ == '__main__':
    for program in map(Program, sys.stdin):
        format_sequence(program)
