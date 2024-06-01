import sys

from tools import get_params as params
from tm.tree import Program
from tm.machine import Machine


if __name__ == '__main__':
    for prog in sys.stdin:
        for slot in (program := Program(params(prog), prog)).open_slots:
            for i, branch in enumerate(program.branch(slot)):
                machine = Machine(branch, opt_macro = 5_000).run(10_000)

                if machine.xlimit is None:
                    print(i, machine)
