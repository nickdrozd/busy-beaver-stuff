import sys
from collections import defaultdict

# pylint: disable = import-error
import matplotlib.pyplot as plt

from tm import Machine
from generate.macro import BlockMacro

WRAPS = 5
CELLS = 20

if __name__ == '__main__':
    for i, prog in enumerate(sys.stdin):
        program = prog.strip()

        macro_data = defaultdict(dict)

        for wraps in range(1, WRAPS + 1):
            for cells in range(1, CELLS + 1):
                macro = program

                for _ in range(wraps):
                    macro = BlockMacro(macro, cells)

                macro_data[wraps][cells] = getattr(
                    Machine(macro).run().final,
                    'halted' if '_' in program else 'spnout')

        fig, ax = plt.subplots()

        for macro, data in macro_data.items():
            ax.scatter(
                *zip(*sorted(data.items())),
                label = f'{macro} macro',
                color = (
                    macro / 10 * 2,
                    macro / 10 + .1,
                    macro / 10 + .3,
                ),
            )

        fig.legend(loc='upper right')

        ax.set_ylabel(
            f'Steps to {"Halt" if "_" in program else "Spinout"}')

        ax.set_xlabel(
            'Macro Block Size')

        fig.suptitle(
            program.replace('  ', ' || '))

        # ax.ticklabel_format(
        #     style = 'plain')

        fig.savefig(
            f'{program.replace(" ", "-")}.png')

        # plt.show()
