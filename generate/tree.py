from queue import Empty, Queue
from collections.abc import Callable
from multiprocessing import cpu_count, Manager, Process

from tm.machine import Machine
from tm.program import Program

Prog = str
Slot = tuple[int, int]

Output  = Callable[[Prog], None]
RunPile = Queue[Prog | tuple[Slot, Prog]]

def stacker(
        steps: int,
        halt: bool,
        blank: bool,
        run_pile: RunPile,
        stack: list[Prog],
) -> None:
    prog: Prog | None = None

    while True:  # pylint: disable = while-used
        if prog is None:
            try:
                prog = stack.pop()
            except IndexError:
                break

        machine = Machine(prog).run(
            sim_lim = steps,
            prover = True,
        )

        if blank and machine.blanks:
            prog = None
            continue

        if machine.xlimit is not None:
            run_pile.put(prog)
            prog = None
            continue

        if machine.undfnd is None:
            prog = None
            continue

        _, slot = machine.undfnd

        branches = (program := Program(prog)).branch(slot, halt)

        if len(program.open_slots) == (2 if halt else 1):
            run_pile.put((slot, prog))
            prog = None
            continue

        prog = next(branches := program.branch(slot, halt))

        for ext in branches:
            stack.append(ext)


def runner(run_pile: RunPile, output: Output) -> None:
    while True:  # pylint: disable = while-used
        try:
            prog = run_pile.get(timeout = 1)
        except Empty:
            break

        if isinstance(prog, Prog):
            output(prog)
        else:
            slot, prog = prog

            for ext in Program(prog).branch(slot):
                output(ext)


def run_tree_gen(
        states: int,
        colors: int,
        steps: int = 500,
        halt: bool = False,
        blank: bool = False,
        output: Output = print,
) -> None:
    run_pile: RunPile = Manager().Queue()

    processes = [
        Process(
            target = stacker,
            args = (
                steps,
                halt,
                blank,
                run_pile,
                [str(Program.empty(states, colors))],
            ),
        )
    ]

    processes += [
        Process(
            target = runner,
            args = (run_pile, output)
        )
        for _ in range(cpu_count())
    ]

    for process in processes:
        process.start()

    for process in processes:
        process.join()
