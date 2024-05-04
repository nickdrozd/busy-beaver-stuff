use std::collections::HashMap;

use pyo3::{pyclass, pymethods};

use crate::instrs::State;

type Cycle = u32;

struct PastConfig {
    cycles: Vec<Cycle>,
}

impl PastConfig {
    fn new(cycle: Cycle) -> Self {
        Self {
            cycles: vec![cycle],
        }
    }

    #[allow(clippy::many_single_char_names)]
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
    configs: HashMap<State, PastConfig>,
}

#[pymethods]
impl PastConfigs {
    #[new]
    fn new(state: State, cycle: Cycle) -> Self {
        Self {
            configs: HashMap::from([(state, PastConfig::new(cycle))]),
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
