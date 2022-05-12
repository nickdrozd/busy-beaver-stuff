from queue import Empty
from multiprocessing import (
    cpu_count,
    Manager,
    Process,
)
from typing import Callable, List

from tm import Machine
from generate import Program


def stacker(steps: int, halt: bool, run_pile, stack: List[str]):
    prog = None

    while True:  # pylint: disable = while-used
        if prog is None:
            try:
                prog = stack.pop()
            except IndexError:
                break

        machine = Machine(prog).run(
            sim_lim = steps,
            check_blanks = True,
        )

        if machine.final.xlimit is not None:
            run_pile.put(prog)
            prog = None
            continue

        if machine.final.undfnd is None:
            prog = None
            continue

        _step, instr = machine.final.undfnd

        branches = (program := Program(prog)).branch(instr, halt)

        if len(program.open_slots) == (2 if halt else 1):
            run_pile.put((instr, prog))
            prog = None
            continue

        try:
            prog = next(branches := program.branch(instr, halt))
        except StopIteration:
            prog = None
            continue

        for ext in branches:
            stack.append(ext)


def runner(run_pile, output: Callable):
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
        halt: bool = False,
        output: Callable = print):
    try:
        steps = {
            (2, 2): 40,
            (3, 2): 126,
            (2, 3): 223,  # 220
        }[(states, colors)]
    except KeyError:
        steps = 500

    run_pile = Manager().Queue()

    processes = [
        Process(
            target = stacker,
            args = (
                steps,
                halt,
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
