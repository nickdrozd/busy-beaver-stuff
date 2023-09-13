use pyo3::prelude::*;

#[pyclass]
pub struct PastConfig {
    cycles: Vec<u32>,
}

#[pymethods]
impl PastConfig {
    #[new]
    #[allow(clippy::new_without_default)]
    const fn new() -> Self {
        Self { cycles: Vec::new() }
    }

    #[allow(clippy::similar_names)]
    #[allow(clippy::many_single_char_names)]
    pub fn next_deltas(&mut self, cycle: u32) -> Option<(u32, u32, u32)> {
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
            let p3 = c - (d * i);
            let p4 = d - (e * i);

            let diff = p1 - p2;
            if diff == p2 - p3 && diff == p3 - p4 {
                let nxt1 = cycle * i + p1 + diff;
                let nxt2 = nxt1 * i + p1 + 2 * diff;
                let nxt3 = nxt2 * i + p1 + 3 * diff;

                return Some((nxt1 - cycle, nxt2 - nxt1, nxt3 - nxt2));
            }
        }

        None
    }
}
