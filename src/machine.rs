use std::collections::BTreeMap as Dict;

use pyo3::{pyclass, pyfunction, pymethods};

use crate::{
    instrs::{CompProg, Parse as _, Slot, State},
    prover::{Prover, ProverResult},
    rules::ApplyRule as _,
    tape::{Alignment as _, BasicTape as Tape, Count, HeadTape},
};

type Step = u64;

type Blanks = Dict<State, Step>;

/**************************************/

#[expect(non_camel_case_types)]
#[pyclass(eq, eq_int)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TermRes {
    xlimit,
    cfglim,
    infrul,
    spnout,
    undfnd,
    mulrul,
}

use ProverResult::*;
use TermRes::*;

#[expect(clippy::fallible_impl_from)]
impl From<ProverResult> for TermRes {
    fn from(prover_result: ProverResult) -> Self {
        match prover_result {
            ConfigLimit => cfglim,
            InfiniteRule => infrul,
            MultRule => mulrul,
            Got(_) => panic!(),
        }
    }
}

/**************************************/

#[pyclass]
#[derive(Debug, PartialEq, Eq)]
pub struct MachineResult {
    pub result: TermRes,

    pub steps: Step,
    pub cycles: Step,
    pub marks: Count,
    pub rulapp: Count,

    pub blanks: Blanks,

    pub last_slot: Option<Slot>,
}

#[pymethods]
impl MachineResult {
    #[new]
    #[pyo3(signature = (result, steps, cycles, marks, rulapp, blanks, last_slot))]
    const fn new(
        result: TermRes,
        steps: Step,
        cycles: Step,
        marks: Count,
        rulapp: Count,
        blanks: Blanks,
        last_slot: Option<Slot>,
    ) -> Self {
        Self {
            result,
            steps,
            cycles,
            marks,
            rulapp,
            blanks,
            last_slot,
        }
    }

    #[getter]
    const fn cycles(&self) -> Step {
        self.cycles
    }

    #[getter]
    const fn steps(&self) -> Step {
        self.steps
    }

    #[getter]
    const fn marks(&self) -> Count {
        self.marks
    }

    #[getter]
    const fn rulapp(&self) -> Count {
        self.rulapp
    }

    #[getter]
    fn blanks(&self) -> Blanks {
        self.blanks.clone()
    }

    #[getter]
    fn undfnd(&self) -> Option<(Step, Slot)> {
        matches!(self.result, undfnd)
            .then_some((self.steps, self.last_slot?))
    }

    #[getter]
    fn simple_termination(&self) -> Option<Step> {
        matches!(self.result, undfnd | spnout).then_some(self.steps)
    }

    #[getter]
    fn halted(&self) -> Option<Step> {
        matches!(self.result, undfnd).then_some(self.steps)
    }

    #[getter]
    fn infrul(&self) -> Option<Step> {
        matches!(self.result, infrul).then_some(self.steps)
    }

    #[getter]
    fn spnout(&self) -> Option<Step> {
        matches!(self.result, spnout).then_some(self.steps)
    }

    #[getter]
    fn xlimit(&self) -> Option<Step> {
        matches!(self.result, xlimit).then_some(self.steps)
    }

    #[getter]
    fn cfglim(&self) -> Option<Step> {
        matches!(self.result, cfglim).then_some(self.steps)
    }

    #[getter]
    fn result(&self) -> TermRes {
        self.result.clone()
    }
}

/**************************************/

#[cfg(test)]
use crate::instrs::GetInstr;

#[cfg(test)]
pub fn run_for_infrul(comp: &impl GetInstr, sim_lim: Step) -> bool {
    let mut tape = Tape::init(0);

    let mut prover = Prover::new(comp);

    let mut state = 0;

    for cycle in 0..sim_lim {
        if let Some(res) = prover.try_rule(cycle, state, &tape) {
            match res {
                ConfigLimit | MultRule => {
                    return false;
                },
                InfiniteRule => {
                    return true;
                },
                Got(rule) => {
                    if tape.apply_rule(&rule).is_some() {
                        // println!("--> applying rule: {:?}", rule);
                        continue;
                    }
                },
            }
        }

        let slot = (state, tape.scan);

        let Some((color, shift, next_state)) = comp.get_instr(&slot)
        else {
            return false;
        };

        let same = state == next_state;

        if same && tape.at_edge(shift) {
            return false;
        }

        tape.step(shift, color, same);

        state = next_state;
    }

    false
}

/**************************************/

