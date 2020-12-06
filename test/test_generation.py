from unittest import TestCase

from generate.generate import yield_programs

EXPECTED = {
    (2, 2, 1): 64,
    (2, 2, 0): 256,

    (2, 3, 1): 41472,
    (2, 3, 0): 124416,

    (3, 2, 1): 57024,
    (3, 2, 0): 186624,
}


class TestGeneration(TestCase):
    def yield_programs(self, states, colors, halt):
        # pylint: disable = attribute-defined-outside-init
        self.progs = tuple(
            yield_programs(
                states,
                colors,
                halt=halt))

    def assert_count(self, count):
        self.assertEqual(
            len(self.progs),
            count)

    def test_general(self):
        for (states, colors, halt), count in EXPECTED.items():
            print(states, colors, halt)
            self.yield_programs(states, colors, halt)
            self.assert_count(count)
