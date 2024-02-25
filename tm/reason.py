from __future__ import annotations

from typing import TYPE_CHECKING
from collections import defaultdict

from tm.program import Program
from tm.tape import HeadTape
from tm.rust_stuff import BackstepMachine

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    from tm.program import State, Slot

    InstrSeq = list[tuple[str, int, Slot]]

    Config = tuple[int, State, HeadTape]

    GetResult = Callable[[BackstepMachine], int | None]


class BackwardReasoner(Program):
    @property
    def cant_halt(self) -> bool:
        return self.cant_reach(
            lambda machine: machine.get_halt(),
            self.halt_slots,
        )

    @property
    def cant_blank(self) -> bool:
        return self.cant_reach(
            lambda machine: machine.get_min_blank(),
            self.erase_slots,
        )

    @property
    def cant_spin_out(self) -> bool:
        return self.cant_reach(
            lambda machine: machine.get_spinout(),
            self.spinout_slots,
        )

    def cant_reach(
            self,
            get_result: GetResult,
            slots: tuple[Slot, ...],
            max_steps: int = 24,
            max_cycles: int = 1_000,
    ) -> bool:
        configs: list[Config] = [
            (1, state, HeadTape(scan = color))
            for state, color in sorted(slots)
        ]

        machine = BackstepMachine(str(self))

        seen: dict[State, set[HeadTape]] = defaultdict(set)

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

            for config in self.branch_back(
                    step, state, tape, machine, get_result):
                configs.append(config)

        return False  # no-cover

    def branch_back(
            self,
            step: int,
            state: State,
            tape: HeadTape,
            machine: BackstepMachine,
            get_result: GetResult,
    ) -> Iterator[Config]:
        for entry in sorted(self.graph.entry_points[state]):
            for _, instr in self.get_switch(entry).items():
                if instr is None:
                    continue

                _, shift, trans = instr

                if trans != state:
                    continue

                for color in self.colors:
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

                    yield (
                        step + 1,
                        entry,
                        yield_tape,
                    )

########################################

def cant_halt(prog: str) -> bool:
    return BackwardReasoner(prog).cant_halt


def cant_blank(prog: str) -> bool:
    return BackwardReasoner(prog).cant_blank


def cant_spin_out(prog: str) -> bool:
    return BackwardReasoner(prog).cant_spin_out
