from __future__ import annotations

from typing import TYPE_CHECKING
from collections import defaultdict
from dataclasses import dataclass, field

from tm.rules import make_rule, UnknownRule

if TYPE_CHECKING:
    from tm.rules import Rule
    from tm.instrs import State, Slot, GetInstr
    from tm.tape import Signature, Tape, BlockTape, TagTape, EnumTape

    MinSig = tuple[Signature, tuple[bool, bool]]


@dataclass
class PastConfig:
    cycles: list[int] = field(default_factory = list)

    def next_deltas(self, cycle: int) -> tuple[int, int] | None:
        (cycles := self.cycles).append(cycle)

        if len(cycles) < 5:
            return None

        # pylint: disable = invalid-name, unbalanced-tuple-unpacking

        e, d, c, b, a = cycles

        cycles.pop(0)

        for i in range(1, 5):
            p1 = a - (b * i)
            p2 = b - (c * i)
            p3 = c - (d * i)
            p4 = d - (e * i)

            if (diff := p1 - p2) == p2 - p3 == p3 - p4:
                nxt = a * i + p1 + diff
                nxxt = nxt * i + p1 + 2 * diff

                return (
                    nxt - cycle,
                    nxxt - nxt,
                )

        return None


@dataclass
class Prover:
    prog: GetInstr

    rules: dict[
        Slot,
        list[tuple[MinSig, Rule]],
    ] = field(default_factory = dict)

    configs: dict[
        Signature, dict[
            State,
            PastConfig,
        ],
    ] = field(default_factory = dict)

    @property
    def config_count(self) -> int:
        return len(self.configs)

    def get_rule(
            self,
            state: State,
            tape: BlockTape,
            sig: Signature | None = None,
    ) -> Rule | None:
        if (temp := self.rules.get((state, tape.scan))) is None:
            return None

        if sig is None:
            sig = tape.signature

        for ((scan, lspan, rspan), (lex, rex)), rule in temp:
            if (scan == sig[0]
                and lspan == (sig[1] if lex else sig[1][:len(lspan)])
                and rspan == (sig[2] if rex else sig[2][:len(rspan)])):
                return rule

        return None

    def set_rule(
            self,
            rule: Rule,
            state: State,
            sig: MinSig,
    ) -> None:
        if (slot := (state, sig[0][0])) not in self.rules:
            self.rules[slot] = []

        self.rules[slot].append((sig, rule))

    def run_simulator(
            self,
            steps: int,
            state: State,
            tape: TagTape,
    ) -> State | None:
        for _ in range(steps):
            if (rule := self.get_rule(state, tape)) is not None:
                if tape.apply_rule(rule) is not None:
                    continue

            # pylint: disable = line-too-long
            if (instr := self.prog[state, tape.scan]) is None:  # no-coverage
                return None

            color, shift, next_state = instr

            tape.step(shift, color, state == next_state)

            if (state := next_state) == -1:  # no-coverage
                return None

        return state

    def get_min_sig(
            self,
            steps: int,
            state: State,
            tape: EnumTape,
            sig: Signature,
    ) -> MinSig:
        for _ in range(steps):
            if (rule := self.get_rule(state, tape)) is not None:
                if tape.apply_rule(rule) is not None:
                    continue

            assert (instr := self.prog[state, tape.scan]) is not None

            color, shift, next_state = instr

            tape.step(shift, color, state == next_state)

            state = next_state

        lmax, rmax = tape.offsets

        return (sig[0], sig[1][:lmax], sig[2][:rmax]), tape.edges

    def try_rule(
            self,
            cycle: int,
            state: State,
            tape: Tape,
    ) -> Rule | None:
        if (known_rule := self.get_rule(
                state, tape, sig := tape.signature)) is not None:
            return known_rule

        if (states := self.configs.get(sig)) is None:
            states = defaultdict(PastConfig)
            self.configs[sig] = states

        if (deltas := states[state].next_deltas(cycle)) is None:
            return None

        tags: TagTape = tape.to_tag()

        for s, span in enumerate(tags.spans):
            for i, block in enumerate(span):
                if block.count > 1:
                    block.tags.append(2 * i + s)

        counts = []

        for delta in deltas:
            if (
                self.run_simulator(delta, state, tags) != state
                or tags.scan != sig[0]
                or tags.signature != sig
                or tags.missing_tags()
            ):
                return None

            counts.append(tags.counts)

        assert len(counts) == 2

        try:
            rule = make_rule(tape.counts, counts[0], counts[1])
        except UnknownRule:
            return None

        del states[state]

        self.set_rule(
            rule,
            state,
            self.get_min_sig(
                deltas[0],
                state,
                tape.to_enum(),
                sig,
            ),
        )

        # print(f'--> proved rule: {rule}')

        return rule
