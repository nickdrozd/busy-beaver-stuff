from __future__ import annotations

from typing import TYPE_CHECKING
from dataclasses import dataclass
from collections import defaultdict

from tm.parse import tcompile
from tm.program import Program
from tm.tape import HeadTape

if TYPE_CHECKING:
    from typing import Self

    from collections.abc import Callable, Iterator

    from tm.tape import PtrTape
    from tm.program import Color, State, Slot

    InstrSeq = list[tuple[str, int, Slot]]

    Config = tuple[int, State, HeadTape]


class BackwardReasoner(Program):
    @property
    def cant_halt(self) -> bool:
        def get_result(machine: BasicMachine) -> int | None:
            final: int | None
            if (und := machine.undfnd):
                final = und
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
                History(),
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

            history.update(state, tape.scan, tape)

            if not history.check_rec():
                repeat = 0
            else:
                repeat += 1

                if repeat > max_repeats:
                    continue

            # print(step, state, tape)

            for config in self.branch_back(
                    step, state, tape, machine, get_result):
                next_history = History(prev = history)

                configs.append((
                    config,
                    repeat,
                    next_history,
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

########################################

class BasicMachine:
    blanks: dict[State, int]

    halted: int | None = None
    spnout: int | None = None
    undfnd: int | None = None

    def __init__(self, prog: str):
        self.comp = tcompile(prog)

    def run(self,
            *,
            sim_lim: int,
            state: State,
            tape: HeadTape,
    ) -> Self:
        comp = self.comp

        self.blanks = {}

        step: int = 0

        for _ in range(sim_lim):

            if (instr := comp[state, tape.scan]) is None:
                self.undfnd = step
                break

            color, shift, next_state = instr

            if (same := state == next_state) and tape.at_edge(shift):
                self.spnout = step
                break

            stepped = tape.step(shift, color, same)

            step += stepped

            if (state := next_state) == -1:
                self.halted = step
                break

            if not color and tape.blank:
                if state in self.blanks:  # no-cover
                    break

                self.blanks[state] = step

                if state == 0:
                    break

        return self


if TYPE_CHECKING:
    GetResult = Callable[[BasicMachine], int | None]

########################################

@dataclass
class History:
    state: State | None = None
    scan: Color | None = None
    head: int | None = None
    tape: PtrTape | None = None
    prev: History | None = None

    def update(self, state: State, scan: Color, tape: HeadTape) -> None:
        self.state = state
        self.scan = scan
        self.head = tape.head
        self.tape = tape.to_ptr()

    def __iter__(self) -> Iterator[History]:
        prev = self.prev

        while prev:  # pylint: disable = while-used
            yield prev

            prev = prev.prev

    def check_rec(self) -> bool:
        curr_head = self.head
        assert curr_head is not None

        leftmost = rightmost = curr_head

        curr_tape = self.tape
        assert curr_tape is not None

        for prev in self:
            if self.state != prev.state or self.scan != prev.scan:
                continue

            prev_head = prev.head
            assert prev_head is not None

            if prev_head < leftmost:
                leftmost = prev_head
            elif rightmost < prev_head:
                rightmost = prev_head

            prev_tape = prev.tape
            assert prev_tape is not None

            if curr_tape.aligns_with(
                    prev_tape,
                    curr_head - prev_head,
                    leftmost,
                    rightmost,
            ):
                return True

        return False
