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

pub type Pipeline = fn(&Prog, PassConfig<'_>) -> bool;

pub struct HoldoutVisited {
    holdout: u64,
    visited: u64,

    pipeline: Pipeline,
}

impl HoldoutVisited {
    pub const fn new(pipeline: Pipeline) -> Self {
        Self {
            holdout: 0,
            visited: 0,
            pipeline,
        }
    }
}

impl Harvester for HoldoutVisited {
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

use tm::reason::BackwardResult;

type Reasoner = fn(&Prog, usize) -> BackwardResult;

pub struct ReasonHarvester {
    holdout: u64,
    refuted: usize,

    cant_reach: Reasoner,
}

impl ReasonHarvester {
    pub const fn new(cant_reach: Reasoner) -> Self {
        Self {
            holdout: 0,
            refuted: 0,

            cant_reach,
        }
    }
}

impl Harvester for ReasonHarvester {
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
