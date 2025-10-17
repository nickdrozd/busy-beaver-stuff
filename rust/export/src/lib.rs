#![expect(clippy::shadow_unrelated, non_camel_case_types)]

use pyo3::{pyclass, pyfunction, pymethods, pymodule};

use tm::{
    Instr, Parse as _, Prog, Slot, State, Steps,
    instrs::show_state,
    reason::{BackwardResult as BackwardResultRs, BackwardResult::*},
    segment::SegmentResult as SegmentResultRs,
};

/***************************************/

#[pyfunction]
pub fn is_connected(prog: &str) -> bool {
    Prog::read(prog).is_connected()
}

#[pyfunction]
pub fn opt_block(prog: &str, steps: Steps) -> usize {
    Prog::read(prog).opt_block(steps)
}

#[pyfunction]
pub fn term_or_rec(prog: &str, sim_lim: Steps) -> bool {
    Prog::read(prog).term_or_rec_fresh(sim_lim).is_settled()
}

#[pyfunction]
pub fn run_transcript(prog: &str, sim_lim: Steps) -> bool {
    Prog::read(prog).run_transcript_fresh(sim_lim).is_settled()
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
    let mut instrs = Instrs::new();

    for (slot, instr) in Prog::read(prog).iter() {
        instrs.insert(slot, *instr);
    }

    instrs
}

#[pyfunction]
pub const fn py_show_state(state: Option<State>) -> char {
    show_state(state)
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
    linrec {},
    spinout {},
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

    const fn __str__(&self) -> &str {
        match self {
            refuted { .. } => "refuted",
            init {} => "init",
            linrec {} => "linrec",
            spinout {} => "spinout",
            step_limit {} => "step_limit",
            depth_limit {} => "depth_limit",
        }
    }
}

impl From<BackwardResultRs> for BackwardResult {
    fn from(result: BackwardResultRs) -> Self {
        match result {
            Refuted(step) => Self::refuted { step },
            Init => Self::init {},
            LinRec => Self::linrec {},
            Spinout => Self::spinout {},
            StepLimit => Self::step_limit {},
            DepthLimit => Self::depth_limit {},
        }
    }
}

#[pyfunction]
pub fn cant_halt(prog: &str, steps: Steps) -> BackwardResult {
    Prog::read(prog).cant_halt(steps).into()
}

#[pyfunction]
pub fn cant_blank(prog: &str, steps: Steps) -> BackwardResult {
    Prog::read(prog).cant_blank(steps).into()
}

#[pyfunction]
pub fn cant_spin_out(prog: &str, steps: Steps) -> BackwardResult {
    Prog::read(prog).cant_spin_out(steps).into()
}

/***************************************/

use tm::segment::Segments;

#[pyclass]
pub enum SegmentResult {
    halt {},
    blank {},
    repeat {},
    spinout {},
    depth_limit {},
    segment_limit {},
    refuted { step: Segments },
}

#[pymethods]
impl SegmentResult {
    const fn is_refuted(&self) -> bool {
        matches!(self, Self::refuted { .. })
    }

    const fn is_settled(&self) -> bool {
        !matches!(self, Self::depth_limit {} | Self::segment_limit {})
    }
}

impl From<SegmentResultRs> for SegmentResult {
    fn from(result: SegmentResultRs) -> Self {
        match result {
            SegmentResultRs::Halt => Self::halt {},
            SegmentResultRs::Blank => Self::blank {},
            SegmentResultRs::Repeat => Self::repeat {},
            SegmentResultRs::Spinout => Self::spinout {},
            SegmentResultRs::DepthLimit => Self::depth_limit {},
            SegmentResultRs::SegmentLimit => Self::segment_limit {},
            SegmentResultRs::Refuted(step) => Self::refuted { step },
        }
    }
}

#[pyfunction]
pub fn segment_cant_halt(prog: &str, segs: Segments) -> SegmentResult {
    Prog::read(prog).seg_cant_halt(segs).into()
}

#[pyfunction]
pub fn segment_cant_blank(prog: &str, segs: Segments) -> SegmentResult {
    Prog::read(prog).seg_cant_blank(segs).into()
}

#[pyfunction]
pub fn segment_cant_spin_out(
    prog: &str,
    segs: Segments,
) -> SegmentResult {
    Prog::read(prog).seg_cant_spin_out(segs).into()
}

/***************************************/

use tm::cps::Radius;

#[pyfunction]
pub fn cps_cant_halt(prog: &str, rad: Radius) -> bool {
    let prog = Prog::read(prog);

    if prog.halt_slots().is_empty() {
        return true;
    }

    prog.cps_cant_halt(rad)
}

#[pyfunction]
pub fn cps_cant_blank(prog: &str, rad: Radius) -> bool {
    let prog = Prog::read(prog);

    if prog.erase_slots().is_empty() {
        return true;
    }

    prog.cps_cant_blank(rad)
}

#[pyfunction]
pub fn cps_cant_spin_out(prog: &str, rad: Radius) -> bool {
    let prog = Prog::read(prog);

    if prog.zr_shifts().is_empty() {
        return true;
    }

    prog.cps_cant_spin_out(rad)
}

/***************************************/

#[pyfunction]
pub fn ctl_cant_halt(prog: &str, steps: Steps) -> bool {
    let prog = Prog::read(prog);

    if prog.halt_slots().is_empty() {
        return true;
    }

    prog.ctl_cant_halt(steps)
}

#[pyfunction]
pub fn ctl_cant_blank(prog: &str, steps: Steps) -> bool {
    let prog = Prog::read(prog);

    if prog.erase_slots().is_empty() {
        return true;
    }

    prog.ctl_cant_blank(steps)
}

#[pyfunction]
pub fn ctl_cant_spin_out(prog: &str, steps: Steps) -> bool {
    let prog = Prog::read(prog);

    if prog.zr_shifts().is_empty() {
        return true;
    }

    prog.ctl_cant_spin_out(steps)
}

/***************************************/

use std::collections::BTreeMap as Dict;

use tm::{config::BigConfig, tape::BigCount};

type BigStep = BigCount;

type Blanks = Dict<State, BigStep>;

#[pyclass(eq, eq_int)]
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
    fn halted(&self) -> Option<BigStep> {
        matches!(self.result, undfnd).then_some(self.steps())
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
    let prog = Prog::read(prog);

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

#[pymodule]
mod rust_stuff {
    #[pymodule_export]
    use {
        crate::{
            BackwardResult, MachineResult, TermRes, cant_blank,
            cant_halt, cant_spin_out, cps_cant_blank, cps_cant_halt,
            cps_cant_spin_out, ctl_cant_blank, ctl_cant_halt,
            ctl_cant_spin_out, is_connected, opt_block, py_show_state,
            read_instr, run_quick_machine, run_transcript,
            segment_cant_blank, segment_cant_halt,
            segment_cant_spin_out, show_comp, show_instr, show_slot,
            tcompile, term_or_rec,
        },
        tm::prover::PastConfigs,
    };
}
