from __future__ import annotations

from typing import TYPE_CHECKING
from collections import defaultdict
from functools import cached_property

from tm.parse import parse
from tm.graph import Graph
from tm.tape import BackstepTape
from tm.rust_stuff import (
    BackstepMachineHalt,
    BackstepMachineBlank,
    BackstepMachineSpinout,
)

if TYPE_CHECKING:
    from collections.abc import Callable

    from tm.parse import Color, State, Slot, Instr, Switch

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
        program := Reasoner(prog),
        program.halt_slots,
        prog,
        BackstepMachineHalt,
    )


def cant_blank(prog: str) -> bool:
    return cant_reach(
        program := Reasoner(prog),
        program.erase_slots,
        prog,
        BackstepMachineBlank,
    )


def cant_spin_out(prog: str) -> bool:
    return cant_reach(
        program := Reasoner(prog),
        program.spinout_slots,
        prog,
        BackstepMachineSpinout,
    )

########################################

class Reasoner:
    prog: dict[State, Switch]

    graph: Graph

    prog_str: str

    def __init__(self, program: str):
        self.prog = {
            state: dict(enumerate(instrs))
            for state, instrs in enumerate(parse(program))
        }

        self.graph = Graph(program)

        self.prog_str = program

    def get_switch(self, state: State) -> Switch:
        return self.prog[state]

    @cached_property
    def colors(self) -> set[Color]:
        return set(range(len(self.prog[0])))

    @property
    def instr_slots(self) -> list[tuple[Slot, Instr | None]]:
        return [
            ((state, color), instr)
            for state, instrs in self.prog.items()
            for color, instr in instrs.items()
        ]

    @property
    def used_instr_slots(self) -> list[tuple[Slot, Instr]]:
        return [
            (slot, instr)
            for slot, instr in self.instr_slots
            if instr is not None
        ]

    @property
    def halt_slots(self) -> tuple[Slot, ...]:
        return tuple(
            slot
            for slot, instr in self.instr_slots
            if instr is None or instr[2] == -1
        )

    @property
    def erase_slots(self) -> tuple[Slot, ...]:
        return tuple(
            slot
            for slot, instr in self.used_instr_slots
            if slot[1] != 0 and instr[0] == 0
        )

    @property
    def spinout_slots(self) -> tuple[Slot, ...]:
        return tuple(
            (state, 0)
            for state in self.graph.zero_reflexive_states
        )

########################################

def cant_reach(
        program: Reasoner,
        slots: tuple[Slot, ...],
        prog: str,
        machine_type: Callable[[str], BackstepMachine],
        max_steps: int = 24,
        max_cycles: int = 1_000,
) -> bool:
    if not slots:
        return True

    configs: list[Config] = [
        (1, state, BackstepTape(scan = color))
        for state, color in sorted(slots)
    ]

    seen: dict[State, set[BackstepTape]] = defaultdict(set)

    machine = machine_type(prog)

    entry_points = program.graph.entry_points

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
            for _, instr in program.get_switch(entry).items():
                if instr is None:
                    continue

                _, shift, trans = instr

                if trans != state:
                    continue

                for color in program.colors:
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
