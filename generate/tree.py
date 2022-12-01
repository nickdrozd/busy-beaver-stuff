from queue import Empty, Queue
from collections.abc import Callable
from multiprocessing import cpu_count, Manager, Process

from tm import Machine, Program

Output  = Callable[[str], None]
RunPile = Queue[str | tuple[tuple[str, int], str]]

def stacker(
        steps: int,
        halt: bool,
        blank: bool,
        run_pile: RunPile,
        stack: list[str],
) -> None:
    prog: str | None = None

    while True:  # pylint: disable = while-used
        if prog is None:
            try:
                prog = stack.pop()
            except IndexError:
                break

        machine = Machine(prog).run(
            sim_lim = steps,
            prover = 100,
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

        _, instr = machine.undfnd

        branches = (program := Program(prog)).branch(instr, halt)

        if len(program.open_slots) == (2 if halt else 1):
            run_pile.put((instr, prog))
            prog = None
            continue

        prog = next(branches := program.branch(instr, halt))

        for ext in branches:
            stack.append(ext)


def runner(run_pile: RunPile, output: Output) -> None:
    while True:  # pylint: disable = while-used
        try:
            prog = run_pile.get(timeout = 1)
        except Empty:
            break

        if isinstance(prog, str):
            output(prog)
        else:
            slot, prog = prog

            for ext in Program(prog).branch((slot[0], int(slot[1]))):
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
