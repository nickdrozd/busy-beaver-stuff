from pathlib import Path
from unittest import TestCase

from tools.c import make_c
from tools.dot import make_dot

TEST_FILES = {
    "1RB 1RC  1LC 1RD  1RA 1LD  0RD 0LB": 2819,
    "1RB 2LA 1RA 1RA  1LB 1LA 3RB 1R_": 3932964,
    "1RB 1LC  1RD 1RB  0RD 0RC  1LD 1LA": 32779477,
    "1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  1R_ 0LA": 47176870,
    "1RB ...  1RC ...  1LC 1LD  1RE 1LF  1RC 1RE  0RC 0RF": 32779478,
}

class TestCode(TestCase):
    def test_code(self):
        self.maxDiff = None

        for prog, name in TEST_FILES.items():
            print(prog)

            self.assertEqual(
                Path(f"data/c/{name}.c.test").read_text(),
                make_c(prog) + '\n')

            self.assertEqual(
                Path(f"data/dot/{name}.dot").read_text(),
                make_dot(prog) + '\n')
