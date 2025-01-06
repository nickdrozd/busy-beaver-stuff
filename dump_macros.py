import sys

from tm.machine import Machine
from tm.macro import MacroProg, show_comp
from tools.normalize import normalize

if __name__ == '__main__':
    for base in sys.stdin:
        machine = Machine(
            base,
            opt_macro = 2_000,
            backsym = None,
        )

        if not isinstance(macro := machine.program, MacroProg):
            continue

        machine.run()

        prog = show_comp(macro.instrs)

        print(normalize(prog))
