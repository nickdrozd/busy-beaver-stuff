from __future__ import annotations

from enum import Enum
from typing import TYPE_CHECKING
from collections import defaultdict

from tm.program import Program
from tm.machine import (
    HeadTape,
    History,
    Machine,
)

if TYPE_CHECKING:
    from tm.program import State, Slot

    InstrSeq = list[tuple[str, int, Slot]]

    Config = tuple[int, State, HeadTape]


Result = Enum('Result', ('halted', 'blanks', 'spnout'))


class BackwardReasoner(Program):
    @property
    def instr_seq(self) -> InstrSeq:
        seqs: InstrSeq = []

        partial = Program.init(len(self.states), len(self.colors))

        for _ in range(len(self.states) * len(self.colors) - 1):
            if (result := Machine(partial).run().undfnd) is None:
                return seqs

            step, slot = result

            seqs.append((str(partial), step, slot))

            partial[slot] = self[slot]

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

        machine = Machine(str(self))

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

            history.add_state_at_step(step, state)
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

                        run = machine.run(
                            sim_lim = step + 1,
                            tape = next_tape.copy(),
                            state = entry,
                        )

                        if not (result := final_value(final_prop, run)):
                            continue

                        if abs(result - step) > 1:
                            continue

                        configs.append((
                            (
                                step + 1,
                                entry,
                                next_tape,
                            ),
                            repeat,
                            history.copy(),
                        ))

        return False


def final_value(final_prop: Result, machine: Machine) -> int | None:
    match final_prop:
        case Result.spnout:
            final = machine.spnout
            machine.spnout = None
        case Result.blanks:
            final = (
                min(blanks.values())
                if (blanks := machine.blanks) else
                None
            )
            machine.blanks = {}
        case Result.halted:
            if (und := machine.undfnd):
                final = und[0]
                machine.undfnd = None
            else:
                final = machine.halted
                machine.halted = None

    return final
