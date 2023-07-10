from argparse import ArgumentParser

from tm.reason import BackwardReasoner
from tm.machine import run_variations

from generate.tree import run_tree_gen, Output, Prog


def filter_run_print(halt: bool) -> Output:
    def cant_halt(prog: Prog) -> bool:
        return BackwardReasoner(prog).cant_halt

    def cant_spin_out(prog: Prog) -> bool:
        return BackwardReasoner(prog).cant_spin_out

    cant_reach = cant_halt if halt else cant_spin_out

    def drop(prog: Prog) -> None:
        if cant_reach(prog):
            return

        for machine in run_variations(prog, 10_000):
            if machine.simple_termination and machine.rulapp > 1_000:
                print(machine)
                return

            if machine.xlimit is None:
                return

        print(prog)

    return drop



if __name__ == '__main__':
    parser = ArgumentParser()

    parser.add_argument('states', type = int)
    parser.add_argument('colors', type = int)

    parser.add_argument('--steps', type = int, default = 200)

    parser.add_argument('--halt', action = 'store_true')

    args = parser.parse_args()

    run_tree_gen(
        states = args.states,
        colors = args.colors,
        halt   = args.halt,
        steps  = args.steps,
        output = filter_run_print(args.halt),
    )
