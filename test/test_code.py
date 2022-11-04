from unittest import TestCase

from tm import Graph
from generate.c import make_c
from generate.dot import make_dot

TEST_FILES = {
    "1RB 1RC  1LC 1RD  1RA 1LD  0RD 0LB": 2819,
    "1RB 2LA 1RA 1RA  1LB 1LA 3RB 1R_": 3932964,
    "1RB 1LC  1RD 1RB  0RD 0RC  1LD 1LA": 32779477,
    "1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  1R_ 0LA": 47176870,
}

class TestCode(TestCase):
    def test_code(self):
        self.maxDiff = None

        for prog, name in TEST_FILES.items():
            print(prog)

            with open(f'test/data/c/{name}.c.test') as test:
                self.assertEqual(
                    test.read(),
                    make_c(prog) + '\n')

            with open(f'test/data/dot/{name}.dot') as test:
                self.assertEqual(
                    test.read(),
                    make_dot(prog, Graph(prog).arrows) + '\n')
