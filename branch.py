import sys

from tm.program import Program
from tm.machine import Machine


if __name__ == '__main__':
    for prog in sys.stdin:
        # pylint: disable = redefined-loop-name
        prog = prog.replace('1R_', '...')

        if (slot := (program := Program(prog)).last_slot) is None:
            continue

        for i, branch in enumerate(program.branch(slot)):
            machine = Machine(branch, opt_macro = 5_000).run(10_000)

            if machine.xlimit is None:
                print(i, machine)