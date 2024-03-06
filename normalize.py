import sys

from tools.normalize import normalize

if __name__ == '__main__':
    for prog in sys.stdin:
        print(normalize(prog))
