from queue import Empty, Queue
from multiprocessing import (
    cpu_count,
    Manager,
    Process,
)
from typing import Any, Callable, List

from tm import Machine
from analyze import Program

RunPile = Queue[Any]

def stacker(
        steps: int,
        halt: bool,
        blank: bool,
        run_pile: RunPile,
        stack: List[str],
) -> None:
    prog = None

    while True:  # pylint: disable = while-used
        if prog is None:
            try:
                prog = stack.pop()
            except IndexError:
                break

        machine = Machine(prog).run(
            sim_lim = steps,
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

        _step, instr = machine.undfnd

        branches = (program := Program(prog)).branch(instr, halt)

        if len(program.open_slots) == (2 if halt else 1):
            run_pile.put((instr, prog))
            prog = None
            continue

        prog = next(branches := program.branch(instr, halt))

        for ext in branches:
            stack.append(ext)


def runner(
        run_pile: RunPile,
        output: Callable[[str], None],
) -> None:
    while True:  # pylint: disable = while-used
        try:
            prog = run_pile.get(timeout = 1)
        except Empty:
            break

        if isinstance(prog, str):
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
        output: Callable[[str], None] = print,
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
