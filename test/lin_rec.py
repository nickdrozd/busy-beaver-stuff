from __future__ import annotations

from typing import Self, TYPE_CHECKING

from tm.lin_rec import BeepHistory, HeadTape, LinRecMachine

if TYPE_CHECKING:
    from tm.lin_rec import State, Slot, Tapes


class LinRecSampler(LinRecMachine):
    history: BeepHistory

    def run(
        self,
        sim_lim: int,
        samples: Tapes,
    ) -> Self:
        self.blanks = {}

        comp = self.comp

        self.tape = tape = HeadTape.init()

        self.history = BeepHistory(tapes = samples)

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

            if (state := next_state) == -1:
                self.halted = step
                break

            if not color and tape.blank and state not in self.blanks:
                self.blanks[state] = step

        else:
            self.xlimit = step

        self.cycles = cycle

        return self


class LooseLinRecMachine(LinRecMachine):
    # pylint: disable = while-used, too-many-locals, line-too-long
    def run(self, sim_lim: int) -> Self:  # no-cover
        self.blanks = {}

        comp = self.comp

        state = 1

        step = 1

        self.tape = tape = HeadTape.init_stepped()

        cycle = 1

        while cycle < sim_lim:
            steps_reset = 2 * step

            leftmost = rightmost = init_pos = tape.head

            init_state = state

            init_tape = tape.to_ptr()

            while step < steps_reset and cycle < sim_lim:
                if (instr := comp[state, tape.scan]) is None:
                    self.undfnd = step, (state, tape.scan)
                    break

                color, shift, next_state = instr

                if (same := state == next_state) and tape.at_edge(shift):
                    self.spnout = step
                    break

                stepped = tape.step(shift, color, same)

                step += stepped

                cycle += 1

                if (state := next_state) == -1:
                    self.halted = step
                    break

                if not color and tape.blank:
                    if state in self.blanks:
                        self.infrul = step
                        break

                    self.blanks[state] = step

                    if state == 0:
                        self.infrul = step
                        break

                if (curr := tape.head) < leftmost:
                    leftmost = curr
                elif rightmost < curr:
                    rightmost = curr

                if state != init_state:
                    continue

                if tape.scan != init_tape.scan:
                    continue

                ptr = tape.to_ptr()

                if 0 < (diff := curr - init_pos):
                    slice1 = init_tape.get_ltr(leftmost)
                    slice2 = ptr.get_ltr(leftmost + diff)

                elif diff < 0:
                    slice1 = init_tape.get_rtl(rightmost)
                    slice2 = ptr.get_rtl(rightmost + diff)

                else:
                    slice1 = init_tape.get_cnt(leftmost, rightmost)
                    slice2 = ptr.get_cnt(leftmost, rightmost)

                if slice1 == slice2:
                    self.infrul = step
                    break

            else:
                continue

            self.xlimit = None

            break

        else:
            self.xlimit = stepped

        self.cycles = cycle

        return self
