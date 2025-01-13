import shlex
import subprocess  # noqa: S404
import sys
import tempfile
from pathlib import Path

from tools.dot import make_dot

if __name__ == '__main__':
    _, path = tempfile.mkstemp()

    for prog in sys.stdin:
        Path(path).write_text(
            make_dot(prog))

        subprocess.call(  # noqa: S603
            shlex.split(
                f'dot {path} -T png -o {prog.replace(" ", "_")}.png'
            )
        )
