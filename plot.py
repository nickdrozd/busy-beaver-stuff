import sys

# pylint: disable = import-error
import matplotlib.pyplot as plt

from tm import Machine
from generate.macro import BlockMacroProg


if __name__ == '__main__':
    for i, prog in enumerate(sys.stdin):
        program = prog.strip()

        data = {
            cells: getattr(
                Machine(
                    BlockMacroProg(
                        program,
                        cells,
                    )
                ).run().final,
                'halted' if '_' in program else 'spnout',
            )
            for cells in range(1, 100)
        }

        fig, ax = plt.subplots()

        ax.scatter(
            *zip(*sorted(
                data.items())))

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
