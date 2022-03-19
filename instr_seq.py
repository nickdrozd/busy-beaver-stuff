import sys

from generate import Program

if __name__ == '__main__':
    for prog in map(Program, sys.stdin):
        print(f'    # {prog}')

        for partial, step, instr in prog.instruction_sequence:
            print(f'    "{partial}": ({step}, \'{instr}\'),')

        print()
