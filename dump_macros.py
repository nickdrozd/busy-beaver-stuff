import sys

from tm.macro import show_comp, MacroProg
from tm.machine import Machine

from tools.normalize import normalize

if __name__ == '__main__':
    for base in sys.stdin:
        machine = Machine(
            base,
            opt_macro = 2_000,
            backsym = None,
        ).run()

        if not isinstance(macro := machine.program, MacroProg):
            continue

        prog = show_comp(macro.instrs)

        print(normalize(prog))
