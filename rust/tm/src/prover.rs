use std::collections::BTreeMap;

use ahash::AHashMap as Dict;

use pyo3::{pyclass, pymethods};

use crate::{
    Slot, State, Steps,
    config::{BigConfig, Config},
    machine::RunProver,
    rules::{Rule, make_rule},
    tape::{EnumTape, GetSig, MinSig, Signature},
};

/**************************************/

pub enum ProverResult {
    ConfigLimit,
    InfiniteRule,
    MultRule,
    Got(Rule),
}

use ProverResult::*;

pub struct Prover<'p, Prog: RunProver> {
    prog: &'p Prog,

    rules: BTreeMap<Slot, Vec<(MinSig, Rule)>>,

    configs: Dict<Signature, PastConfigs>,
}

impl<'p, Prog: RunProver> Prover<'p, Prog> {
    pub fn new(prog: &'p Prog) -> Self {
        Self {
            prog,
            rules: BTreeMap::new(),
            configs: Dict::new(),
        }
    }

    pub fn config_count(&self) -> usize {
        self.configs.len()
    }

    fn set_rule(
        &mut self,
        rule: &Rule,
        steps: Steps,
        config: &BigConfig,
        sig: &Signature,
    ) {
        let mut enum_config: Config<EnumTape> = Config {
            state: config.state,
            tape: (&config.tape).into(),
        };

        self.prog.run_rules(steps, &mut enum_config, self);

        let min_sig = enum_config.tape.get_min_sig(sig);

        self.rules
            .entry(config.slot())
            .or_default()
            .push((min_sig, rule.clone()));
    }

    pub fn get_rule<T: GetSig>(
        &self,
        config: &Config<T>,
        sig: Option<&Signature>,
    ) -> Option<&Rule> {
        let rules = self.rules.get(&config.slot())?;

        #[expect(clippy::option_if_let_else)]
        let sig = match sig {
            Some(sig) => sig,
            None => &config.tape.signature(),
        };

        rules
            .iter()
            .find(|(min_sig, _)| sig.matches(min_sig))
            .map(|(_, rule)| rule)
    }

    pub fn try_rule(
        &mut self,
        cycle: usize,
        config: &BigConfig,
    ) -> Option<ProverResult> {
        let sig = config.tape.signature();

        if let Some(known_rule) = self.get_rule(config, Some(&sig)) {
            return Some(Got((*known_rule).clone()));
        }

        if !self.configs.contains_key(&sig) {
            if self.config_count() > 1_000 {
                return Some(ConfigLimit);
            }

            self.configs
                .insert(sig, PastConfigs::new(config.state, cycle));

            return None;
        }

        let deltas = {
            let (d1, d2, d3) = self
                .configs
                .get_mut(&sig)?
                .next_deltas(config.state, cycle)?;

            vec![d1, d2, d3]
        };

        if deltas.iter().any(|&delta| delta > 90_000) {
            return None;
        }

        let mut tags = config.clone();

        let mut counts = vec![];

        for delta in &deltas {
            if self.prog.run_rules(*delta, &mut tags, self)?
                != config.state
                || !tags.tape.sig_compatible(&sig)
            {
                return None;
            }

            counts.push(tags.tape.counts());
        }

        let rule = make_rule(
            &config.tape.counts(),
            &counts[0],
            &counts[1],
            &counts[2],
        )?;

        if rule.is_infinite() {
            return Some(InfiniteRule);
        }

        if rule.is_mult() {
            return Some(MultRule);
        }

        if config.tape.length_one_spans() && rule.has_two_values_same()
        {
            return None;
        }

        self.configs.get_mut(&sig)?.delete_configs(config.state);

        self.set_rule(&rule, deltas[0], config, &sig);

        // println!("--> proved rule: {:?}", rule);

        Some(Got(rule))
    }
}

/**************************************/

type Diff = i32;

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

        if self.cycles.len() < 5 {
            return None;
        }

        let [e, d, c, b, a] = self.cycles[..] else {
            unreachable!();
        };

        self.cycles.remove(0);

        let a = a as Diff;
        let b = b as Diff;
        let c = c as Diff;
        let d = d as Diff;
        let e = e as Diff;

        // println!("{e} {d} {c} {b} {a}");

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

#[pyclass]
pub struct PastConfigs {
    configs: BTreeMap<State, PastConfig>,
}

#[pymethods]
impl PastConfigs {
    #[new]
    fn new(state: State, cycle: Steps) -> Self {
        Self {
            configs: BTreeMap::from([(state, PastConfig::new(cycle))]),
        }
    }

    fn next_deltas(
        &mut self,
        state: State,
        cycle: Steps,
    ) -> Option<(Steps, Steps, Steps)> {
        self.configs
            .entry(state)
            .or_insert_with(|| PastConfig::new(cycle))
            .next_deltas(cycle)
    }

    fn delete_configs(&mut self, state: State) {
        self.configs.remove(&state);
    }
}
