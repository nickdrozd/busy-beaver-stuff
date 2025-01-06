import sys

from test.test_turing import branch_last
from tm.machine import Machine

if __name__ == '__main__':
    for prog in sys.stdin:
        for i, branch in enumerate(branch_last(prog)):
            machine = Machine(branch, opt_macro = 5_000).run(10_000)

            if machine.xlimit is None:
                print(i, machine)
