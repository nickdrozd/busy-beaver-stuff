use std::collections::HashMap as Dict;

use tm::{Instr, Prog};

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
    fn harvest(&mut self, _: &Prog<s, c>, _: &mut PassConfig<'_>) {
        self.visited += 1;

        // prog.print();
    }

    type Output = (u64, Dict<Instr, u64>);

    fn combine(results: &TreeResult<Self>) -> Self::Output {
        let by_instr = results
            .iter()
            .map(|(&instr, harv)| (instr, harv.visited))
            .collect::<Dict<_, _>>();
        let visited = by_instr.values().sum();

        (visited, by_instr)
    }
}

/**************************************/

pub type Pipeline<const s: usize, const c: usize> =
    fn(&Prog<s, c>, &mut PassConfig<'_>) -> bool;

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
    fn harvest(
        &mut self,
        prog: &Prog<s, c>,
        config: &mut PassConfig<'_>,
    ) {
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

pub type Decider<const s: usize, const c: usize> =
    fn(&Prog<s, c>) -> bool;

pub struct MultiCollector<
    const s: usize,
    const c: usize,
    const n: usize,
> {
    progs: [Vec<String>; n],
    visited: u64,

    initial: Pipeline<s, c>,
    bkw: [Decider<s, c>; n],
    rec_prover_short: Pipeline<s, c>,
    cps: [Decider<s, c>; n],
    rec_prover_long: Pipeline<s, c>,
    far: [Decider<s, c>; n],
}

impl<const s: usize, const c: usize, const n: usize>
    MultiCollector<s, c, n>
{
    pub fn new(
        initial: Pipeline<s, c>,
        bkw: [Decider<s, c>; n],
        rec_prover_short: Pipeline<s, c>,
        cps: [Decider<s, c>; n],
        rec_prover_long: Pipeline<s, c>,
        far: [Decider<s, c>; n],
    ) -> Self {
        Self {
            progs: core::array::from_fn(|_| Vec::new()),
            visited: 0,
            initial,
            bkw,
            rec_prover_short,
            cps,
            rec_prover_long,
            far,
        }
    }

    fn run_deciders(
        alive: &mut [bool; n],
        deciders: &[Decider<s, c>; n],
        prog: &Prog<s, c>,
    ) {
        alive.iter_mut().zip(deciders.iter()).for_each(
            |(alive, decider)| {
                if *alive && decider(prog) {
                    *alive = false;
                }
            },
        );
    }
}

impl<const s: usize, const c: usize, const n: usize> Harvester<s, c>
    for MultiCollector<s, c, n>
{
    fn harvest(
        &mut self,
        prog: &Prog<s, c>,
        config: &mut PassConfig<'_>,
    ) {
        self.visited += 1;

        if (self.initial)(prog, config) {
            return;
        }

        let mut alive = [true; n];

        Self::run_deciders(&mut alive, &self.bkw, prog);

        if !alive.iter().any(|&alive| alive) {
            return;
        }

        if (self.rec_prover_short)(prog, config) {
            return;
        }

        Self::run_deciders(&mut alive, &self.cps, prog);

        if !alive.iter().any(|&alive| alive) {
            return;
        }

        if (self.rec_prover_long)(prog, config) {
            return;
        }

        Self::run_deciders(&mut alive, &self.far, prog);

        alive.into_iter().zip(self.progs.iter_mut()).for_each(
            |(alive, progs)| {
                if alive {
                    progs.push(prog.to_string());
                }
            },
        );
    }

    type Output = ([Vec<String>; n], u64);

    fn combine(results: &TreeResult<Self>) -> Self::Output {
        let mut progs: [Vec<String>; n] =
            core::array::from_fn(|_| Vec::new());
        let visited = results.values().map(|harv| harv.visited).sum();

        results.values().for_each(|harv| {
            progs.iter_mut().zip(harv.progs.iter()).for_each(
                |(acc, progs)| acc.extend(progs.iter().cloned()),
            );
        });

        progs.iter_mut().for_each(|progs| progs.sort());

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
    fn harvest(
        &mut self,
        prog: &Prog<s, c>,
        config: &mut PassConfig<'_>,
    ) {
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
