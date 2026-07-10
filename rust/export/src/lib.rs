#![expect(clippy::shadow_unrelated, non_camel_case_types)]

use pyo3::{pyclass, pyfunction, pymethods, pymodule};

use tm::{
    Instr, Prog as ProgGen, Slot, State, Steps,
    backward::{BackwardResult as BackwardResultRs, BackwardResult::*},
    instrs::{self, Parse as _},
};

type Prog = ProgGen<10, 10>;

/***************************************/

#[pyfunction]
pub fn opt_block(prog: &str, steps: Steps) -> usize {
    Prog::from(prog).opt_block(steps)
}

#[pyfunction]
pub fn term_or_rec(prog: &str, sim_lim: Steps) -> bool {
    Prog::from(prog).term_or_rec_fresh(sim_lim).is_settled()
}

#[pyfunction]
pub fn run_transcript(prog: &str, sim_lim: Steps) -> bool {
    Prog::from(prog).run_transcript_fresh(sim_lim).is_settled()
}

#[pyfunction]
pub fn show_slot(slot: Slot) -> String {
    slot.show()
}

type Instrs = Dict<Slot, Instr>;

#[expect(clippy::needless_pass_by_value)]
#[pyfunction]
pub fn show_comp(comp: Instrs) -> String {
    let (states, colors) =
        comp.iter().fold((0, 0), |acc, (&(a, b), &(c, _, d))| {
            (acc.0.max(a).max(d), acc.1.max(b).max(c))
        });

    (0..=states)
        .map(|state| {
            (0..=colors)
                .map(|color| comp.get(&(state, color)).show())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("  ")
}

#[pyfunction]
pub fn tcompile(prog: &str) -> Instrs {
    let p = Prog::from(prog);
    let mut instrs = Instrs::new();

    for (slot, instr) in p.iter() {
        instrs.insert(slot, *instr);
    }

    instrs
}

#[pyfunction]
pub const fn show_state(state: Option<State>) -> char {
    instrs::show_state_maybe(state)
}

#[pyfunction]
pub fn show_instr(instr: Option<Instr>) -> String {
    instr.show()
}

#[pyfunction]
pub fn read_instr(instr: &str) -> Option<Instr> {
    Option::<Instr>::read(instr)
}

/***************************************/

#[pyclass]
pub enum BackwardResult {
    refuted { step: Steps },
    init {},
    step_limit {},
    depth_limit {},
}

use BackwardResult::*;

#[pymethods]
impl BackwardResult {
    const fn is_refuted(&self) -> bool {
        matches!(self, Self::refuted { .. })
    }

    const fn is_settled(&self) -> bool {
        matches!(self, Self::refuted { .. } | init {})
    }

    fn __str__(&self) -> String {
        match self {
            refuted { step } => format!("refuted({step})"),
            init {} => "init".into(),
            step_limit {} => "step_limit".into(),
            depth_limit {} => "depth_limit".into(),
        }
    }
}

impl From<BackwardResultRs> for BackwardResult {
    fn from(result: BackwardResultRs) -> Self {
        match result {
            Refuted(step) => Self::refuted { step },
            Init => Self::init {},
            StepLimit => Self::step_limit {},
            DepthLimit => Self::depth_limit {},
        }
    }
}

#[pyfunction]
pub fn bkw_cant_halt(prog: &str, steps: Steps) -> BackwardResult {
    Prog::from(prog).bkw_cant_halt(steps).into()
}

#[pyfunction]
pub fn bkw_cant_blank(prog: &str, steps: Steps) -> BackwardResult {
    Prog::from(prog).bkw_cant_blank(steps).into()
}

#[pyfunction]
pub fn bkw_cant_zloop(prog: &str, steps: Steps) -> BackwardResult {
    Prog::from(prog).bkw_cant_zloop(steps).into()
}

#[pyfunction]
pub fn bkw_cant_spinout(prog: &str, steps: Steps) -> BackwardResult {
    Prog::from(prog).bkw_cant_spinout(steps).into()
}

#[pyfunction]
pub fn bkw_cant_twostep(prog: &str, steps: Steps) -> BackwardResult {
    Prog::from(prog).bkw_cant_twostep(steps).into()
}

/***************************************/

use tm::cps::Radius;

#[pyfunction]
pub fn cps_cant_halt(prog: &str, rad: Radius) -> bool {
    let p = Prog::from(prog);

    if p.halt_slots().is_empty() {
        return true;
    }

    p.cps_cant_halt(rad)
}

#[pyfunction]
pub fn cps_cant_blank(prog: &str, rad: Radius) -> bool {
    let p = Prog::from(prog);

    if p.erase_slots().is_empty() {
        return true;
    }

    p.cps_cant_blank(rad)
}

#[pyfunction]
pub fn cps_cant_spinout(prog: &str, rad: Radius) -> bool {
    let p = Prog::from(prog);

    if p.zr_shifts().is_empty() {
        return true;
    }

    p.cps_cant_spinout(rad)
}

#[pyfunction]
pub fn cps_cant_quasihalt(prog: &str, rad: Radius) -> bool {
    Prog::from(prog).cps_cant_quasihalt(rad)
}

/***************************************/

#[pyfunction]
pub fn far_cant_halt(prog: &str, steps: Steps) -> bool {
    let p = Prog::from(prog);

    if p.halt_slots().is_empty() {
        return true;
    }

    p.far_cant_halt(steps)
}

#[pyfunction]
pub fn far_cant_blank(prog: &str, steps: Steps) -> bool {
    let p = Prog::from(prog);

    if p.erase_slots().is_empty() {
        return true;
    }

    p.far_cant_blank(steps)
}

#[pyfunction]
pub fn far_cant_spinout(prog: &str, steps: Steps) -> bool {
    let p = Prog::from(prog);

    if p.zr_shifts().is_empty() {
        return true;
    }

    p.far_cant_spinout(steps)
}

/***************************************/

use std::collections::BTreeMap as Dict;

use tm::{config::BigConfig, tape::BigCount};

type BigStep = BigCount;

type Blanks = Dict<State, BigStep>;

#[pyclass(eq, eq_int, from_py_object)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TermRes {
    xlimit,
    infrul,
    spnout,
    undfnd,
}

use TermRes::*;

#[pyclass]
#[derive(Debug, PartialEq, Eq)]
pub struct MachineResult {
    pub result: TermRes,

    pub steps: BigStep,
    pub cycles: BigStep,
    pub marks: BigCount,

    pub blanks: Blanks,

    pub last_slot: Option<Slot>,
}

#[pymethods]
impl MachineResult {
    #[new]
    const fn new(
        result: TermRes,
        steps: BigStep,
        cycles: BigStep,
        marks: BigCount,
        blanks: Blanks,
        last_slot: Option<Slot>,
    ) -> Self {
        Self {
            result,
            steps,
            cycles,
            marks,
            blanks,
            last_slot,
        }
    }

    #[getter]
    fn cycles(&self) -> BigStep {
        self.cycles.clone()
    }

    #[getter]
    fn steps(&self) -> BigStep {
        self.steps.clone()
    }

    #[getter]
    fn marks(&self) -> BigCount {
        self.marks.clone()
    }

    #[getter]
    fn blanks(&self) -> Blanks {
        self.blanks.clone()
    }

    #[getter]
    fn undfnd(&self) -> Option<(BigStep, Slot)> {
        matches!(self.result, undfnd)
            .then_some((self.steps(), self.last_slot?))
    }

    #[getter]
    fn simple_termination(&self) -> Option<BigStep> {
        matches!(self.result, undfnd | spnout).then_some(self.steps())
    }

    #[getter]
    fn infrul(&self) -> Option<BigStep> {
        matches!(self.result, infrul).then_some(self.steps())
    }

    #[getter]
    fn spnout(&self) -> Option<BigStep> {
        matches!(self.result, spnout).then_some(self.steps())
    }

    #[getter]
    fn xlimit(&self) -> Option<BigStep> {
        matches!(self.result, xlimit).then_some(self.steps())
    }

    #[getter]
    fn result(&self) -> TermRes {
        self.result.clone()
    }
}

#[pyfunction]
pub fn run_quick_machine(prog: &str, sim_lim: Steps) -> MachineResult {
    let prog = Prog::from(prog);

    let mut config = BigConfig::init();

    let mut blanks = Blanks::new();

    let mut cycles = 0;

    let mut steps = BigCount::ZERO;

    let mut result: Option<TermRes> = None;
    let mut last_slot: Option<Slot> = None;

    for cycle in 0..sim_lim {
        let slot = config.slot();

        let Some(&(color, shift, next_state)) = prog.get(&slot) else {
            cycles = cycle;
            result = Some(undfnd);
            last_slot = Some(slot);
            break;
        };

        let same = config.state == next_state;

        if same && config.tape.at_edge(shift) {
            cycles = cycle;
            result = Some(spnout);
            break;
        }

        let stepped = config.tape.step(shift, color, same);

        steps += stepped;

        config.state = next_state;

        if color == 0 && config.tape.blank() {
            if blanks.contains_key(&config.state) {
                result = Some(infrul);
                break;
            }

            blanks.insert(config.state, steps.clone());

            if config.state == 0 {
                result = Some(infrul);
                break;
            }
        }
    }

    MachineResult {
        result: result.unwrap_or(xlimit),
        steps,
        cycles: cycles.into(),
        marks: config.tape.marks(),
        last_slot,
        blanks,
    }
}

/**************************************/

use tm::prover::PastConfigs;

#[pyclass(name = "PastConfigs")]
struct PastConfigPy(PastConfigs);

#[pymethods]
impl PastConfigPy {
    #[new]
    fn new(state: State, cycle: Steps) -> Self {
        Self(PastConfigs::new(state, cycle))
    }

    fn next_deltas(
        &mut self,
        state: State,
        cycle: Steps,
    ) -> Option<(Steps, Steps, Steps)> {
        self.0.next_deltas(state, cycle)
    }

    fn delete_configs(&mut self, state: State) {
        self.0.delete_configs(state);
    }
}

/**************************************/

#[pymodule]
mod rust_stuff {
    #[pymodule_export]
    use crate::{
        BackwardResult, MachineResult, PastConfigPy, TermRes,
        bkw_cant_blank, bkw_cant_halt, bkw_cant_spinout,
        bkw_cant_twostep, bkw_cant_zloop, cps_cant_blank,
        cps_cant_halt, cps_cant_quasihalt, cps_cant_spinout,
        far_cant_blank, far_cant_halt, far_cant_spinout, opt_block,
        read_instr, run_quick_machine, run_transcript, show_comp,
        show_instr, show_slot, show_state, tcompile, term_or_rec,
    };
}
