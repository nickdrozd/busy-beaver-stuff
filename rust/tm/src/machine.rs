use crate::{
    instrs::{GetInstr, Prog, Slot, State},
    prover::{Prover, ProverResult},
    rules::ApplyRule as _,
    tape::{Alignment as _, BigTape, HeadTape, MachineTape as _},
};

/**************************************/

pub enum RunResult {
    Recur,
    Spinout,
    MultRule,
    InfiniteRule,
    Undefined(Slot),
    StepLimit,
    ConfigLimit,
}

impl RunResult {
    pub const fn is_settled(&self) -> bool {
        self.is_recur() || self.is_infinite() || self.is_undefined()
    }

    pub const fn is_undefined(&self) -> bool {
        matches!(self, Self::Undefined(_))
    }

    pub const fn is_recur(&self) -> bool {
        matches!(self, Self::Recur | Self::Spinout)
    }

    pub const fn is_infinite(&self) -> bool {
        matches!(self, Self::InfiniteRule)
    }

    pub const fn is_mult(&self) -> bool {
        matches!(self, Self::MultRule)
    }
}

/**************************************/

use ProverResult::*;

pub fn run_for_infrul(
    comp: &impl GetInstr,
    sim_lim: usize,
) -> RunResult {
    let mut tape = BigTape::init();

    let mut prover = Prover::new(comp);

    let mut state: State = 0;

    for cycle in 0..sim_lim {
        if let Some(res) = prover.try_rule(cycle, state, &tape) {
            match res {
                ConfigLimit => {
                    return RunResult::ConfigLimit;
                },
                InfiniteRule => {
                    return RunResult::InfiniteRule;
                },
                MultRule => {
                    return RunResult::MultRule;
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
            return RunResult::Undefined(slot);
        };

        let same = state == next_state;

        if same && tape.at_edge(shift) {
            return RunResult::Spinout;
        }

        tape.step(shift, color, same);

        state = next_state;
    }

    RunResult::StepLimit
}

/**************************************/

impl Prog {
    pub fn term_or_rec(&self, sim_lim: usize) -> RunResult {
        let mut state = 1;

        let mut tape = HeadTape::init_stepped();

        let head = tape.head();

        let (mut ref_state, mut ref_tape, mut leftmost, mut rightmost) =
            (state, tape.clone(), head, head);

        let mut reset = 1;

        for cycle in 1..sim_lim {
            let slot = (state, tape.scan());

            let Some((color, shift, next_state)) =
                self.get_instr(&slot)
            else {
                return RunResult::Undefined(slot);
            };

            let curr_state = state;

            state = next_state;

            let same = curr_state == next_state;

            if same && tape.at_edge(shift) {
                return RunResult::Spinout;
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
                return RunResult::Recur;
            }
        }

        RunResult::StepLimit
    }
}

/**************************************/

#[cfg(test)]
use crate::instrs::Parse as _;

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
            Prog::read(prog).term_or_rec(100).is_recur(),
            expected,
            "{prog}",
        );
    }
}

/**************************************/

#[cfg(test)]
use crate::macros::Macro as _;

#[test]
fn test_macro_loop() {
    let prog = Prog::read("1RB 0RA 1LB  2LA 2RB 0LA");
    let block = prog.make_block_macro(3);
    let back = block.make_backsymbol_macro(1);

    assert!(!run_for_infrul(&back, 1000).is_infinite());
}

/**************************************/

#[test]
fn test_mult_rule() {
    assert!(run_for_infrul(
        &Prog::read(
            "1RB 0LD  1RC 0RF  1LC 1LA  0LE ...  1LA 0RB  0RC 0RE",
        ),
        10_000,
    )
    .is_mult());
}
