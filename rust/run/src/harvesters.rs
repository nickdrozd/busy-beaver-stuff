use tm::Prog;

use crate::tree::{Harvester, PassConfig, TreeResult};

/**************************************/

pub struct Visited<const s: usize, const c: usize> {
    visited: u64,
}

impl<const s: usize, const c: usize> Visited<s, c> {
    pub const fn new() -> Self {
        Self { visited: 0 }
    }
}

impl<const s: usize, const c: usize> Harvester<s, c> for Visited<s, c> {
    fn harvest(&mut self, _: &Prog<s, c>, _: PassConfig<'_>) {
        self.visited += 1;

        // prog.print();
    }

    type Output = u64;

    fn combine(results: &TreeResult<Self>) -> Self::Output {
        results.values().map(|harv| harv.visited).sum()
    }
}

/**************************************/

pub struct Collector<const s: usize, const c: usize> {
    progs: Vec<String>,
}

impl<const s: usize, const c: usize> Collector<s, c> {
    pub const fn new() -> Self {
        Self { progs: vec![] }
    }
}

impl<const s: usize, const c: usize> Harvester<s, c>
    for Collector<s, c>
{
    fn harvest(&mut self, prog: &Prog<s, c>, _: PassConfig<'_>) {
        self.progs.push(prog.to_string());
    }

    type Output = Vec<String>;

    fn combine(results: &TreeResult<Self>) -> Self::Output {
        results
            .values()
            .flat_map(|harv| harv.progs.clone())
            .collect()
    }
}

/**************************************/

pub type Pipeline<const s: usize, const c: usize> =
    fn(&Prog<s, c>, PassConfig<'_>) -> bool;

pub struct HoldoutVisited<const s: usize, const c: usize> {
    holdout: u64,
    visited: u64,

    pipeline: Pipeline<s, c>,
}

impl<const s: usize, const c: usize> HoldoutVisited<s, c> {
    pub const fn new(pipeline: Pipeline<s, c>) -> Self {
        Self {
            holdout: 0,
            visited: 0,
            pipeline,
        }
    }
}

impl<const s: usize, const c: usize> Harvester<s, c>
    for HoldoutVisited<s, c>
{
    fn harvest(&mut self, prog: &Prog<s, c>, config: PassConfig<'_>) {
        self.visited += 1;

        if (self.pipeline)(prog, config) {
            return;
        }

        self.holdout += 1;

        // prog.print();
    }

    type Output = (u64, u64);

    fn combine(results: &TreeResult<Self>) -> Self::Output {
        results
            .values()
            .map(|harv| (harv.holdout, harv.visited))
            .fold((0, 0), |(acc1, acc2), (v1, v2)| {
                (acc1 + v1, acc2 + v2)
            })
    }
}

/**************************************/

use tm::reason::BackwardResult;

type Reasoner<const s: usize, const c: usize> =
    fn(&Prog<s, c>, usize) -> BackwardResult;

pub struct ReasonHarvester<const s: usize, const c: usize> {
    holdout: u64,
    refuted: usize,

    goal: u8,
    cant_reach: Reasoner<s, c>,
}

impl<const s: usize, const c: usize> ReasonHarvester<s, c> {
    pub const fn new(goal: u8) -> Self {
        let cant_reach = match goal {
            0 => Prog::cant_halt,
            1 => Prog::cant_spinout,
            2 => Prog::cant_blank,
            3 => Prog::cant_twostep,
            4 => Prog::cant_zloop,
            _ => unreachable!(),
        };

        Self {
            holdout: 0,
            refuted: 0,

            goal,
            cant_reach,
        }
    }
}

impl<const s: usize, const c: usize> Harvester<s, c>
    for ReasonHarvester<s, c>
{
    fn harvest(&mut self, prog: &Prog<s, c>, _: PassConfig<'_>) {
        let result = (self.cant_reach)(prog, 256);

        if let BackwardResult::Refuted(steps) = result {
            if self.refuted < steps {
                self.refuted = steps;
            }

            return;
        }

        self.holdout += 1;

        // prog.print();
    }

    type Output = (usize, u64);

    fn combine(results: &TreeResult<Self>) -> Self::Output {
        results
            .values()
            .map(|harv| (harv.refuted, harv.holdout))
            .fold((0, 0), |(r_acc, h_acc), (r_val, h_val)| {
                (r_acc.max(r_val), h_acc + h_val)
            })
    }
}
