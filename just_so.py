import sys

from generate import Program

if __name__ == '__main__':
    for prog in map(Program, sys.stdin):
        if prog.can_spin_out:
            try:
                print(prog)
            except BrokenPipeError:
                sys.exit()
