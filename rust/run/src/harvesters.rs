use tm::{Prog, tree::TreeResult};

pub use tm::tree::{Harvester, PassConfig};

/**************************************/

pub struct Visited {
    visited: u64,
}

impl Visited {
    pub const fn new() -> Self {
        Self { visited: 0 }
    }
}

impl Harvester for Visited {
    fn harvest(&mut self, _: &Prog, _: PassConfig<'_>) {
        self.visited += 1;

        // prog.print();
    }

    type Output = u64;

    fn combine(results: &TreeResult<Self>) -> Self::Output {
        results.values().map(|harv| harv.visited).sum()
    }
}

/**************************************/

pub struct Collector {
    progs: Vec<String>,
}

impl Collector {
    pub const fn new() -> Self {
        Self { progs: vec![] }
    }
}

use tm::Parse as _;

impl Harvester for Collector {
    fn harvest(&mut self, prog: &Prog, _: PassConfig<'_>) {
        self.progs.push(prog.show());
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

pub struct HoldoutVisited<P> {
    holdout: u64,
    visited: u64,

    pipeline: P,
}

impl<P: Fn(&Prog, PassConfig<'_>) -> bool> HoldoutVisited<P> {
    pub const fn new(pipeline: P) -> Self {
        Self {
            holdout: 0,
            visited: 0,
            pipeline,
        }
    }
}

impl<P: Send + Fn(&Prog, PassConfig<'_>) -> bool> Harvester
    for HoldoutVisited<P>
{
    fn harvest(&mut self, prog: &Prog, config: PassConfig<'_>) {
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

pub struct ReasonHarvester<R> {
    holdout: u64,
    refuted: usize,

    cant_reach: R,
}

use tm::reason::BackwardResult;

impl<R: Fn(&Prog, usize) -> BackwardResult> ReasonHarvester<R> {
    pub const fn new(cant_reach: R) -> Self {
        Self {
            holdout: 0,
            refuted: 0,

            cant_reach,
        }
    }
}

impl<R: Send + Sync + Fn(&Prog, usize) -> BackwardResult> Harvester
    for ReasonHarvester<R>
{
    fn harvest(&mut self, prog: &Prog, _: PassConfig<'_>) {
        let result = (self.cant_reach)(prog, 256);

        if let BackwardResult::Refuted(steps) = result
            && self.refuted < steps
        {
            self.refuted = steps;
        }

        if result.is_refuted() {
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
