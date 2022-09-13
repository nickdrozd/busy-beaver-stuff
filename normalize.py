import sys

from analyze import Program

if __name__ == '__main__':
    for prog in sys.stdin:
        print(Program(prog).normalize())
