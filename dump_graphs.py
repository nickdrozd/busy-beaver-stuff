import sys
import shlex
import tempfile
import subprocess

from tm import Graph
from generate.dot import make_dot

if __name__ == '__main__':
    _, path = tempfile.mkstemp()

    for prog in sys.stdin:
        graph = Graph(prog)

        with open(path, 'w') as temp:
            temp.write(make_dot(prog, graph.arrows))

        subprocess.call(shlex.split(
            f'dot {path} -Tpng -o {graph.flatten("")}.png'))
