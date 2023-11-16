from __future__ import annotations

from typing import TYPE_CHECKING
from collections import defaultdict

from tm.program import Program
from tm.machine import QuickMachine as BasicMachine
from tm.lin_rec import History, HeadTape

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    from tm.program import State, Slot

    InstrSeq = list[tuple[str, int, Slot]]

    Config = tuple[int, State, HeadTape]

    GetResult = Callable[[BasicMachine], int | None]


class BackwardReasoner(Program):
    @property
    def cant_halt(self) -> bool:
        def get_result(machine: BasicMachine) -> int | None:
            final: int | None
            if (und := machine.undfnd):
                final = und[0]
                machine.undfnd = None
            else:
                final = machine.halted
                machine.halted = None
            return final

        return self.cant_reach(
            get_result,
            self.halt_slots,
        )

    @property
    def cant_blank(self) -> bool:
        def get_result(machine: BasicMachine) -> int | None:
            final = (
                min(blanks.values())
                if (blanks := machine.blanks) else
                None
            )
            machine.blanks = {}
            return final

        return self.cant_reach(
            get_result,
            self.erase_slots,
        )

    @property
    def cant_spin_out(self) -> bool:
        def get_result(machine: BasicMachine) -> int | None:
            final = machine.spnout
            machine.spnout = None
            return final

        return self.cant_reach(
            get_result,
            self.spinout_slots,
        )

    def cant_reach(
            self,
            get_result: GetResult,
            slots: tuple[Slot, ...],
            max_steps: int = 24,
            max_cycles: int = 1_000,
    ) -> bool:
        configs: list[tuple[Config, int, History]] = [
            (
                (
                    1,
                    state,
                    HeadTape.init(color),
                ),
                0,
                History(tapes = {}),
            )
            for state, color in sorted(slots)
        ]

        machine = BasicMachine(str(self))

        max_repeats = max_steps // 2

        seen: dict[State, set[HeadTape]] = defaultdict(set)

        for _ in range(max_cycles):
            try:
                (step, state, tape), repeat, history = configs.pop()
            except IndexError:
                return True

            if step > max_steps:
                return False

            if state == 0 and tape.blank:
                return False

            if tape in seen[state]:
                continue

            seen[state].add(tape)

            history.add_tape_at_step(step, tape)

            if history.check_rec(
                    step,
                    slot := (state, tape.scan)) is None:
                repeat = 0
            else:
                repeat += 1

                if repeat > max_repeats:
                    continue

            history.add_slot_at_step(step, slot)

            # print(step, state, tape)

            for config in self.branch_back(
                    step, state, tape, machine, get_result):
                configs.append((
                    config,
                    repeat,
                    history.copy(),
                ))

        return False

    def branch_back(
            self,
            step: int,
            state: State,
            tape: HeadTape,
            machine: BasicMachine,
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
                    next_tape = tape.copy()

                    _ = next_tape.step(
                        not shift,
                        next_tape.scan,
                        False,
                    )

                    next_tape.scan = color

                    machine = machine.run(
                        sim_lim = step + 1,
                        tape = next_tape,
                        state = entry,
                    )

                    if not (result := get_result(machine)):
                        continue

                    if abs(result - step) > 1:
                        continue

                    yield_tape = tape.copy()

                    _ = yield_tape.step(
                        not shift,
                        yield_tape.scan,
                        False,
                    )

                    yield_tape.scan = color

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
