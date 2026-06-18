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
    fn harvest(&mut self, _: &Prog<s, c>, _: &mut PassConfig<'_>) {
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

pub struct MultiCollector<
    const s: usize,
    const c: usize,
    const n: usize,
> {
    progs: [Vec<String>; n],
    visited: [u64; n],

    shared: Pipeline<s, c>,
    pipelines: [Pipeline<s, c>; n],
}

impl<const s: usize, const c: usize, const n: usize>
    MultiCollector<s, c, n>
{
    pub fn new(
        shared: Pipeline<s, c>,
        pipelines: [Pipeline<s, c>; n],
    ) -> Self {
        Self {
            progs: core::array::from_fn(|_| Vec::new()),
            visited: [0; n],
            shared,
            pipelines,
        }
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
        let shared = (self.shared)(prog, config);

        self.visited
            .iter_mut()
            .zip(self.pipelines.iter())
            .zip(self.progs.iter_mut())
            .for_each(|((visited, pipeline), progs)| {
                *visited += 1;

                if shared || pipeline(prog, config) {
                    return;
                }

                progs.push(prog.to_string());
            });
    }

    type Output = ([Vec<String>; n], u64);

    fn combine(results: &TreeResult<Self>) -> Self::Output {
        let mut progs: [Vec<String>; n] =
            core::array::from_fn(|_| Vec::new());
        let mut visited = [0; n];

        results.values().for_each(|harv| {
            progs.iter_mut().zip(harv.progs.iter()).for_each(
                |(acc, progs)| acc.extend(progs.iter().cloned()),
            );

            visited
                .iter_mut()
                .zip(harv.visited.iter())
                .for_each(|(acc, v)| *acc += v);
        });

        progs.iter_mut().for_each(|progs| progs.sort());

        let total_visited = visited.first().copied().unwrap_or(0);

        assert!(
            visited.iter().all(|v| *v == total_visited),
            "multi-collector visited counts differed: {visited:?}",
        );

        (progs, total_visited)
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
