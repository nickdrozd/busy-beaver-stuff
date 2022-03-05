import sys

from generate import Program

if __name__ == '__main__':
    for prog in map(Program, sys.stdin):
        if prog.cant_spin_out:
            continue

        try:
            print(prog)
        except BrokenPipeError:
            sys.exit()
