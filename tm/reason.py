from __future__ import annotations

from typing import TYPE_CHECKING
from dataclasses import dataclass
from collections import defaultdict

from tm.parse import tcompile
from tm.program import Program
from tm.tape import HeadTape

if TYPE_CHECKING:
    from collections.abc import Callable, Iterator

    from tm.parse import Prog
    from tm.program import Color, Shift, State, Slot

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
                    HeadTape(scan = color),
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
                    next_tape = BasicTape(
                        [Block(blk.color, blk.count)
                             for blk in tape.lspan],
                        tape.scan,
                        [Block(blk.color, blk.count)
                             for blk in tape.rspan],
                    )

                    next_tape.backstep(shift, color)

                    machine.run(
                        sim_lim = step + 1,
                        tape = next_tape,
                        state = entry,
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

class BasicMachine:
    comp: Prog

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
            tape: BasicTape,
    ) -> None:
        comp = self.comp

        self.blanks = {}

        step: int = 0

        for _ in range(sim_lim):

            try:
                instr = comp[state, tape.scan]
            except KeyError:
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


@dataclass(slots = True)
class Block:
    color: Color
    count: int


@dataclass(slots = True)
class BasicTape:
    lspan: list[Block]
    scan: Color
    rspan: list[Block]

    @property
    def blank(self) -> bool:
        return self.scan == 0 and not self.lspan and not self.rspan

    def at_edge(self, edge: Shift) -> bool:
        return (
            self.scan == 0
            and not (self.rspan if edge else self.lspan)
        )

    def backstep(self, shift: Shift, color: Color) -> None:
        _ = self.step(
            not shift,
            self.scan,
            False,
        )

        self.scan = color

    def step(self, shift: Shift, color: Color, skip: bool) -> int:
        pull, push = (
            (self.rspan, self.lspan)
            if shift else
            (self.lspan, self.rspan)
        )

        push_block = (
            pull.pop(0)
            if skip and pull and pull[0].color == self.scan else
            None
        )

        stepped = 1 if push_block is None else 1 + push_block.count

        next_scan: Color

        if not pull:
            next_scan = 0
        else:
            next_pull = pull[0]

            if next_pull.count != 1:
                next_pull.count -= 1
            else:
                popped = pull.pop(0)

                if push_block is None:
                    push_block = popped
                    push_block.count = 0

            next_scan = next_pull.color

        if push and (top_block := push[0]).color == color:
            top_block.count += stepped
        elif push or color != 0:
            if push_block is None:
                push_block = Block(color, 1)
            else:
                push_block.color = color
                push_block.count += 1

            push.insert(0, push_block)

        self.scan = next_scan

        return stepped


if TYPE_CHECKING:
    GetResult = Callable[[BasicMachine], int | None]

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
