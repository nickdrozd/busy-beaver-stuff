use crate::{
    instrs::{CompProg, GetInstr, Slot, State},
    prover::{Prover, ProverResult},
    rules::ApplyRule as _,
    tape::{
        Alignment as _, BasicTape as Tape, HeadTape, MachineTape as _,
    },
};

/**************************************/

use ProverResult::*;

#[expect(dead_code)]
pub fn run_for_infrul(comp: &impl GetInstr, sim_lim: u64) -> bool {
    let mut tape = Tape::init(0);

    let mut prover = Prover::new(comp);

    let mut state: State = 0;

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
            quick_term_or_rec(&CompProg::from_str(prog), 100)
                .is_recur(),
            expected,
            "{prog}",
        );
    }
}

/**************************************/

#[cfg(test)]
use crate::macros::{make_backsymbol_macro, make_block_macro};

#[test]
fn test_macro_loop() {
    let prog = CompProg::from_str("1RB 0RA 1LB  2LA 2RB 0LA");
    let block = make_block_macro(&prog, (2, 3), 3);
    let back = make_backsymbol_macro(&block, (2, 3), 1);

    assert!(!run_for_infrul(&back, 1000));
}
