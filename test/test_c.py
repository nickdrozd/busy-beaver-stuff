from unittest import TestCase

from generate.c import make_c

class TestC(TestCase):
    def test_c(self):
        with open('test/data/2819.c.test') as test:
            expected = test.read()
            actual = make_c("1RB 1RC  1LC 1RD  1RA 1LD  0RD 0LB")

            self.assertEqual(
                expected,
                actual)
