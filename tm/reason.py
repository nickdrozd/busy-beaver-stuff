from __future__ import annotations

from enum import Enum
from typing import TYPE_CHECKING
from collections import defaultdict

from tm.tape import BackstepTape
from tm.rust_stuff import (
    halt_slots,
    erase_slots,
    zero_reflexive_slots,
    reason_parse,
    BackstepMachineHalt,
    BackstepMachineBlank,
    BackstepMachineSpinout,
)

if TYPE_CHECKING:
    from tm.parse import State, Slot, Instr

    InstrSeq = list[tuple[str, int, Slot]]

    Config = tuple[int, State, BackstepTape]

    BackstepMachine = (
        BackstepMachineHalt
        | BackstepMachineBlank
        | BackstepMachineSpinout
    )

class TermType(Enum):
    # pylint: disable = invalid-name
    Halt = 0
    Blank = 1
    Spinout = 2

########################################

def cant_halt(prog: str) -> bool:
    return cant_reach(prog, TermType.Halt)

def cant_blank(prog: str) -> bool:
    return cant_reach(prog, TermType.Blank)

def cant_spin_out(prog: str) -> bool:
    return cant_reach(prog, TermType.Spinout)

########################################

def cant_reach(
        prog: str,
        term_type: TermType,
        max_steps: int = 24,
        max_cycles: int = 1_000,
) -> bool:
    slots = (
        halt_slots if term_type == TermType.Halt else
        erase_slots if term_type == TermType.Blank else
        zero_reflexive_slots
    )(prog)

    if not slots:
        return True

    program: dict[State, list[Instr]]

    entry_points: dict[State, list[State]]

    color_count, entry_points, program = reason_parse(prog)

    configs: list[Config] = [
        (1, state, BackstepTape(scan = color))
        for state, color in sorted(slots)
    ]

    seen: dict[State, set[BackstepTape]] = defaultdict(set)

    machine: BackstepMachine = (  # type: ignore[assignment]
        BackstepMachineHalt if term_type == TermType.Halt else
        BackstepMachineBlank if term_type == TermType.Blank else
        BackstepMachineSpinout
    )(prog)

    colors = tuple(range(color_count))

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

        for entry in entry_points[state]:
            for _, shift, trans in program[entry]:
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
