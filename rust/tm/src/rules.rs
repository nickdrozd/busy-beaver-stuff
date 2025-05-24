use std::collections::{BTreeMap as Dict, BTreeSet as Set};

use num_bigint::BigInt;
use num_integer::Integer as _;
use num_traits::{
    CheckedMul as _, One as _, Signed as _, ToPrimitive as _, Zero as _,
};

use crate::tape::{BigCount as Count, Index, IndexTape};

/**************************************/

pub type Diff = BigInt;

#[derive(Clone, Debug)]
pub enum Op {
    Plus(Diff),
    Mult((Count, Count)),
}

/**************************************/

#[derive(Clone, Debug)]
pub struct Rule(Dict<Index, Op>);

impl Rule {
    #[expect(clippy::new_without_default)]
    pub const fn new() -> Self {
        Self(Dict::new())
    }

    #[cfg(test)]
    #[expect(clippy::ref_patterns)]
    pub fn from_triples(triples: &[(Index, Op)]) -> Self {
        Self(
            triples
                .iter()
                .map(|&(index, ref op)| (index, op.clone()))
                .collect(),
        )
    }

    pub fn is_infinite(&self) -> bool {
        !self.0.values().any(
            |diff| matches!(diff, Plus(plus) if plus.is_negative()),
        )
    }

    pub fn is_mult(&self) -> bool {
        self.0.values().any(|diff| matches!(diff, Mult(_)))
    }

    pub fn has_two_values_same(&self) -> bool {
        self.0.len() == 2
            && self
                .0
                .values()
                .filter_map(|diff| match diff {
                    Plus(diff) => Some(diff.abs()),
                    Mult(_) => None,
                })
                .collect::<Set<Diff>>()
                .len()
                == 1
    }
}

/**************************************/

enum DiffResult {
    Got(Op),
    Unknown,
}

use DiffResult::*;
use Op::*;

fn calculate_diff(
    a: &Count,
    b: &Count,
    c: &Count,
    d: &Count,
) -> Option<DiffResult> {
    if a == b && b == c && c == d {
        return None;
    }

    let (a, b, c, d): (Diff, Diff, Diff, Diff) = (
        a.clone().into(),
        b.clone().into(),
        c.clone().into(),
        d.clone().into(),
    );

    let Some(diff_1) = b.checked_sub(&a) else {
        return Some(Unknown);
    };

    let Some(diff_2) = c.checked_sub(&b) else {
        return Some(Unknown);
    };

    if diff_1 == diff_2
        && diff_2 == {
            let Some(diff_3) = d.checked_sub(&c) else {
                return Some(Unknown);
            };
            diff_3
        }
    {
        return Some(Got(Plus(diff_1)));
    }

    if a.is_zero() || b.is_zero() {
        return Some(Unknown);
    }

    let divmod1 = b.div_rem(&a);
    let divmod2 = c.div_rem(&b);

    if divmod1 == divmod2 && divmod2 == d.div_rem(&c) {
        let (mul, add) = divmod1;

        assert!(mul.is_positive());
        assert!(!add.is_negative());

        let (mul, add) =
            (mul.to_biguint().unwrap(), add.to_biguint().unwrap());

        return Some(Got(Mult((mul, add))));
    }

    Some(Unknown)
}

type Counts = (Vec<Count>, Vec<Count>);

pub fn make_rule(
    (l1, r1): &Counts,
    (l2, r2): &Counts,
    (l3, r3): &Counts,
    (l4, r4): &Counts,
) -> Option<Rule> {
    let countses: Vec<Vec<_>> = vec![
        l1.iter()
            .zip(l2.iter())
            .zip(l3.iter())
            .zip(l4.iter())
            .map(|(((a, b), c), d)| {
                (a.clone(), b.clone(), c.clone(), d.clone())
            })
            .collect(),
        r1.iter()
            .zip(r2.iter())
            .zip(r3.iter())
            .zip(r4.iter())
            .map(|(((a, b), c), d)| {
                (a.clone(), b.clone(), c.clone(), d.clone())
            })
            .collect(),
    ];

    let mut rule = Rule::new();

    for (s, spans) in countses.iter().enumerate() {
        for (i, (a, b, c, d)) in spans.iter().enumerate() {
            let Some(diff) = calculate_diff(a, b, c, d) else {
                continue;
            };

            let Got(op) = diff else {
                return None;
            };

            rule.0.insert((s == 1, i), op);
        }
    }

    Some(rule)
}

/**************************************/

pub trait ApplyRule: IndexTape {
    fn apply_rule(&mut self, rule: &Rule) -> Option<Count> {
        let (times, min_pos, min_res) = self.count_apps(rule)?;

        for (pos, diff) in &rule.0 {
            let result = match diff {
                Mult((mul, add)) => {
                    apply_mult(self.get_count(pos), &times, mul, add)?
                },
                Plus(plus) => {
                    if *pos == min_pos {
                        assert!(plus.is_negative());
                        min_res.clone()
                    } else {
                        apply_plus(self.get_count(pos), &times, plus)?
                    }
                },
            };

            self.set_count(pos, result);
        }

        Some(times)
    }

    fn count_apps(&self, rule: &Rule) -> Option<(Count, Index, Count)> {
        let mut apps: Option<(Count, Index, Count)> = None;

        for (pos, diff) in &rule.0 {
            let Plus(diff) = diff else {
                continue;
            };

            if !diff.is_negative() {
                continue;
            }

            let count = self.get_count(pos);
            let absdiff = diff.abs().to_biguint()?;

            if absdiff >= *count {
                return None;
            }

            let (div, rem) = count.div_rem(&absdiff);

            let (times, min_res) = if rem.is_zero() {
                (div - Count::one(), absdiff)
            } else {
                assert!(Count::zero() < rem);
                (div, rem)
            };

            if apps.as_ref().is_none_or(|(curr, _, _)| times < *curr) {
                apps = Some((times, *pos, min_res));
            }
        }

        apps
    }
}

impl<T: IndexTape> ApplyRule for T {}

/**************************************/

fn apply_plus(
    count: &Count,
    times: &Count,
    diff: &Diff,
) -> Option<Count> {
    let mult = diff.abs().to_biguint()?.checked_mul(times)?;

    Some(count + mult)
}

fn apply_mult(
    count: &Count,
    times: &Count,
    mul: &Count,
    add: &Count,
) -> Option<Count> {
    let term = mul.pow(times.to_u32()?);

    Some(
        (count * &term)
            + (add
                * (Count::one()
                    + ((&term - mul) / (mul - Count::one())))),
    )
}
