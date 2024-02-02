use std::collections::HashMap;

use pyo3::prelude::*;

use crate::instrs::State;

type Cycle = u32;

struct PastConfig {
    cycles: Vec<Cycle>,
}

impl PastConfig {
    const fn new() -> Self {
        Self { cycles: Vec::new() }
    }

    #[allow(clippy::many_single_char_names)]
    pub fn next_deltas(&mut self, cycle: Cycle) -> Option<(Cycle, Cycle, Cycle)> {
        let cycles = &mut self.cycles;
        cycles.push(cycle);

        if cycles.len() < 5 {
            return None;
        }

        let (e, d, c, b, a) = (cycles[0], cycles[1], cycles[2], cycles[3], cycles[4]);

        cycles.remove(0);

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

            let nxt1 = cycle * i + p1 + diff;
            let nxt2 = nxt1 * i + p1 + 2 * diff;
            let nxt3 = nxt2 * i + p1 + 3 * diff;

            return Some((nxt1 - cycle, nxt2 - nxt1, nxt3 - nxt2));
        }

        None
    }
}

#[pyclass]
pub struct PastConfigs {
    configs: HashMap<State, PastConfig>,
}

#[pymethods]
impl PastConfigs {
    #[new]
    fn new() -> Self {
        Self {
            configs: HashMap::new(),
        }
    }

    fn next_deltas(&mut self, state: State, cycle: Cycle) -> Option<(Cycle, Cycle, Cycle)> {
        let config = self.configs.entry(state).or_insert_with(PastConfig::new);
        config.next_deltas(cycle)
    }

    fn delete_configs(&mut self, state: State) {
        self.configs.remove(&state);
    }
}
