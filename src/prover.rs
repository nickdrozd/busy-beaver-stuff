use std::collections::{BTreeMap, BTreeSet as Set, HashMap};

use pyo3::{pyclass, pymethods};

use crate::{
    instrs::{GetInstr, Slot, State},
    rules::{make_rule, ApplyRule as _, Diff, Op, Rule},
    tape::{BasicTape, EnumTape, GetSig, Signature},
};

type Cycle = i32;

type MinSig = (Signature, (bool, bool));

/**************************************/

pub enum ProverResult {
    ConfigLimit,
    InfiniteRule,
    MultRule,
    Got(Rule),
}

use Op::*;
use ProverResult::*;

pub struct Prover<'p, Prog: GetInstr> {
    prog: &'p Prog,

    rules: BTreeMap<Slot, Vec<(MinSig, Rule)>>,

    configs: HashMap<Signature, PastConfigs>,
}

impl<'p, Prog: GetInstr> Prover<'p, Prog> {
    pub fn new(prog: &'p Prog) -> Self {
        Self {
            prog,
            rules: BTreeMap::new(),
            configs: HashMap::new(),
        }
    }

    pub fn config_count(&self) -> usize {
        self.configs.len()
    }

    fn set_rule(&mut self, rule: Rule, state: State, sig: MinSig) {
        self.rules
            .entry((state, sig.0.scan))
            .or_default()
            .push((sig, rule));
    }

    fn get_rule(
        &self,
        state: State,
        tape: &impl GetSig,
        sig: Option<&Signature>,
    ) -> Option<&Rule> {
        let rules = self.rules.get(&(state, tape.scan()))?;

        #[expect(clippy::option_if_let_else)]
        let sig = match sig {
            Some(sig) => sig,
            None => &tape.signature(),
        };

        for ((Signature { scan, lspan, rspan }, (lex, rex)), rule) in
            rules
        {
            if *scan == sig.scan
                && (if *lex {
                    lspan == &sig.lspan
                } else {
                    sig.lspan.starts_with(lspan)
                })
                && (if *rex {
                    rspan == &sig.rspan
                } else {
                    sig.rspan.starts_with(rspan)
                })
            {
                return Some(rule);
            }
        }

        None
    }

    fn run_simulator(
        &self,
        steps: Cycle,
        mut state: State,
        tape: &mut BasicTape,
    ) -> Option<State> {
        for _ in 0..steps {
            if let Some(rule) = self.get_rule(state, tape, None) {
                if tape.apply_rule(rule).is_some() {
                    continue;
                }
            }

            let (color, shift, next_state) =
                self.prog.get_instr(&(state, tape.scan))?;

            tape.step(shift, color, state == next_state);

            state = next_state;
        }

        Some(state)
    }

    fn get_min_sig(
        &self,
        steps: Cycle,
        mut state: State,
        mut tape: EnumTape,
        sig: &Signature,
    ) -> MinSig {
        for _ in 0..steps {
            if let Some(rule) = self.get_rule(state, &tape, None) {
                if tape.apply_rule(rule).is_some() {
                    continue;
                }
            }

            let (color, shift, next_state) =
                self.prog.get_instr(&(state, tape.scan())).unwrap();

            tape.step(shift, color, state == next_state);

            state = next_state;
        }

        let (lmax, rmax) = tape.offsets();

        (
            Signature {
                scan: sig.scan,
                lspan: sig.lspan[..lmax].to_vec(),
                rspan: sig.rspan[..rmax].to_vec(),
            },
            tape.edges(),
        )
    }

    pub fn try_rule(
        &mut self,
        cycle: u64,
        state: State,
        tape: &BasicTape,
    ) -> Option<ProverResult> {
        let cycle = cycle as Cycle;

        let sig = tape.signature();

        if let Some(known_rule) = self.get_rule(state, tape, Some(&sig))
        {
            return Some(Got((*known_rule).clone()));
        }

        if !self.configs.contains_key(&sig) {
            if self.config_count() > 100_000 {
                return Some(ConfigLimit);
            }

            self.configs.insert(sig, PastConfigs::new(state, cycle));

            return None;
        }

        let deltas = {
            let (d1, d2, d3) = self
                .configs
                .get_mut(&sig)?
                .next_deltas(state, cycle)?;

            vec![d1, d2, d3]
        };

        if deltas.iter().any(|&delta| delta > 90_000) {
            return None;
        }

        let mut tags = tape.clone();

        let mut counts = vec![];

        for delta in &deltas {
            if self.run_simulator(*delta, state, &mut tags)? != state
                || !tags.sig_compatible(&sig)
            {
                return None;
            }

            counts.push(tags.counts());
        }

        let rule = make_rule(
            &tape.counts(),
            &counts[0],
            &counts[1],
            &counts[2],
        )?;

        if !rule
            .values()
            .any(|diff| matches!(diff, Plus(plus) if *plus < 0))
        {
            return Some(InfiniteRule);
        }

        if rule.values().any(|diff| matches!(diff, Mult(_))) {
            return Some(MultRule);
        }

        if rule.len() == 2
            && tape.span_lens() == (1, 1)
            && rule.values().all(|diff| matches!(diff, Plus(_)))
            && rule
                .values()
                .map(|diff| match diff {
                    Plus(diff) => diff.abs(),
                    Mult(_) => 0,
                })
                .collect::<Set<Diff>>()
                .len()
                == 1
        {
            return None;
        }

        self.configs.get_mut(&sig)?.delete_configs(state);

        self.set_rule(
            rule.clone(),
            state,
            self.get_min_sig(deltas[0], state, tape.into(), &sig),
        );

        // println!("--> proved rule: {:?}", rule);

        Some(Got(rule))
    }
}

/**************************************/

const PAST_CONFIG_LIMIT: usize = 5;

struct PastConfig {
    cycles: Vec<Cycle>,
}

impl PastConfig {
    fn new(cycle: Cycle) -> Self {
        let mut cycles = Vec::with_capacity(PAST_CONFIG_LIMIT);

        cycles.push(cycle);

        Self { cycles }
    }

    #[expect(clippy::many_single_char_names)]
    pub fn next_deltas(
        &mut self,
        cycle: Cycle,
    ) -> Option<(Cycle, Cycle, Cycle)> {
        self.cycles.push(cycle);

        if self.cycles.len() < 5 {
            return None;
        }

        let [e, d, c, b, a] = self.cycles[..] else {
            panic!();
        };

        self.cycles.remove(0);

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

            return Some((nxt1 - a, nxt2 - nxt1, nxt3 - nxt2));
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
    fn new(state: State, cycle: Cycle) -> Self {
        Self {
            configs: BTreeMap::from([(state, PastConfig::new(cycle))]),
        }
    }

    fn next_deltas(
        &mut self,
        state: State,
        cycle: Cycle,
    ) -> Option<(Cycle, Cycle, Cycle)> {
        self.configs
            .entry(state)
            .or_insert_with(|| PastConfig::new(cycle))
            .next_deltas(cycle)
    }

    fn delete_configs(&mut self, state: State) {
        self.configs.remove(&state);
    }
}
