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

pub type Pipeline<const s: usize, const c: usize> =
    fn(&Prog<s, c>, PassConfig<'_>) -> bool;

pub struct Collector<const s: usize, const c: usize> {
    progs: Vec<String>,
    visited: u64,
    pipeline: Pipeline<s, c>,
}

impl<const s: usize, const c: usize> Collector<s, c> {
    pub const fn new(pipeline: Pipeline<s, c>) -> Self {
        Self {
            progs: vec![],
            visited: 0,
            pipeline,
        }
    }
}

impl<const s: usize, const c: usize> Harvester<s, c>
    for Collector<s, c>
{
    fn harvest(&mut self, prog: &Prog<s, c>, config: PassConfig<'_>) {
        self.visited += 1;

        if (self.pipeline)(prog, config) {
            return;
        }

        self.progs.push(prog.to_string());
    }

    type Output = (Vec<String>, u64);

    fn combine(results: &TreeResult<Self>) -> Self::Output {
        let mut progs = results
            .values()
            .flat_map(|harv| harv.progs.clone())
            .collect::<Vec<_>>();
        let visited = results.values().map(|harv| harv.visited).sum();

        progs.sort();

        (progs, visited)
    }
}

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
