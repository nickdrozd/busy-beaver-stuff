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
    return Reasoner(prog).cant_reach(
        halt_slots(prog),
        lambda: BackstepMachineHalt(prog),
    )


def cant_blank(prog: str) -> bool:
    return Reasoner(prog).cant_reach(
        erase_slots(prog),
        lambda: BackstepMachineBlank(prog),
    )


def cant_spin_out(prog: str) -> bool:
    return (program := Reasoner(prog)).cant_reach(
        program.spinout_slots,
        lambda: BackstepMachineSpinout(prog),
    )

########################################

class Reasoner:
    prog: dict[State, list[Instr | None]]

    graph: Graph

    def __init__(self, program: str):
        self.prog = dict(enumerate(parse(program)))

        self.graph = Graph(program)

    @property
    def spinout_slots(self) -> list[Slot]:
        return [
            (state, 0)
            for state in self.graph.zero_reflexive_states
        ]

    def cant_reach(
            self,
            slots: list[Slot],
            get_machine: Callable[[], BackstepMachine],
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

        machine = get_machine()

        entry_points = self.graph.entry_points

        colors = self.graph.colors

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
                for instr in self.prog[entry]:
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
