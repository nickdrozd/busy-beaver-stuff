from __future__ import annotations

from enum import Enum
from typing import TYPE_CHECKING
from collections import defaultdict

from tm.program import Program
from tm.machine import (
    HeadTape,
    History,
    BasicMachine,
)

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    from tm.program import State, Slot

    InstrSeq = list[tuple[str, int, Slot]]

    Config = tuple[int, State, HeadTape]

    GetResult = Callable[[BasicMachine], int | None]


class Result(Enum):
    # pylint: disable = invalid-name
    halted = 0
    blanks = 1
    spnout = 2


class BackwardReasoner(Program):
    @property
    def instr_seq(self) -> InstrSeq:
        seqs: InstrSeq = []

        partial = Program.init(len(self.states), len(self.colors))

        machine = BasicMachine(partial)

        for _ in range(len(self.states) * len(self.colors) - 1):
            if (result := machine.run().undfnd) is None:
                return seqs

            step, slot = result

            seqs.append((str(partial), step, slot))

            partial[slot] = self[slot]

            machine.undfnd = None

        return seqs

    @property
    def cant_halt(self) -> bool:
        return self.cant_reach(
            Result.halted,
            self.halt_slots,
        )

    @property
    def cant_blank(self) -> bool:
        return self.cant_reach(
            Result.blanks,
            self.erase_slots,
        )

    @property
    def cant_spin_out(self) -> bool:
        return self.cant_reach(
            Result.spnout,
            self.spinout_slots,
        )

    def cant_reach(
            self,
            final_prop: Result,
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

        get_result = final_value(final_prop)

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
                        tape = next_tape.copy(),
                        state = entry,
                    )

                    if not (result := get_result(machine)):
                        continue

                    if abs(result - step) > 1:
                        continue

                    yield (
                        step + 1,
                        entry,
                        next_tape,
                    )


def final_value(final_prop: Result) -> GetResult:
    # pylint: disable = function-redefined
    match final_prop:
        case Result.spnout:
            def result(machine: BasicMachine) -> int | None:
                final = machine.spnout
                machine.spnout = None
                return final
        case Result.blanks:
            def result(machine: BasicMachine) -> int | None:
                final = (
                    min(blanks.values())
                    if (blanks := machine.blanks) else
                    None
                )
                machine.blanks = {}
                return final
        case Result.halted:  # no-branch
            def result(machine: BasicMachine) -> int | None:
                final: int | None
                if (und := machine.undfnd):
                    final = und[0]
                    machine.undfnd = None
                else:
                    final = machine.halted
                    machine.halted = None
                return final

    return result

########################################

def instr_seq(prog: str) -> InstrSeq:
    return BackwardReasoner(prog).instr_seq


def cant_halt(prog: str) -> bool:
    return BackwardReasoner(prog).cant_halt


def cant_blank(prog: str) -> bool:
    return BackwardReasoner(prog).cant_blank


def cant_spin_out(prog: str) -> bool:
    return BackwardReasoner(prog).cant_spin_out
