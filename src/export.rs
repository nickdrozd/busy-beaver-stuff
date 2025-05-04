use pyo3::{pyclass, pyfunction, pymethods};

use crate::{
    blocks::opt_block,
    cps::Cps as _,
    ctl::Ctl as _,
    graph::is_connected,
    instrs::{CompProg, Params, Parse as _, Slot, State},
    machine::quick_term_or_rec,
    reason::{
        Backward as _, BackwardResult as BackwardResultRs,
        BackwardResult::*, Depth, Step,
    },
    segment::{Segment as _, SegmentResult as SegmentResultRs},
    tree::{access, build_tree, get_val, set_val, Step as TreeStep},
};

/***************************************/

#[pyfunction]
pub fn py_is_connected(prog: &str, states: State) -> bool {
    is_connected(&CompProg::from_str(prog), states)
}

#[pyfunction]
pub fn py_opt_block(prog: &str, steps: usize) -> usize {
    opt_block(&CompProg::from_str(prog), steps)
}

#[pyfunction]
pub fn py_quick_term_or_rec(prog: &str, sim_lim: usize) -> bool {
    quick_term_or_rec(&CompProg::from_str(prog), sim_lim).is_recur()
}

#[pyfunction]
#[pyo3(signature = (comp, params=None))]
#[expect(clippy::needless_pass_by_value)]
pub fn py_show_comp(comp: CompProg, params: Option<Params>) -> String {
    show_comp(&comp, params)
}

#[pyfunction]
pub fn tcompile(prog: &str) -> CompProg {
    CompProg::from_str(prog)
}

pub fn show_comp(comp: &CompProg, params: Option<Params>) -> String {
    comp.show(params)
}

/***************************************/

#[expect(non_camel_case_types)]
#[pyclass]
pub enum BackwardResult {
    refuted { step: Step },
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
pub fn py_cant_halt(prog: &str, depth: Depth) -> BackwardResult {
    CompProg::from_str(prog).cant_halt(depth).into()
}

#[pyfunction]
pub fn py_cant_blank(prog: &str, depth: Depth) -> BackwardResult {
    CompProg::from_str(prog).cant_blank(depth).into()
}

#[pyfunction]
pub fn py_cant_spin_out(prog: &str, depth: Depth) -> BackwardResult {
    CompProg::from_str(prog).cant_spin_out(depth).into()
}

/***************************************/

#[expect(non_camel_case_types)]
#[pyclass]
pub enum SegmentResult {
    halt {},
    blank {},
    repeat {},
    spinout {},
    depth_limit {},
    segment_limit {},
    refuted { step: Step },
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

fn get_comp(prog: &str) -> (CompProg, Params) {
    let prog = CompProg::from_str(prog);

    let (states, colors) = prog
        .keys()
        .fold((0, 0), |acc, &(a, b)| (acc.0.max(a), acc.1.max(b)));

    let params = (1 + states, 1 + colors);

    (prog, params)
}

#[pyfunction]
pub fn py_segment_cant_halt(prog: &str, segs: usize) -> SegmentResult {
    let (comp, params) = get_comp(prog);

    comp.seg_cant_halt(params, segs).into()
}

#[pyfunction]
pub fn py_segment_cant_blank(prog: &str, segs: usize) -> SegmentResult {
    let (comp, params) = get_comp(prog);

    comp.seg_cant_blank(params, segs).into()
}

#[pyfunction]
pub fn py_segment_cant_spin_out(
    prog: &str,
    segs: usize,
) -> SegmentResult {
    let (comp, params) = get_comp(prog);

    comp.seg_cant_spin_out(params, segs).into()
}

/***************************************/

#[pyfunction]
pub fn py_cps_cant_halt(prog: &str, segs: usize) -> bool {
    CompProg::from_str(prog).cps_cant_halt(segs)
}

#[pyfunction]
pub fn py_cps_cant_blank(prog: &str, segs: usize) -> bool {
    CompProg::from_str(prog).cps_cant_blank(segs)
}

#[pyfunction]
pub fn py_cps_cant_spin_out(prog: &str, segs: usize) -> bool {
    CompProg::from_str(prog).cps_cant_spin_out(segs)
}

/***************************************/

#[pyfunction]
pub fn py_ctl_cant_halt(prog: &str, steps: usize) -> bool {
    CompProg::from_str(prog).ctl_cant_halt(steps)
}

#[pyfunction]
pub fn py_ctl_cant_blank(prog: &str, steps: usize) -> bool {
    CompProg::from_str(prog).ctl_cant_blank(steps)
}

#[pyfunction]
pub fn py_ctl_cant_spin_out(prog: &str, steps: usize) -> bool {
    CompProg::from_str(prog).ctl_cant_spin_out(steps)
}

/***************************************/

#[pyfunction]
pub fn tree_progs(
    params: Params,
    halt: bool,
    sim_lim: TreeStep,
) -> Vec<String> {
    let progs = set_val(vec![]);

    build_tree(params, halt, sim_lim, &|comp| {
        access(&progs).push(comp.show(Some(params)));
    });

    get_val(progs)
}

/**************************************/

use std::collections::BTreeMap as Dict;

use crate::tape::{BasicTape as Tape, BigCount, MachineTape as _};

type BigStep = u64;

type Blanks = Dict<State, BigStep>;

#[expect(non_camel_case_types)]
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
    #[pyo3(signature = (result, steps, cycles, marks, blanks, last_slot))]
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
    const fn cycles(&self) -> BigStep {
        self.cycles
    }

