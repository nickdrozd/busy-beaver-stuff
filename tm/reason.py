from __future__ import annotations

from typing import TYPE_CHECKING
from collections import defaultdict

from tm.parse import parse
from tm.graph import Graph
from tm.tape import BackstepTape
from tm.rust_stuff import (
    halt_slots,
    erase_slots,
    BackstepMachineHalt,
    BackstepMachineBlank,
    BackstepMachineSpinout,
)

if TYPE_CHECKING:
    from collections.abc import Callable

    from tm.parse import State, Slot, Instr

    InstrSeq = list[tuple[str, int, Slot]]

    Config = tuple[int, State, BackstepTape]

    BackstepMachine = (
        BackstepMachineHalt
        | BackstepMachineBlank
        | BackstepMachineSpinout
    )

########################################

def cant_halt(prog: str) -> bool:
    return cant_reach(
        prog,
        halt_slots(prog),
        lambda: BackstepMachineHalt(prog),
    )


def cant_blank(prog: str) -> bool:
    return cant_reach(
        prog,
        erase_slots(prog),
        lambda: BackstepMachineBlank(prog),
    )


def cant_spin_out(prog: str) -> bool:
    graph = Graph(prog)

    return cant_reach(
        prog,
        [
            (state, 0)
            for state in graph.zero_reflexive_states
        ],
        lambda: BackstepMachineSpinout(prog),
        graph = graph,
    )

########################################

def cant_reach(
        program: str,
        slots: list[Slot],
        get_machine: Callable[[], BackstepMachine],
        graph: Graph | None = None,
        max_steps: int = 24,
        max_cycles: int = 1_000,
) -> bool:
    if not slots:
        return True

    prog: dict[State, list[Instr | None]] = \
        dict(enumerate(parse(program)))

    configs: list[Config] = [
        (1, state, BackstepTape(scan = color))
        for state, color in sorted(slots)
    ]

    seen: dict[State, set[BackstepTape]] = defaultdict(set)

    machine = get_machine()

    if graph is None:
        graph = Graph(program)

    entry_points = graph.entry_points

    colors = graph.colors

    for _ in range(max_cycles):  # no-branch
        try:
            step, state, tape = configs.pop()
        except IndexError:
            return True

        if step > max_steps:
            return False

        if state == 0 and tape.blank:
            return False

        if tape in seen[state]:
            continue

        seen[state].add(tape)

        # print(step, state, tape)

        for entry in sorted(entry_points[state]):
            for instr in prog[entry]:
                if instr is None:
                    continue

                _, shift, trans = instr

                if trans != state:
                    continue

                for color in colors:
                    result = machine.backstep_run(
                        sim_lim = step + 1,
                        init_tape = tape.to_tuples(),
                        state = entry,
                        shift = shift,
                        color = color,
                    )

                    if not result:
                        continue

                    if abs(result - step) > 1:
                        continue

                    yield_tape = tape.copy()

                    yield_tape.backstep(shift, color)

                    configs.append((
                        step + 1,
                        entry,
                        yield_tape,
                    ))

    return False  # no-cover
