import sys

from tm.program import Program

if __name__ == '__main__':
    for prog in sys.stdin:
        print(Program(prog).normalize())
