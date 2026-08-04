use std::collections::hash_map::Entry;

use ahash::AHashMap as Dict;

use crate::{
    Slot, State, Steps,
    config::Config,
    machine::RunProver,
    rules::{ApplyRule, Rule, make_rule},
    tape::{
        DynamicEnumTape, DynamicGetSig, DynamicMinSig,
        DynamicSignature, DynamicTape, DynamicTapeOps, GetSig,
        Signature,
    },
};

/**************************************/

pub enum ProverResult {
    ConfigLimit,
    InfiniteRule,
    Got(Rule),
}

use ProverResult::*;

pub struct Prover {
    rules: Dict<Slot, Vec<(DynamicMinSig, Rule)>>,
    configs: Dict<DynamicSignature, PastConfigs>,
    attempts: usize,
}

impl Prover {
    pub fn new() -> Self {
        Self {
            rules: Dict::new(),
            configs: Dict::new(),
            attempts: 0,
        }
    }

    pub fn config_count(&self) -> usize {
        self.configs.len()
    }

    fn get_dynamic_rule_for<T: DynamicGetSig>(
        &self,
        state: State,
        tape: &T,
        sig: Option<&DynamicSignature>,
    ) -> Option<&Rule> {
        let rules = self.rules.get(&(state, tape.scan()))?;

        let owned_sig;
        #[expect(clippy::option_if_let_else)]
        let sig = if let Some(sig) = sig {
            sig
        } else {
            owned_sig = tape.dynamic_signature();
            &owned_sig
        };

        rules
            .iter()
            .find(|(min_sig, rule)| {
                sig.matches(min_sig)
                    && rule.0.keys().all(|&(side, pos)| {
                        let len = if side {
                            sig.rspan.len()
                        } else {
                            sig.lspan.len()
                        };
                        pos < len
                    })
            })
            .map(|(_, rule)| rule)
    }

    // Preserve the original public lookup API for callers replaying rules on
    // an ordinary run-length tape. Rules involving compound dynamic words do
    // not match the converted one-symbol signature and are simply ignored.
    pub fn get_rule<T: GetSig>(
        &self,
        config: &Config<T>,
        sig: Option<&Signature>,
    ) -> Option<&Rule> {
        let owned_sig;
        #[expect(clippy::option_if_let_else)]
        let sig = if let Some(sig) = sig {
            sig
        } else {
            owned_sig = config.tape.signature();
            &owned_sig
        };
        let dynamic_sig = DynamicSignature::from(sig);

        self.get_dynamic_rule_for(
            config.state,
            &StaticDynamicSig {
                scan: config.tape.scan(),
                sig: dynamic_sig,
            },
            None,
        )
    }

    fn run_simulator<T>(
        &self,
        steps: Steps,
        mut state: State,
        tape: &mut T,
        prog: &impl RunProver,
    ) -> Option<State>
    where
        T: ApplyRule + DynamicTapeOps,
    {
        for _ in 0..steps {
            if let Some(rule) =
                self.get_dynamic_rule_for(state, tape, None)
                && tape.apply_rule(rule).is_some()
            {
                tape.normalize_dynamic();
                continue;
            }

            let slot = (state, tape.scan());
            let (color, shift, next_state) =
                prog.get_instr(&slot).ok().flatten()?;

            tape.mstep(shift, color, state == next_state);
            state = next_state;
        }

        Some(state)
    }

    fn set_rule(
        &mut self,
        rule: &Rule,
        steps: Steps,
        config: &Config<DynamicTape>,
        sig: &DynamicSignature,
        prog: &impl RunProver,
    ) {
        // Keep dependency replay in the exact block coordinate system in
        // which the candidate signature and rule were observed. Rebalancing
        // here would change block indices while `sig` and `rule` still refer
        // to the live tape's original coordinates.
        let replay_tape = config.tape.clone();
        let mut enum_tape = DynamicEnumTape::from(&replay_tape);

        let _ = self.run_simulator(
            steps,
            config.state,
            &mut enum_tape,
            prog,
        );

        let (mut min_sig, exact) = enum_tape.get_min_sig(sig);

        // A rule must never refer to a block outside the signature guarding
        // it, even if conservative dependency tracking missed that count.
        for &(side, pos) in rule.0.keys() {
            let source = if side { &sig.rspan } else { &sig.lspan };
            let target = if side {
                &mut min_sig.rspan
            } else {
                &mut min_sig.lspan
            };
            let needed = (pos + 1).min(source.len());
            if target.len() < needed {
                target.extend_from_slice(&source[target.len()..needed]);
            }
        }

        self.rules
            .entry(config.slot())
            .or_default()
            .push(((min_sig, exact), rule.clone()));
    }

