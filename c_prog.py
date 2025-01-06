import sys

from tools.c import make_c

if __name__ == '__main__':
    for PROG in sys.stdin:
        print(
            make_c(
                PROG.strip()))
