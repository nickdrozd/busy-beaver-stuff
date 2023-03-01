from queue import Empty, Queue
from collections.abc import Callable
from multiprocessing import cpu_count, Manager, Process

from tm.instrs import Slot
from tm.machine import Machine
from tm.program import Program

Prog = str

Output  = Callable[[Prog], None]
RunPile = Queue[Prog | tuple[Slot, Prog]]

def stacker(
        steps: int,
        halt: bool,
        run_pile: RunPile,
        stack: list[Prog],
) -> None:
    prog: Prog | None = None

    open_slot_lim = 2 if halt else 1

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

        if machine.xlimit or machine.blanks or machine.rulapp:
            run_pile.put(prog)
            prog = None
            continue

        if machine.undfnd is None:
            prog = None
            continue

        _, slot = machine.undfnd

        if len((program := Program(prog)).open_slots) == open_slot_lim:
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
        output: Output = print,
) -> None:
    run_pile: RunPile = Manager().Queue()

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