    #[getter]
    const fn steps(&self) -> BigStep {
        self.steps
    }

    #[getter]
    const fn marks(&self) -> BigCount {
        self.marks
    }

    #[getter]
    fn blanks(&self) -> Blanks {
        self.blanks.clone()
    }

    #[getter]
    fn undfnd(&self) -> Option<(BigStep, Slot)> {
        matches!(self.result, undfnd)
            .then_some((self.steps, self.last_slot?))
    }

    #[getter]
    fn simple_termination(&self) -> Option<BigStep> {
        matches!(self.result, undfnd | spnout).then_some(self.steps)
    }

    #[getter]
    fn halted(&self) -> Option<BigStep> {
        matches!(self.result, undfnd).then_some(self.steps)
    }

    #[getter]
    fn infrul(&self) -> Option<BigStep> {
        matches!(self.result, infrul).then_some(self.steps)
    }

    #[getter]
    fn spnout(&self) -> Option<BigStep> {
        matches!(self.result, spnout).then_some(self.steps)
    }

    #[getter]
    fn xlimit(&self) -> Option<BigStep> {
        matches!(self.result, xlimit).then_some(self.steps)
    }

    #[getter]
    fn result(&self) -> TermRes {
        self.result.clone()
    }
}

#[pyfunction]
#[pyo3(signature = (prog, sim_lim=100_000_000))]
pub fn run_quick_machine(
    prog: &str,
    sim_lim: BigStep,
) -> MachineResult {
    let comp = CompProg::from_str(prog);

    let mut tape = Tape::init(0);

    let mut blanks = Blanks::new();

    let mut state = 0;
    let mut steps = 0;
    let mut cycles = 0;

    let mut result: Option<TermRes> = None;
    let mut last_slot: Option<Slot> = None;

    for cycle in 0..sim_lim {
        let slot = (state, tape.scan);

        let Some(&(color, shift, next_state)) = comp.get(&slot) else {
            cycles = cycle;
            result = Some(undfnd);
            last_slot = Some(slot);
            break;
        };

        let same = state == next_state;

        if same && tape.at_edge(shift) {
            cycles = cycle;
            result = Some(spnout);
            break;
        }

        let stepped = tape.step(shift, color, same);

        steps += stepped;

        state = next_state;

        if color == 0 && tape.blank() {
            if blanks.contains_key(&state) {
                result = Some(infrul);
                break;
            }

            blanks.insert(state, steps);

            if state == 0 {
                result = Some(infrul);
                break;
            }
        }
    }

    MachineResult {
        result: result.unwrap_or(xlimit),
        steps,
        cycles,
        marks: tape.marks(),
        last_slot,
        blanks,
    }
}
