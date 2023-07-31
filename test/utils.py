from __future__ import annotations

from typing import Self, TYPE_CHECKING

from tm.machine import (
    History,
    HeadTape,
    BasicMachine,
)

if TYPE_CHECKING:
    from tm.machine import State, Slot, Tapes


class LinRecSampler(BasicMachine):
    history: History

    def run(  # type: ignore[override]  # pylint: disable = arguments-differ
        self,
        sim_lim: int,
        samples: Tapes,
    ) -> Self:
        self.blanks = {}

        comp = self.comp

        self.tape = tape = HeadTape.init()

        self.history = History(tapes = samples)

        step: int = 0
        state: State = 0

        for cycle in range(sim_lim or 1_000_000):
            slot: Slot = state, tape.scan

            if step in self.history.tapes:
                self.history.add_state_at_step(step, state)
                self.history.add_tape_at_step(step, tape)

            if (instr := comp[slot]) is None:
                self.undfnd = step, slot
                break

            color, shift, next_state = instr

            step += tape.step(shift, color, False)

            # pylint: disable = duplicate-code
            if (state := next_state) == -1:
                self.halted = step
                break

            if not color and tape.blank and state not in self.blanks:
                self.blanks[state] = step

        else:
            self.xlimit = step

        self.cycles = cycle

        return self
