import sys
from itertools import product

from tm.machine import Machine
from tm.macro import show_comp
from tm.parse import tcompile
from tools import get_params

if __name__ == '__main__':
    for prog in sys.stdin:
        prog = prog.strip()  # noqa: PLW2901

        print(f'trunk: {prog}')

        todo: list[str] = [prog]

        while todo:  # pylint: disable = while-used
            branch = todo.pop()

            machine = Machine(
                branch,
                opt_macro = 5_000,
            ).run(10_000)

            if (undfnd := machine.undfnd) is None:
                if not machine.infrul and not machine.xlimit:
                    print(machine)

                continue

            print(f'undfnd: {branch}')

            _, slot = undfnd

            states, colors = get_params(branch)
            comp = tcompile(branch)

            instrs = product(
                range(colors),
                (True, False),
                range(states),
            )

            for instr in instrs:
                ext = show_comp(comp | { slot : instr })

                print(f'branch: {ext}')

                todo.append(ext)
