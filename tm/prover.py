from typing import TYPE_CHECKING

from tm.rules import Plus, apply_rule, make_rule
from tm.rust_stuff import PastConfigs

if TYPE_CHECKING:
    from tm.macro import GetInstr, Slot, State
    from tm.rules import Rule
    from tm.tape import EnumTape, MinSig, Signature, Tape


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
    def has_mult_rules(self) -> bool:
        return any(
            not isinstance(diff, Plus)
            for rules in self.rules.values()
            for (_, rule) in rules
            for diff in rule.values()
        )

    @property
    def config_count(self) -> int:
        return len(self.configs)

    def get_rule(
            self,
            state: State,
            tape: Tape | EnumTape,
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
            steps: int,
            state: State,
            tape: Tape,
            sig: Signature,
    ) -> None:
        enum_tape = tape.to_enum()

        _ = self.run_simulator(steps, state, enum_tape)

        min_sig = enum_tape.get_min_sig(sig)

        if (slot := (state, tape.scan)) not in self.rules:
            self.rules[slot] = []

        self.rules[slot].append((min_sig, rule))

    def run_simulator(
            self,
            steps: int,
            state: State,
            tape: Tape | EnumTape,
    ) -> State | None:
        for _ in range(steps):
            if ((rule := self.get_rule(state, tape)) is not None
                    and apply_rule(rule, tape) is not None):
                continue

            try:
                instr = self.prog[state, tape.scan]
            except KeyError:
                return None

            color, shift, next_state = instr

            tape.step(shift, color, state == next_state)

            state = next_state

        return state

    def try_rule(
            self,
            cycle: int,
            state: State,
            tape: Tape,
    ) -> Rule | None:
        sig = tape.signature

        if (known := self.get_rule(state, tape, sig)) is not None:
            return known

        if (past_configs := self.configs.get(sig)) is None:
            if self.config_count > 100_000:  # no-cover
                raise ConfigLimit

            self.configs[sig] = PastConfigs(state, cycle)
            return None

        if (deltas := past_configs.next_deltas(state, cycle)) is None:
            return None

        if (rule := self.prove_rule(deltas, state, tape, sig)) is None:
            return None

        past_configs.delete_configs(state)

        self.set_rule(rule, deltas[0], state, tape, sig)

        # print(f'--> proved rule: {rule}')

        return rule

    def prove_rule(
            self,
            deltas: tuple[int, int, int],
            state: State,
            tape: Tape,
            sig: Signature,
    ) -> Rule | None:
        tags = tape.clone()

        counts = []

        for delta in deltas:
            if (
                self.run_simulator(delta, state, tags) != state
                or not tags.sig_compatible(sig)
            ):
                return None

            counts.append(tags.counts)

        assert len(counts) == 3

        if (rule := make_rule(tape.counts, *counts)) is None:
            return None

        if (tape.length_one_spans
                and len(rule) == 2
                and all(isinstance(val, int) for val in rule.values())
                and len({
                    abs(val) for val in rule.values()
                    if isinstance(val, int)}) == 1):
            return None

        return rule
