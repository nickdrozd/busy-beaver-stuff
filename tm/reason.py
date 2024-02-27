from __future__ import annotations

from typing import TYPE_CHECKING
from collections import defaultdict

from tm.program import Program
from tm.tape import BackstepTape
from tm.rust_stuff import BackstepMachine

if TYPE_CHECKING:
    from collections.abc import Callable

    from tm.program import State, Slot

    InstrSeq = list[tuple[str, int, Slot]]

    Config = tuple[int, State, BackstepTape]

    GetResult = Callable[[BackstepMachine], int | None]

########################################

def cant_halt(prog: str) -> bool:
    return cant_reach(
        program := Program(prog),
        program.halt_slots,
        lambda machine: machine.get_halt(),
    )


def cant_blank(prog: str) -> bool:
    return cant_reach(
        program := Program(prog),
        program.erase_slots,
        lambda machine: machine.get_min_blank(),
    )


def cant_spin_out(prog: str) -> bool:
    return cant_reach(
        program := Program(prog),
        program.spinout_slots,
        lambda machine: machine.get_spinout(),
    )

########################################

def cant_reach(
        program: Program,
        slots: tuple[Slot, ...],
        get_result: GetResult,
        max_steps: int = 24,
        max_cycles: int = 1_000,
) -> bool:
    configs: list[Config] = [
        (1, state, BackstepTape(scan = color))
        for state, color in sorted(slots)
    ]

    machine = BackstepMachine(str(program))

    seen: dict[State, set[BackstepTape]] = defaultdict(set)

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

        for entry in sorted(program.graph.entry_points[state]):
            for _, instr in program.get_switch(entry).items():
                if instr is None:
                    continue

                _, shift, trans = instr

                if trans != state:
                    continue

                for color in program.colors:
                    machine.backstep_run(
                        sim_lim = step + 1,
                        init_tape = tape.to_tuples(),
                        state = entry,
                        shift = shift,
                        color = color,
                    )

                    if not (result := get_result(machine)):
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
