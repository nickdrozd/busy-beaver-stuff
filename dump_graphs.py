import sys
import shlex
import tempfile
import subprocess

from analyze import Graph


if __name__ == '__main__':
    _, path = tempfile.mkstemp()

    for prog in sys.stdin:
        graph = Graph(prog)

        with open(path, 'w') as temp:
            temp.write(graph.dot)

        subprocess.call(shlex.split(
            f'dot {path} -Tpng -o {graph.flatten("")}.png'))
