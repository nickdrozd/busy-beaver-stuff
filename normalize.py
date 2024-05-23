import sys

from tools.normalize import normalize, expand

if __name__ == '__main__':
    for prog in sys.stdin:
        print((normalize if ' ' in prog else expand)(prog))