    pub fn try_rule(
        &mut self,
        cycle: usize,
        config: &Config<DynamicTape>,
        prog: &impl RunProver,
    ) -> Option<ProverResult> {
        let sig = config.tape.dynamic_signature();

        if let Some(rule) = self.get_dynamic_rule_for(
            config.state,
            &config.tape,
            Some(&sig),
        ) {
            return Some(Got(rule.clone()));
        }

        let config_limit =
            self.config_count() > 100_000 || config.tape.blocks() > 350;

        let deltas = match self.configs.entry(sig.clone()) {
            Entry::Vacant(entry) => {
                if config_limit {
                    return Some(ConfigLimit);
                }

                entry.insert(PastConfigs::new(config.state, cycle));
                return None;
            },
            Entry::Occupied(mut entry) => {
                if self.attempts > 400 {
                    return None;
                }

                entry.get_mut().next_deltas(config.state, cycle)?
            },
        };

        let rule = self.prove_rule(deltas, config, &sig, prog)?;

        if rule.is_infinite() {
            return Some(InfiniteRule);
        }

        if config.tape.length_one_spans()
            && !rule.is_mult()
            && rule.has_two_values_same()
        {
            return None;
        }

        self.attempts = 0;

        self.configs.get_mut(&sig)?.delete_configs(config.state);

        self.set_rule(&rule, deltas.0, config, &sig, prog);

        Some(Got(rule))
    }

    fn prove_rule(
        &mut self,
        deltas: (Steps, Steps, Steps),
        config: &Config<DynamicTape>,
        sig: &DynamicSignature,
        prog: &impl RunProver,
    ) -> Option<Rule> {
        self.attempts += 1;

        // Replay from the live tape representation without rebalancing.
        // The initial counts, candidate signature, replay counts, and
        // generated rule must all use the same block coordinates.
        let mut tape = config.tape.clone();
        let mut counts = Vec::with_capacity(3);

        #[expect(clippy::tuple_array_conversions)]
        for delta in [deltas.0, deltas.1, deltas.2] {
            if self.run_simulator(delta, config.state, &mut tape, prog)
                != Some(config.state)
                || !tape.sig_compatible(sig)
            {
                return None;
            }

            counts.push(tape.counts());
        }

        make_rule(
            &config.tape.counts(),
            &counts[0],
            &counts[1],
            &counts[2],
        )
    }
}

// Lightweight adapter used only to preserve Prover::get_rule for ordinary
// tapes. It exposes a precomputed dynamic signature without owning a tape.
struct StaticDynamicSig {
    scan: crate::Color,
    sig: DynamicSignature,
}

impl crate::tape::Scan for StaticDynamicSig {
    fn scan(&self) -> crate::Color {
        self.scan
    }
}

impl DynamicGetSig for StaticDynamicSig {
    fn dynamic_signature(&self) -> DynamicSignature {
        self.sig.clone()
    }
}

/**************************************/

type CycleDiff = i32;

const PAST_CONFIG_LIMIT: usize = 5;

struct PastConfig {
    cycles: Vec<Steps>,
}

impl PastConfig {
    fn new(cycle: Steps) -> Self {
        let mut cycles = Vec::with_capacity(PAST_CONFIG_LIMIT);
        cycles.push(cycle);

        Self { cycles }
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        clippy::many_single_char_names
    )]
    fn next_deltas(
        &mut self,
        cycle: Steps,
    ) -> Option<(Steps, Steps, Steps)> {
        self.cycles.push(cycle);

        if self.cycles.len() < PAST_CONFIG_LIMIT {
            return None;
        }

        let [e, d, c, b, a] = self.cycles[..] else {
            unreachable!();
        };

        self.cycles.remove(0);

        let a = a as CycleDiff;
        let b = b as CycleDiff;
        let c = c as CycleDiff;
        let d = d as CycleDiff;
        let e = e as CycleDiff;

        for i in 1..=4 {
            let p1 = a - (b * i);
            let p2 = b - (c * i);
            let diff = p1 - p2;

            let p3 = c - (d * i);
            if p2 - p3 != diff {
                continue;
            }

            let p4 = d - (e * i);
            if p3 - p4 != diff {
                continue;
            }

            let nxt1 = a * i + p1 + diff;
            let nxt2 = nxt1 * i + p1 + 2 * diff;
            let nxt3 = nxt2 * i + p1 + 3 * diff;

            if a > nxt1 || nxt1 > nxt2 || nxt2 > nxt3 {
                return None;
            }

            return Some((
                (nxt1 - a) as Steps,
                (nxt2 - nxt1) as Steps,
                (nxt3 - nxt2) as Steps,
            ));
        }

        None
    }
}

pub struct PastConfigs {
    configs: Dict<State, PastConfig>,
}

impl PastConfigs {
    pub fn new(state: State, cycle: Steps) -> Self {
        Self {
            configs: Dict::from([(state, PastConfig::new(cycle))]),
        }
    }

    pub fn next_deltas(
        &mut self,
        state: State,
        cycle: Steps,
    ) -> Option<(Steps, Steps, Steps)> {
        self.configs
            .entry(state)
            .or_insert_with(|| PastConfig::new(cycle))
            .next_deltas(cycle)
    }

    pub fn delete_configs(&mut self, state: State) {
        self.configs.remove(&state);
    }
}