#[pyfunction]
#[pyo3(signature = (prog, sim_lim=100_000_000))]
pub fn run_prover(prog: &str, sim_lim: Step) -> MachineResult {
    let comp = CompProg::from_str(prog);

    let mut tape = Tape::init(0);

    let mut prover = Prover::new(&comp);

    let mut blanks = Blanks::new();

    let mut state = 0;
    let mut steps = 0;
    let mut cycles = 0;
    let mut rulapp = 0;

    let mut result: Option<TermRes> = None;
    let mut last_slot: Option<Slot> = None;

    for cycle in 0..sim_lim {
        match prover.try_rule(cycle, state, &tape) {
            Some(Got(rule)) => {
                if let Some(times) = tape.apply_rule(&rule) {
                    // println!("--> applying rule: {:?}", rule);
                    rulapp += times;
                    continue;
                }
            },
            Some(res) => {
                cycles = cycle;
                result = Some(res.into());
                break;
            },
            None => {},
        }

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
        rulapp,
        last_slot,
        blanks,
    }
}

/**************************************/

#[pyfunction]
#[pyo3(signature = (prog, sim_lim=100_000_000))]
pub fn run_quick_machine(prog: &str, sim_lim: Step) -> MachineResult {
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
        rulapp: 0,
        last_slot,
        blanks,
    }
}

/**************************************/

pub enum RecRes {
    Limit,
    Recur,
    Spinout,
    #[expect(dead_code)]
    Undefined(Slot),
}

impl RecRes {
    #[cfg(test)]
    pub const fn is_settled(&self) -> bool {
        !matches!(self, Self::Limit)
    }

    pub const fn is_recur(&self) -> bool {
        matches!(self, Self::Recur | Self::Spinout)
    }
}

pub fn quick_term_or_rec(comp: &CompProg, sim_lim: usize) -> RecRes {
    let mut state = 1;

    let mut tape = HeadTape::init_stepped();

    let head = tape.head();

    let (mut ref_state, mut ref_tape, mut leftmost, mut rightmost) =
        (state, tape.clone(), head, head);

    let mut reset = 1;

    for cycle in 1..sim_lim {
        let slot = (state, tape.scan());

        let Some(&(color, shift, next_state)) = comp.get(&slot) else {
            return RecRes::Undefined(slot);
        };

        let curr_state = state;

        state = next_state;

        let same = curr_state == next_state;

        if same && tape.at_edge(shift) {
            return RecRes::Spinout;
        }

        if reset == 0 {
            ref_state = curr_state;
            ref_tape = tape.clone();
            let head = ref_tape.head();
            leftmost = head;
            rightmost = head;
            reset = cycle;
        }

        reset -= 1;

        tape.step(shift, color, same);

        let curr = tape.head();

        if curr < leftmost {
            leftmost = curr;
        } else if rightmost < curr {
            rightmost = curr;
        }

        if state == ref_state
            && tape.aligns_with(&ref_tape, leftmost, rightmost)
        {
            return RecRes::Recur;
        }
    }

    RecRes::Limit
}

/**************************************/

#[cfg(test)]
use crate::macros::make_block_macro;

#[test]
fn test_prover() {
    assert_eq!(
        run_prover("1RB 2LA 1RA 1RA  1LB 1LA 3RB ...", 1000),
        MachineResult {
            result: undfnd,
            steps: 36686,
            cycles: 397,
            marks: 2050,
            rulapp: 987,
            blanks: Dict::from([]),
            last_slot: Some((1, 3))
        }
    );

    assert_eq!(
        run_prover("1RB 1LC  1RD 1RB  0RD 0RC  1LD 1LA", 1000),
        MachineResult {
            result: spnout,
            steps: 56459,
            cycles: 229,
            marks: 0,
            rulapp: 5073,
            blanks: Dict::from([(2, 56458), (3, 56459)]),
            last_slot: None
        }
    );

    assert!(run_for_infrul(
        &make_block_macro(
            &CompProg::from_str("1RB 1LA ... 3LA  2LA 3RB 3LA 0RA"),
            (2, 4),
            4,
        ),
        1000,
    ));
}

#[test]
#[should_panic]
fn test_overflow() {
    run_prover("1RB 2LA 3LA 2RA  0LA ... 2RB 3RB", 10_000);
}

/**************************************/

#[cfg(test)]
const REC_PROGS: [(&str, bool); 5] = [
    ("1RB ...  0LB 0LA", true),
    ("1RB 1LA  0LA 0RB", false),
    ("1RB 1LA  0LA 1RB", false),
    ("1RB 0LB  1LA 0RA", false),
    ("1RB 1LA  1LA 1RB", false),
];

#[test]
fn test_rec() {
    for (prog, expected) in REC_PROGS {
        assert_eq!(
            quick_term_or_rec(&CompProg::from_str(prog), 100)
                .is_recur(),
            expected,
            "{prog}",
        );
    }
}
