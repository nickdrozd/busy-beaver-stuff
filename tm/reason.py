from __future__ import annotations

from typing import TYPE_CHECKING
from dataclasses import dataclass
from collections import defaultdict

from tm.program import Program
from tm.tape import HeadTape
from tm.rust_stuff import BackstepMachine

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    from tm.program import Color, State, Slot

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
        configs: list[tuple[Config, int, History]] = [
            (
                (
                    1,
                    state,
                    HeadTape(scan = color),
                ),
                0,
                History(),
            )
            for state, color in sorted(slots)
        ]

        machine = BackstepMachine(str(self))

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

########################################

@dataclass(slots = True)
class History:
    state: State | None = None
    scan: Color | None = None
    head: int | None = None
    tape: HeadTape | None = None
    prev: History | None = None

    def update(self, state: State, scan: Color, tape: HeadTape) -> None:
        self.state = state
        self.scan = scan
        self.head = tape.head
        self.tape = tape.copy()

    def __iter__(self) -> Iterator[History]:
        prev = self.prev

        while prev:  # pylint: disable = while-used
            yield prev

            prev = prev.prev

    def check_rec(self) -> bool:
        assert (curr_head := self.head) is not None

        leftmost = rightmost = curr_head

        curr_tape = self.tape
        assert curr_tape is not None

        for prev in self:
            if self.state != prev.state or self.scan != prev.scan:
                continue

            assert (prev_head := prev.head) is not None

            if prev_head < leftmost:
                leftmost = prev_head
            elif rightmost < prev_head:
                rightmost = prev_head

            prev_tape = prev.tape
            assert prev_tape is not None

            if curr_tape.aligns_with(prev_tape, leftmost, rightmost):
                return True

        return False
