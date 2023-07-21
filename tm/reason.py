from __future__ import annotations

from typing import TYPE_CHECKING
from collections import defaultdict

from tm.program import Program
from tm.machine import (
    HeadTape,
    History,
    Machine,
    tcompile,
)

if TYPE_CHECKING:
    from tm.program import State, Slot

    InstrSeq = list[tuple[str, int, Slot]]


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
        return self._cant_reach(
            'halted',
            self.halt_slots,
        )

    @property
    def cant_blank(self) -> bool:
        return self._cant_reach(
            'blanks',
            self.erase_slots,
        )

    @property
    def cant_spin_out(self) -> bool:
        return self._cant_reach(
            'spnout',
            self.spinout_slots,
        )

    def _cant_reach(
            self,
            final_prop: str,
            slots: tuple[Slot, ...],
            max_steps: int = 24,
            max_cycles: int = 1_000,
    ) -> bool:
        configs: list[
            tuple[int, State, HeadTape, int, History]
        ] = [
            (
                1,
                state,
                HeadTape.init(color),
                0,
                History(tapes = {}),
            )
            for state, color in sorted(slots)
        ]

        comp = tcompile(str(self))

        max_repeats = max_steps // 2

        seen: dict[State, set[HeadTape]] = defaultdict(set)

        for _ in range(max_cycles):
            try:
                step, state, tape, repeat, history = configs.pop()
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

                        run = Machine(comp).run(
                            sim_lim = step + 1,
                            tape = next_tape.copy(),
                            state = entry,
                        )

                        if not (result := final_value(final_prop, run)):
                            continue

                        if abs(result - step) > 1:
                            continue

                        configs.append((
                            step + 1,
                            entry,
                            next_tape,
                            repeat,
                            history.copy(),
                        ))

        return False


def final_value(final_prop: str, machine: Machine) -> int | None:
    match final_prop:
        case 'spnout':
            return machine.spnout
        case 'blanks':
            return (
                min(blanks.values())
                if (blanks := machine.blanks) else
                None
            )
        case 'halted':
            return (
                und[0]
                if (und := machine.undfnd) else
                machine.halted
            )

    return None  # no-coverage
