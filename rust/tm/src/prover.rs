use std::collections::{BTreeMap as TreeDict, VecDeque};

use ahash::AHashMap as Dict;
use num_bigint::{BigInt, Sign};
use num_integer::Integer as _;
use num_traits::{One as _, Signed as _, ToPrimitive as _, Zero as _};

use crate::{
    Color, Shift, Slot, State, Steps,
    config::BigConfig,
    machine::RunProver,
    rules::{Op, Rule},
    tape::{
        BigCount, Block as _, ColorCount, GetSig, Index, IndexTape,
        MinSig, Signature,
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
    // Unguarded rules retain the original storage and public lookup API.
    rules: Dict<Slot, Vec<(MinSig, Rule)>>,

    // Congruence-specialized rules are private to the prover.  They cannot
    // be returned by get_rule because a generic Config<T> does not expose
    // enough information through the existing GetSig API to check them.
    guarded_rules: Dict<Slot, Vec<StoredRule>>,

    configs: Dict<Signature, PastConfigs>,
}

struct StoredRule {
    min_sig: MinSig,
    domain: CountDomain,
    rule: Rule,
}

impl Prover {
    pub fn new() -> Self {
        Self {
            rules: Dict::new(),
            guarded_rules: Dict::new(),
            configs: Dict::new(),
        }
    }

    pub fn config_count(&self) -> usize {
        self.configs.len()
    }

    fn set_rule(
        &mut self,
        rule: &Rule,
        min_sig: MinSig,
        domain: CountDomain,
        config: &BigConfig,
    ) {
        if domain.is_empty() {
            self.rules
                .entry(config.slot())
                .or_default()
                .push((min_sig, rule.clone()));
        } else {
            self.guarded_rules.entry(config.slot()).or_default().push(
                StoredRule {
                    min_sig,
                    domain,
                    rule: rule.clone(),
                },
            );
        }
    }

    // Keep the original Config/GetSig API unchanged.  Only rules that are
    // valid for every count represented by the signature are exposed here.
    pub fn get_rule<T: GetSig>(
        &self,
        config: &crate::config::Config<T>,
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

    fn get_guarded_rule(
        &self,
        config: &BigConfig,
        sig: &Signature,
    ) -> Option<&Rule> {
        self.guarded_rules
            .get(&config.slot())?
            .iter()
            .find(|entry| {
                sig.matches(&entry.min_sig)
                    && entry.domain.matches(&config.tape)
            })
            .map(|entry| &entry.rule)
    }

    pub fn try_rule(
        &mut self,
        cycle: usize,
        config: &BigConfig,
        prog: &impl RunProver,
    ) -> Option<ProverResult> {
        let sig = config.tape.signature();

        if let Some(known_rule) = self
            .get_rule(config, Some(&sig))
            .or_else(|| self.get_guarded_rule(config, &sig))
        {
            return Some(Got((*known_rule).clone()));
        }

        let Some(past_configs) = self.configs.get_mut(&sig) else {
            if config.tape.blocks() > 350 {
                return Some(ConfigLimit);
            }

            let counts = CountSnapshot::from_config(config, &sig);

            self.configs.insert(
                sig,
                PastConfigs::new_with_counts(
                    config.state,
                    cycle,
                    counts,
                ),
            );

            return None;
        };

        let counts = CountSnapshot::from_config(config, &sig);
        let (delta, delta_2, delta_3, domain) = past_configs
            .next_deltas_with_domain(config.state, cycle, counts)?;

        if [delta, delta_2, delta_3].into_iter().any(|d| d > 90_000) {
            return None;
        }

        let proved =
            self.prove_rule(delta, config, &sig, &domain, prog)?;

        #[expect(clippy::shadow_unrelated)]
        match proved {
            Proved::Infinite => Some(InfiniteRule),
            Proved::Rule(rule, min_sig, domain) => {
                self.configs
                    .get_mut(&sig)?
                    .delete_configs(config.state);

                self.set_rule(&rule, min_sig, domain, config);

                // println!("--> proved rule: {:?}", rule);

                Some(Got(rule))
            },
        }
    }

    fn prove_rule(
        &self,
        delta: Steps,
        config: &BigConfig,
        sig: &Signature,
        domain: &CountDomain,
        prog: &impl RunProver,
    ) -> Option<Proved> {
        if delta == 0 {
            return None;
        }

        let mut context = AlgebraicContext::new();
        let tape = AlgebraicTape::from_concrete(
            &config.tape,
            sig,
            domain,
            &mut context,
        )
        .ok()?;

        let mut symbolic = AlgebraicConfig {
            state: config.state,
            tape,
        };

        self.run_algebraic(delta, &mut symbolic, &mut context, prog)
            .ok()??;

        if symbolic.state != config.state {
            return None;
        }

        let (rule, min_sig) =
            symbolic.tape.extract_rule(sig, &context).ok()?;
        let domain = domain.restrict(&min_sig);

        if rule.is_infinite() {
            domain.is_preserved_by(&rule).then_some(Proved::Infinite)
        } else {
            Some(Proved::Rule(rule, min_sig, domain))
        }
    }

    fn run_algebraic(
        &self,
        steps: Steps,
        config: &mut AlgebraicConfig,
        context: &mut AlgebraicContext,
        prog: &impl RunProver,
    ) -> ProofResult<Option<State>> {
        for _ in 0..steps {
            let slot = config.slot();
            let mut applied = false;

            if let Some(entries) = self.rules.get(&slot) {
                for (min_sig, rule) in entries {
                    if !config.tape.min_sig_matches(context, min_sig)? {
                        continue;
                    }

                    applied = config.tape.apply_rule(context, rule)?;
                    break;
                }
            }

            if !applied
                && let Some(entries) = self.guarded_rules.get(&slot)
            {
                for entry in entries {
                    if !config
                        .tape
                        .min_sig_matches(context, &entry.min_sig)?
                        || !entry
                            .domain
                            .matches_algebraic(&mut config.tape)?
                    {
                        continue;
                    }

                    applied =
                        config.tape.apply_rule(context, &entry.rule)?;
                    break;
                }
            }

            if applied {
                continue;
            }

            let Some((color, shift, next_state)) =
                prog.get_instr(&slot).ok().flatten()
            else {
                return Ok(None);
            };

            config.tape.step(
                context,
                shift,
                color,
                config.state == next_state,
            )?;

            config.state = next_state;
        }

        Ok(Some(config.state))
    }
}

/**************************************/

#[derive(Debug)]
struct ProofFailure;

type ProofResult<T> = Result<T, ProofFailure>;

enum Proved {
    Infinite,
    Rule(Rule, MinSig, CountDomain),
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct AlgebraicValue {
    terms: TreeDict<Index, BigInt>,
    constant: BigInt,
    witness: BigInt,
}

impl AlgebraicValue {
    fn constant(value: impl Into<BigInt>) -> Self {
        let value = value.into();

        Self {
            terms: TreeDict::new(),
            constant: value.clone(),
            witness: value,
        }
    }

    #[cfg(test)]
    fn variable(index: Index, witness: BigInt) -> Self {
        Self {
            terms: TreeDict::from([(index, BigInt::one())]),
            constant: BigInt::zero(),
            witness,
        }
    }

    fn is_constant(&self) -> bool {
        self.terms.is_empty()
    }

    fn is_constant_value(&self, value: i32) -> bool {
        self.is_constant() && self.constant == BigInt::from(value)
    }

    #[cfg(test)]
    fn add_i32(&self, value: i32) -> Self {
        let mut result = self.clone();
        result.add_i32_assign(value);
        result
    }

    fn add_i32_assign(&mut self, value: i32) {
        let value = BigInt::from(value);
        self.constant += &value;
        self.witness += value;
    }

    fn add_assign(&mut self, other: &Self) {
        for (index, coefficient) in &other.terms {
            let remove = {
                let entry = self
                    .terms
                    .entry(*index)
                    .or_insert_with(BigInt::zero);
                *entry += coefficient;
                entry.is_zero()
            };

            if remove {
                self.terms.remove(index);
            }
        }

        self.constant += &other.constant;
        self.witness += &other.witness;
    }

    fn add_scaled_assign(&mut self, other: &Self, multiplier: &BigInt) {
        if multiplier.is_zero() {
            return;
        }

        for (index, coefficient) in &other.terms {
            let scaled = coefficient * multiplier;

            if scaled.is_zero() {
                continue;
            }

            let remove = {
                let entry = self
                    .terms
                    .entry(*index)
                    .or_insert_with(BigInt::zero);
                *entry += scaled;
                entry.is_zero()
            };

            if remove {
                self.terms.remove(index);
            }
        }

        self.constant += &other.constant * multiplier;
        self.witness += &other.witness * multiplier;
    }

    fn add_bigint_assign(&mut self, value: &BigInt) {
        self.constant += value;
        self.witness += value;
    }

    fn scale_assign(&mut self, multiplier: &BigInt) {
        if multiplier.is_one() {
            return;
        }

        if multiplier.is_zero() {
            self.terms.clear();
            self.constant = BigInt::zero();
            self.witness = BigInt::zero();
            return;
        }

        self.terms.retain(|_, coefficient| {
            *coefficient *= multiplier;
            !coefficient.is_zero()
        });
        self.constant *= multiplier;
        self.witness *= multiplier;
    }

    fn subtract_then_divide_exact(
        &self,
        subtrahend: &BigInt,
        divisor: &BigInt,
    ) -> ProofResult<Self> {
        if divisor <= &BigInt::zero() {
            return Err(ProofFailure);
        }

        let mut terms = TreeDict::new();

        for (index, coefficient) in &self.terms {
            let (quotient, remainder) = coefficient.div_rem(divisor);

            if !remainder.is_zero() {
                return Err(ProofFailure);
            }

            if !quotient.is_zero() {
                terms.insert(*index, quotient);
            }
        }

        let (constant, constant_rem) =
            (&self.constant - subtrahend).div_rem(divisor);
        let (witness, witness_rem) =
            (&self.witness - subtrahend).div_rem(divisor);

        if !constant_rem.is_zero() || !witness_rem.is_zero() {
            return Err(ProofFailure);
        }

        Ok(Self {
            terms,
            constant,
            witness,
        })
    }

    fn is_congruent(
        &self,
        modulus: &BigCount,
        residue: &BigCount,
    ) -> bool {
        if modulus == &BigCount::from(2_u8) {
            return self
                .terms
                .values()
                .all(num_integer::Integer::is_even)
                && self.constant.is_even() == residue.is_zero();
        }

        let modulus = BigInt::from_biguint(Sign::Plus, modulus.clone());
        let residue = BigInt::from_biguint(Sign::Plus, residue.clone());

        self.terms.values().all(|coefficient| {
            coefficient.mod_floor(&modulus).is_zero()
        }) && self.constant.mod_floor(&modulus) == residue
    }

    fn lower_bound(
        &self,
        minima: &TreeDict<Index, BigInt>,
    ) -> ProofResult<BigInt> {
        let mut result = self.constant.clone();

        for (index, coefficient) in &self.terms {
            if coefficient.is_negative() {
                return Err(ProofFailure);
            }

            let minimum = minima.get(index).ok_or(ProofFailure)?;
            result += coefficient * minimum;
        }

        Ok(result)
    }
}

struct AlgebraicContext {
    witnesses: TreeDict<Index, BigInt>,
    initial_values: TreeDict<Index, AlgebraicValue>,
    variable_minima: TreeDict<Index, BigInt>,
    // Conjunctions with the same linear coefficients collapse to the
    // strongest right-hand side.  A long symbolic countdown otherwise
    // records one nearly identical allocation per simulated step.
    lower_bounds: TreeDict<TreeDict<Index, BigInt>, BigInt>,
}

impl AlgebraicContext {
    const fn new() -> Self {
        Self {
            witnesses: TreeDict::new(),
            initial_values: TreeDict::new(),
            variable_minima: TreeDict::new(),
            lower_bounds: TreeDict::new(),
        }
    }

    fn variable(
        &mut self,
        index: Index,
        witness: &BigCount,
        domain: &CountDomain,
    ) -> ProofResult<AlgebraicValue> {
        let raw_witness =
            BigInt::from_biguint(Sign::Plus, witness.clone());

        if raw_witness < BigInt::from(2) {
            return Err(ProofFailure);
        }

        // Parameterize the raw count as residue + modulus * x.  This
        // makes division by a learned countdown stride exact when the
        // repeated concrete configurations establish the congruence.
        let (modulus, residue) = domain.constraint(index);
        let modulus = BigInt::from_biguint(Sign::Plus, modulus);
        let residue = BigInt::from_biguint(Sign::Plus, residue);
        let (normalized, remainder) =
            (&raw_witness - &residue).div_rem(&modulus);

        if !remainder.is_zero() {
            return Err(ProofFailure);
        }

        let minimum = (BigInt::from(2) - &residue).div_ceil(&modulus);

        if normalized < minimum {
            return Err(ProofFailure);
        }

        let initial = AlgebraicValue {
            terms: TreeDict::from([(index, modulus)]),
            constant: residue,
            witness: raw_witness,
        };

        self.witnesses.insert(index, normalized);
        self.initial_values.insert(index, initial.clone());
        self.variable_minima.insert(index, minimum);

        Ok(initial)
    }

    fn minimum_for_raw(
        &self,
        index: Index,
        raw_minimum: BigInt,
    ) -> ProofResult<BigInt> {
        let initial =
            self.initial_values.get(&index).ok_or(ProofFailure)?;
        let coefficient =
            initial.terms.get(&index).ok_or(ProofFailure)?;

        Ok((raw_minimum - &initial.constant).div_ceil(coefficient))
    }

    fn require_at_least(
        &mut self,
        value: &AlgebraicValue,
        minimum: impl Into<BigInt>,
    ) {
        // value >= minimum  <=>  terms(value) >= minimum - constant(value).
        // For identical terms only the greatest required RHS matters.
        let required = minimum.into() - &value.constant;

        if let Some(entry) = self.lower_bounds.get_mut(&value.terms) {
            if *entry < required {
                *entry = required;
            }
        } else {
            self.lower_bounds.insert(value.terms.clone(), required);
        }
    }

    fn discharge(
        &self,
        minima: &TreeDict<Index, BigInt>,
    ) -> ProofResult<()> {
        for (terms, required) in &self.lower_bounds {
            let mut lower = BigInt::zero();

            for (index, coefficient) in terms {
                if coefficient.is_negative() {
                    return Err(ProofFailure);
                }

                let minimum = minima.get(index).ok_or(ProofFailure)?;
                lower += coefficient * minimum;
            }

            if &lower < required {
                return Err(ProofFailure);
            }
        }

        Ok(())
    }
}

#[derive(Clone)]
struct AlgebraicBlock {
    color: Color,
    count: AlgebraicValue,
    origin: Option<Index>,
}

struct AlgebraicTape {
    scan: Color,

    // Unlike tape::Span, these deques are stored nearest-block first.
    spans: [VecDeque<AlgebraicBlock>; 2],

    offsets: [usize; 2],
    edges: [bool; 2],
}

impl AlgebraicTape {
    fn from_concrete(
        tape: &crate::tape::BigTape,
        sig: &Signature,
        domain: &CountDomain,
        context: &mut AlgebraicContext,
    ) -> ProofResult<Self> {
        if tape.scan != sig.scan
            || tape.lspan.len() != sig.lspan.len()
            || tape.rspan.len() != sig.rspan.len()
        {
            return Err(ProofFailure);
        }

        fn make_span<'a>(
            side: Shift,
            blocks: impl Iterator<Item = &'a crate::tape::BigBlock>,
            sig: &[ColorCount],
            domain: &CountDomain,
            context: &mut AlgebraicContext,
        ) -> ProofResult<VecDeque<AlgebraicBlock>> {
            blocks
                .zip(sig)
                .enumerate()
                .map(|(pos, (block, color_count))| {
                    let color = match color_count {
                        ColorCount::Just(color)
                        | ColorCount::Mult(color) => *color,
                    };

                    if block.get_color() != color {
                        return Err(ProofFailure);
                    }

                    let index = (side, pos);
                    let count = match color_count {
                        ColorCount::Just(_) => {
                            if !block.get_count().is_one() {
                                return Err(ProofFailure);
                            }

                            AlgebraicValue::constant(1)
                        },
                        ColorCount::Mult(_) => context.variable(
                            index,
                            block.get_count(),
                            domain,
                        )?,
                    };

                    Ok(AlgebraicBlock {
                        color,
                        count,
                        origin: Some(index),
                    })
                })
                .collect()
        }

        Ok(Self {
            scan: tape.scan,
            spans: [
                make_span(
                    false,
                    tape.lspan.iter(),
                    &sig.lspan,
                    domain,
                    context,
                )?,
                make_span(
                    true,
                    tape.rspan.iter(),
                    &sig.rspan,
                    domain,
                    context,
                )?,
            ],
            offsets: [0, 0],
            edges: [false, false],
        })
    }

    const fn side(shift: Shift) -> usize {
        if shift { 1 } else { 0 }
    }

    fn track_origin(&mut self, origin: Option<Index>) {
        let Some((side, pos)) = origin else {
            return;
        };

        let side = Self::side(side);
        self.offsets[side] = self.offsets[side].max(pos + 1);
    }

    fn track_block(
        &mut self,
        side: usize,
        pos: usize,
    ) -> ProofResult<()> {
        let origin =
            self.spans[side].get(pos).ok_or(ProofFailure)?.origin;
        self.track_origin(origin);

        Ok(())
    }

    const fn touch_edge(&mut self, side: usize) {
        self.edges[side] = true;
    }

    fn count_is_one(
        count: &AlgebraicValue,
        context: &mut AlgebraicContext,
    ) -> ProofResult<bool> {
        if !count.witness.is_positive() {
            return Err(ProofFailure);
        }

        if count.witness.is_one() {
            if !count.is_constant_value(1) {
                return Err(ProofFailure);
            }

            Ok(true)
        } else {
            context.require_at_least(count, 2);
            Ok(false)
        }
    }

    fn check_step(
        &mut self,
        shift: Shift,
        color: Color,
        skip: bool,
    ) -> ProofResult<()> {
        let pull = Self::side(shift);
        let push = 1 - pull;

        if self.spans[pull].is_empty() {
            self.touch_edge(pull);
        } else {
            self.track_block(pull, 0)?;

            if skip && self.spans[pull][0].color == self.scan {
                if self.spans[pull].len() == 1 {
                    self.touch_edge(pull);
                } else {
                    self.track_block(pull, 1)?;
                }
            }
        }

        if self.spans[push]
            .front()
            .is_some_and(|block| block.color == color)
        {
            self.track_block(push, 0)?;
        }

        Ok(())
    }

    fn step(
        &mut self,
        context: &mut AlgebraicContext,
        shift: Shift,
        color: Color,
        skip: bool,
    ) -> ProofResult<()> {
        self.check_step(shift, color, skip)?;

        let pull = Self::side(shift);
        let push = 1 - pull;

        let stepped = if skip
            && self.spans[pull]
                .front()
                .is_some_and(|block| block.color == self.scan)
        {
            let mut block =
                self.spans[pull].pop_front().ok_or(ProofFailure)?;
            block.count.add_i32_assign(1);
            block.count
        } else {
            AlgebraicValue::constant(1)
        };

        let next_scan = if self.spans[pull].is_empty() {
            0
        } else {
            let pull_color = self.spans[pull][0].color;

            if Self::count_is_one(&self.spans[pull][0].count, context)?
            {
                self.spans[pull].pop_front().ok_or(ProofFailure)?;
            } else {
                self.spans[pull][0].count.add_i32_assign(-1);
            }

            pull_color
        };

        if let Some(block) = self.spans[push].front_mut()
            && block.color == color
        {
            block.count.add_assign(&stepped);
        } else if !self.spans[push].is_empty() || color != 0 {
            self.spans[push].push_front(AlgebraicBlock {
                color,
                count: stepped,
                origin: None,
            });
        }

        self.scan = next_scan;

        Ok(())
    }

    fn min_sig_matches(
        &mut self,
        context: &mut AlgebraicContext,
        (sig, exact): &MinSig,
    ) -> ProofResult<bool> {
        if self.scan != sig.scan {
            return Ok(false);
        }

        for side in 0..2 {
            let required =
                if side == 0 { &sig.lspan } else { &sig.rspan };

            let is_exact = if side == 0 { exact.0 } else { exact.1 };

            if self.spans[side].len() < required.len() {
                self.touch_edge(side);
                return Ok(false);
            }

            if is_exact && self.spans[side].len() != required.len() {
                if self.spans[side].len() > required.len() {
                    self.track_block(side, required.len())?;
                } else {
                    self.touch_edge(side);
                }

                return Ok(false);
            }

            for (pos, color_count) in required.iter().enumerate() {
                self.track_block(side, pos)?;

                let block = &self.spans[side][pos];
                let (required_color, required_one) = match color_count {
                    ColorCount::Just(color) => (*color, true),
                    ColorCount::Mult(color) => (*color, false),
                };

                if block.color != required_color {
                    return Ok(false);
                }

                if Self::count_is_one(&block.count, context)?
                    != required_one
                {
                    return Ok(false);
                }
            }

            if is_exact {
                self.touch_edge(side);
            }
        }

        Ok(true)
    }

    fn domain_matches(
        &mut self,
        domain: &CountDomain,
    ) -> ProofResult<bool> {
        for (index, constraint) in &domain.0 {
            let count = self.get_count(index)?;

            if !count
                .is_congruent(&constraint.modulus, &constraint.residue)
            {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn get_count(
        &mut self,
        &(side, pos): &Index,
    ) -> ProofResult<&AlgebraicValue> {
        let side = Self::side(side);
        self.track_block(side, pos)?;

        Ok(&self.spans[side][pos].count)
    }

    fn get_count_mut(
        &mut self,
        &(side, pos): &Index,
    ) -> ProofResult<&mut AlgebraicValue> {
        let side = Self::side(side);
        self.track_block(side, pos)?;

        Ok(&mut self.spans[side][pos].count)
    }

    fn rule_times(
        &mut self,
        context: &mut AlgebraicContext,
        rule: &Rule,
    ) -> ProofResult<Option<AlgebraicValue>> {
        let mut candidates = Vec::with_capacity(rule.0.len());

        for (index, op) in &rule.0 {
            let Op::Plus(diff) = op else {
                continue;
            };

            if !diff.is_negative() {
                continue;
            }

            let count = self.get_count(index)?;
            let decrement = diff.abs();

            if count.witness <= decrement {
                if count.is_constant() && count.constant <= decrement {
                    return Ok(None);
                }

                return Err(ProofFailure);
            }

            // count_apps leaves either the positive remainder or one
            // full decrement when the count is exactly divisible.
            let remainder = count.witness.mod_floor(&decrement);
            let final_count = if remainder.is_zero() {
                decrement.clone()
            } else {
                remainder
            };
            let times = count
                .subtract_then_divide_exact(&final_count, &decrement)?;

            candidates.push(times);
        }

        if candidates.is_empty() {
            return Ok(None);
        }

        let chosen_pos = candidates
            .iter()
            .enumerate()
            .min_by(|(_, left), (_, right)| {
                left.witness.cmp(&right.witness)
            })
            .map(|(pos, _)| pos)
            .ok_or(ProofFailure)?;

        let chosen = candidates[chosen_pos].clone();

        context.require_at_least(&chosen, 1);

        let minus_one = BigInt::from(-1);

        for mut candidate in candidates {
            candidate.add_scaled_assign(&chosen, &minus_one);
            context.require_at_least(&candidate, 0);
        }

        Ok(Some(chosen))
    }

    fn apply_rule(
        &mut self,
        context: &mut AlgebraicContext,
        rule: &Rule,
    ) -> ProofResult<bool> {
        let Some(times) = self.rule_times(context, rule)? else {
            return Ok(false);
        };

        // Validate the multiplicative case before mutating any counts.
        let repetitions =
            if rule.0.values().any(|op| matches!(op, Op::Mult(_))) {
                if !times.is_constant() {
                    return Err(ProofFailure);
                }

                times.constant.to_u32().ok_or(ProofFailure)?
            } else {
                0
            };

        for (index, op) in &rule.0 {
            match op {
                Op::Plus(diff) => {
                    self.get_count_mut(index)?
                        .add_scaled_assign(&times, diff);
                },
                Op::Mult((mul, add)) => {
                    if mul.is_zero() || mul.is_one() {
                        return Err(ProofFailure);
                    }

                    let scale = mul.pow(repetitions);
                    let geometric = if add.is_zero() {
                        BigCount::zero()
                    } else {
                        add * ((&scale - BigCount::one())
                            / (mul - BigCount::one()))
                    };
                    let scale = BigInt::from_biguint(Sign::Plus, scale);
                    let geometric =
                        BigInt::from_biguint(Sign::Plus, geometric);
                    let count = self.get_count_mut(index)?;

                    count.scale_assign(&scale);
                    count.add_bigint_assign(&geometric);
                },
            }
        }

        Ok(true)
    }

    fn extract_rule(
        &self,
        sig: &Signature,
        context: &AlgebraicContext,
    ) -> ProofResult<(Rule, MinSig)> {
        #[expect(clippy::suspicious_operation_groupings)]
        if self.scan != sig.scan
            || self.spans[0].len() != sig.lspan.len()
            || self.spans[1].len() != sig.rspan.len()
        {
            return Err(ProofFailure);
        }

        let mut rule = Rule::new();
        let mut final_values = TreeDict::new();

        for side in 0..2 {
            let expected =
                if side == 0 { &sig.lspan } else { &sig.rspan };

            for (pos, color_count) in expected.iter().enumerate() {
                let index = (side == 1, pos);
                let block = &self.spans[side][pos];
                let expected_color = match color_count {
                    ColorCount::Just(color)
                    | ColorCount::Mult(color) => *color,
                };

                if block.color != expected_color {
                    return Err(ProofFailure);
                }

                let value = block.count.clone();
                final_values.insert(index, value.clone());

                match color_count {
                    ColorCount::Just(_) => {
                        if !value.is_constant_value(1) {
                            return Err(ProofFailure);
                        }
                    },
                    ColorCount::Mult(_) => {
                        if value.witness < BigInt::from(2)
                            || value.terms.len() != 1
                        {
                            return Err(ProofFailure);
                        }

                        let initial = context
                            .initial_values
                            .get(&index)
                            .ok_or(ProofFailure)?;
                        let initial_coefficient = initial
                            .terms
                            .get(&index)
                            .ok_or(ProofFailure)?;
                        let final_coefficient = value
                            .terms
                            .get(&index)
                            .ok_or(ProofFailure)?;

                        if initial_coefficient <= &BigInt::zero()
                            || final_coefficient < initial_coefficient
                        {
                            return Err(ProofFailure);
                        }

                        if final_coefficient == initial_coefficient {
                            let diff =
                                &value.constant - &initial.constant;

                            if !diff.is_zero() {
                                rule.0.insert(index, Op::Plus(diff));
                            }
                        } else {
                            let (multiplier, remainder) =
                                final_coefficient
                                    .div_rem(initial_coefficient);

                            if !remainder.is_zero()
                                || multiplier <= BigInt::one()
                            {
                                return Err(ProofFailure);
                            }

                            let addend = &value.constant
                                - (&initial.constant * &multiplier);
                            let Some(multiplier) =
                                multiplier.to_biguint()
                            else {
                                return Err(ProofFailure);
                            };
                            let Some(addend) = addend.to_biguint()
                            else {
                                return Err(ProofFailure);
                            };

                            rule.0.insert(
                                index,
                                Op::Mult((multiplier, addend)),
                            );
                        }
                    },
                }
            }
        }

        let mut minima = context.variable_minima.clone();

        for (index, op) in &rule.0 {
            if let Op::Plus(diff) = op
                && diff.is_negative()
            {
                let needed = context.minimum_for_raw(
                    *index,
                    BigInt::one() + diff.abs(),
                )?;
                let entry = minima
                    .entry(*index)
                    .or_insert_with(|| needed.clone());
                *entry = entry.clone().max(needed);
            }
        }

        context.discharge(&minima)?;

        for (index, value) in final_values {
            if !context.witnesses.contains_key(&index) {
                continue;
            }

            let minimum = match rule.0.get(&index) {
                Some(Op::Plus(diff)) if diff.is_negative() => {
                    BigInt::one()
                },
                _ => BigInt::from(2),
            };

            if value.lower_bound(&minima)? < minimum {
                return Err(ProofFailure);
            }
        }

        let min_sig = (
            Signature {
                scan: sig.scan,
                lspan: sig.lspan[..self.offsets[0]].to_vec(),
                rspan: sig.rspan[..self.offsets[1]].to_vec(),
            },
            self.edges.into(),
        );

        Ok((rule, min_sig))
    }
}

struct AlgebraicConfig {
    state: State,
    tape: AlgebraicTape,
}

impl AlgebraicConfig {
    const fn slot(&self) -> Slot {
        (self.state, self.tape.scan)
    }
}

/**************************************/

#[cfg(test)]
mod algebraic_tests {
    use super::*;

    #[test]
    fn rejects_witness_specific_single_branch() {
        let mut context = AlgebraicContext::new();
        let value =
            AlgebraicValue::variable((false, 0), BigInt::from(2))
                .add_i32(-1);

        assert!(
            AlgebraicTape::count_is_one(&value, &mut context).is_err()
        );
    }

    #[test]
    fn extracts_additive_countdown_with_exact_minimum() {
        let tape = crate::tape::BigTape::from("1^5 [0]");
        let sig = tape.signature();
        let mut context = AlgebraicContext::new();
        let mut symbolic = AlgebraicTape::from_concrete(
            &tape,
            &sig,
            &CountDomain::default(),
            &mut context,
        )
        .unwrap();

        let initial = symbolic.spans[0][0].count.clone();

        // This is the branch encountered while executing x -> x - 2.
        context.require_at_least(&initial.add_i32(-1), 2);
        symbolic.spans[0][0].count = initial.add_i32(-2);

        let (rule, _) = symbolic.extract_rule(&sig, &context).unwrap();

        assert!(matches!(
            rule.0.get(&(false, 0)),
            Some(Op::Plus(diff)) if diff == &BigInt::from(-2)
        ));
    }

    #[test]
    fn rejects_cross_variable_output() {
        let tape = crate::tape::BigTape::from("1^3 [0] 2^4");
        let sig = tape.signature();
        let mut context = AlgebraicContext::new();
        let mut symbolic = AlgebraicTape::from_concrete(
            &tape,
            &sig,
            &CountDomain::default(),
            &mut context,
        )
        .unwrap();

        let right = symbolic.spans[1][0].count.clone();
        symbolic.spans[0][0].count.add_assign(&right);

        assert!(symbolic.extract_rule(&sig, &context).is_err());
    }

    #[test]
    fn coalesces_equivalent_lower_bounds() {
        let mut context = AlgebraicContext::new();
        let value =
            AlgebraicValue::variable((false, 0), BigInt::from(7));

        context.require_at_least(&value.add_i32(-1), 2);
        context.require_at_least(&value.add_i32(-3), 2);

        assert_eq!(context.lower_bounds.len(), 1);
        assert!(
            context
                .discharge(&TreeDict::from([(
                    (false, 0),
                    BigInt::from(5)
                )]))
                .is_ok()
        );
        assert!(
            context
                .discharge(&TreeDict::from([(
                    (false, 0),
                    BigInt::from(4)
                )]))
                .is_err()
        );
    }

    #[test]
    fn divides_symbolic_even_countdown_exactly() {
        let tape = crate::tape::BigTape::from("1^10 [0]");
        let sig = tape.signature();
        let domain = CountDomain(TreeDict::from([(
            (false, 0),
            Congruence {
                modulus: BigCount::from(2_u8),
                residue: BigCount::zero(),
            },
        )]));
        let mut context = AlgebraicContext::new();
        let mut symbolic = AlgebraicTape::from_concrete(
            &tape,
            &sig,
            &domain,
            &mut context,
        )
        .unwrap();
        let rule = Rule(TreeDict::from([(
            (false, 0),
            Op::Plus(BigInt::from(-2)),
        )]));

        assert!(symbolic.apply_rule(&mut context, &rule).unwrap());
        assert!(symbolic.spans[0][0].count.is_constant_value(2));
    }
}

/**************************************/

type CycleDiff = i32;

const PAST_CONFIG_LIMIT: usize = 5;

// Snapshot order is fixed by the signature, so a Vec avoids building a
// BTreeMap on every concrete cycle.  The Index is retained to make the
// invariant explicit and to reject mismatched histories defensively.
#[derive(Clone)]
struct CountSnapshot(Vec<(Index, BigCount)>);

impl CountSnapshot {
    fn from_config(config: &BigConfig, sig: &Signature) -> Self {
        let mut counts = Vec::with_capacity(config.tape.blocks());

        for (side, (span, signature)) in [
            (&config.tape.lspan, &sig.lspan),
            (&config.tape.rspan, &sig.rspan),
        ]
        .into_iter()
        .enumerate()
        {
            for (pos, (block, color_count)) in
                span.iter().zip(signature).enumerate()
            {
                if matches!(color_count, ColorCount::Mult(_)) {
                    counts.push((
                        (side == 1, pos),
                        block.get_count().clone(),
                    ));
                }
            }
        }

        Self(counts)
    }
}

#[derive(Clone, Debug)]
struct Congruence {
    modulus: BigCount,
    residue: BigCount,
}

impl Congruence {
    fn matches(&self, count: &BigCount) -> bool {
        if self.modulus == BigCount::from(2_u8) {
            return count.is_even() == self.residue.is_zero();
        }

        let remainder = count % &self.modulus;
        remainder.eq(&self.residue)
    }
}

// Congruence guards are inferred from repeated concrete configurations.
// They are retained on stored rules, so a parity/stride-specialized proof
// can never be reused on a tape outside the domain that justified it.
#[derive(Clone, Debug, Default)]
struct CountDomain(TreeDict<Index, Congruence>);

impl CountDomain {
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn from_snapshots(snapshots: &[CountSnapshot]) -> Self {
        let Some(first) = snapshots.first() else {
            return Self::default();
        };

        let mut constraints = TreeDict::new();

        for (offset, (index, first_count)) in first.0.iter().enumerate()
        {
            let mut modulus = BigCount::zero();
            let mut previous = first_count;
            let mut complete = true;

            for snapshot in &snapshots[1..] {
                let Some((current_index, current)) =
                    snapshot.0.get(offset)
                else {
                    complete = false;
                    break;
                };

                if current_index != index {
                    complete = false;
                    break;
                }

                let difference = if current >= previous {
                    current - previous
                } else {
                    previous - current
                };

                modulus = modulus.gcd(&difference);
                previous = current;
            }

            if complete && modulus > BigCount::one() {
                constraints.insert(
                    *index,
                    Congruence {
                        residue: first_count % &modulus,
                        modulus,
                    },
                );
            }
        }

        Self(constraints)
    }

    fn constraint(&self, index: Index) -> (BigCount, BigCount) {
        self.0.get(&index).map_or_else(
            || (BigCount::one(), BigCount::zero()),
            |constraint| {
                (constraint.modulus.clone(), constraint.residue.clone())
            },
        )
    }

    fn matches<T: IndexTape<BigCount>>(&self, tape: &T) -> bool {
        self.0.iter().all(|(index, constraint)| {
            constraint.matches(tape.get_count(index))
        })
    }

    fn matches_algebraic(
        &self,
        tape: &mut AlgebraicTape,
    ) -> ProofResult<bool> {
        tape.domain_matches(self)
    }

    fn restrict(&self, (sig, _): &MinSig) -> Self {
        Self(
            self.0
                .iter()
                .filter_map(|(index, constraint)| {
                    let (side, pos) = *index;
                    let span_len = if side {
                        sig.rspan.len()
                    } else {
                        sig.lspan.len()
                    };

                    (pos < span_len)
                        .then(|| (*index, constraint.clone()))
                })
                .collect(),
        )
    }

    fn is_preserved_by(&self, rule: &Rule) -> bool {
        self.0.iter().all(|(index, constraint)| {
            let modulus = BigInt::from_biguint(
                Sign::Plus,
                constraint.modulus.clone(),
            );
            let residue = BigInt::from_biguint(
                Sign::Plus,
                constraint.residue.clone(),
            );

            let output_residue = match rule.0.get(index) {
                None => residue.clone(),
                Some(Op::Plus(diff)) => &residue + diff,
                Some(Op::Mult((multiplier, addend))) => {
                    let multiplier = BigInt::from_biguint(
                        Sign::Plus,
                        multiplier.clone(),
                    );
                    let addend = BigInt::from_biguint(
                        Sign::Plus,
                        addend.clone(),
                    );
                    (&residue * multiplier) + addend
                },
            };

            output_residue.mod_floor(&modulus) == residue
        })
    }
}

struct PastConfig {
    cycles: Vec<Steps>,

    // Kept beside the cycle window so both histories advance together.
    // Empty for callers using the original next_deltas API.
    count_history: Vec<CountSnapshot>,
}

impl PastConfig {
    fn new(cycle: Steps) -> Self {
        let mut cycles = Vec::with_capacity(PAST_CONFIG_LIMIT);

        cycles.push(cycle);

        Self {
            cycles,
            count_history: vec![],
        }
    }

    fn new_with_counts(cycle: Steps, counts: CountSnapshot) -> Self {
        let mut result = Self::new(cycle);
        result.count_history.push(counts);
        result
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

        let a = a as CycleDiff;
        let b = b as CycleDiff;
        let c = c as CycleDiff;
        let d = d as CycleDiff;
        let e = e as CycleDiff;

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

    fn next_deltas_with_domain(
        &mut self,
        cycle: Steps,
        counts: CountSnapshot,
    ) -> Option<(Steps, Steps, Steps, CountDomain)> {
        self.count_history.push(counts);

        if self.count_history.len() > PAST_CONFIG_LIMIT {
            self.count_history.remove(0);
        }

        // Do not perform BigUint subtraction/GCD work until the cycle
        // sequence has actually produced a candidate delta.
        let (delta, delta_2, delta_3) = self.next_deltas(cycle)?;
        let domain = CountDomain::from_snapshots(&self.count_history);

        Some((delta, delta_2, delta_3, domain))
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

    fn new_with_counts(
        state: State,
        cycle: Steps,
        counts: CountSnapshot,
    ) -> Self {
        Self {
            configs: Dict::from([(
                state,
                PastConfig::new_with_counts(cycle, counts),
            )]),
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

    fn next_deltas_with_domain(
        &mut self,
        state: State,
        cycle: Steps,
        counts: CountSnapshot,
    ) -> Option<(Steps, Steps, Steps, CountDomain)> {
        let Some(past) = self.configs.get_mut(&state) else {
            self.configs.insert(
                state,
                PastConfig::new_with_counts(cycle, counts),
            );
            return None;
        };

        past.next_deltas_with_domain(cycle, counts)
    }

    pub fn delete_configs(&mut self, state: State) {
        self.configs.remove(&state);
    }
}
