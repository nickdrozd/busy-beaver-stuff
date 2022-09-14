import sys

from generate.c import make_c


if __name__ == '__main__':
    for PROG in sys.stdin:
        print(
            make_c(
                PROG.strip()
            ).replace('      \n', '\n'))
