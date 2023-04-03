use pyo3::prelude::*;

#[pyclass]
pub struct PastConfig {
    cycles: Vec<u32>,
}

#[pymethods]
impl PastConfig {
    #[new]
    pub fn new() -> Self {
        PastConfig { cycles: Vec::new() }
    }

    pub fn next_deltas(&mut self, cycle: u32) -> PyResult<Option<(u32, u32)>> {
        let cycles = &mut self.cycles;
        cycles.push(cycle);

        if cycles.len() < 5 {
            return Ok(None);
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
                let nxt = a * i + p1 + diff;
                let nxxt = nxt * i + p1 + 2 * diff;

                return Ok(Some((nxt - cycle, nxxt - nxt)));
            }
        }

        Ok(None)
    }
}
