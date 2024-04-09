from __future__ import annotations

from typing import TYPE_CHECKING

from tm.rules import apply_rule, make_rule
from tm.rust_stuff import PastConfigs

if TYPE_CHECKING:
    from tm.rules import Rule
    from tm.parse import State, Slot, GetInstr
    from tm.tape import Signature, Tape, BlockTape, TagTape, EnumTape

    MinSig = tuple[Signature, tuple[bool, bool]]


class ConfigLimit(Exception):
    pass


class Prover:
    prog: GetInstr

    rules: dict[
        Slot,
        list[tuple[MinSig, Rule]],
    ]

    configs: dict[Signature, PastConfigs]

    def __init__(self, prog: GetInstr):
        self.prog = prog
        self.rules = {}
        self.configs = {}

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
                if apply_rule(rule, tape) is not None:
                    continue

            try:
                instr = self.prog[state, tape.scan]
            except KeyError:
                return None

            color, shift, next_state = instr

            tape.step(shift, color, state == next_state)

            state = next_state

        return state

    def get_min_sig(
            self,
            steps: int,
            state: State,
            tape: EnumTape,
            sig: Signature,
    ) -> MinSig:
        for _ in range(steps):
            if (rule := self.get_rule(state, tape.tape)) is not None:
                if apply_rule(rule, tape) is not None:
                    continue

            color, shift, next_state = self.prog[state, tape.tape.scan]

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

        if (past_configs := self.configs.get(sig)) is None:
            if self.config_count > 100_000:  # no-cover
                raise ConfigLimit

            past_configs = PastConfigs()
            self.configs[sig] = past_configs

        if (deltas := past_configs.next_deltas(state, cycle)) is None:
            return None

        if any(delta > 90_000 for delta in deltas):
            return None

        tags: TagTape = tape.to_tag()

        counts = []

        for delta in deltas:
            if (
                self.run_simulator(delta, state, tags) != state
                or not tags.sig_compatible(sig)
                or tags.missing_tags
            ):
                return None

            counts.append(tags.counts)

        assert len(counts) == 3

        if (rule := make_rule(tape.counts, *counts)) is None:
            return None

        past_configs.delete_configs(state)

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
