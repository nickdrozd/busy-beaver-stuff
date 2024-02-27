import sys

from tools.normalize import Normalizer

if __name__ == '__main__':
    for prog in sys.stdin:
        print(Normalizer(prog).normalize())
