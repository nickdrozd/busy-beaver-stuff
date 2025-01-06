import shlex
import subprocess
import sys
import tempfile

from tools.dot import make_dot

if __name__ == '__main__':
    _, path = tempfile.mkstemp()

    for prog in sys.stdin:
        with open(path, 'w') as temp:
            temp.write(make_dot(prog))

        subprocess.call(  # noqa: S603
            shlex.split(
                f'dot {path} -T png -o {prog.replace(" ", "_")}.png'
            )
        )
