use crate::{
    Prog, Slot, State,
    macros::GetInstr,
    prover::{Prover, ProverResult},
    rules::ApplyRule as _,
    tape::{Alignment as _, BigTape, HeadTape},
};

/**************************************/

pub enum RunResult {
    Blank,
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
        self.is_recur()
            || self.is_infinite()
            || self.is_undefined()
            || self.is_blank()
    }

    pub const fn is_undefined(&self) -> bool {
        matches!(self, Self::Undefined(_))
    }

    pub const fn is_blank(&self) -> bool {
        matches!(self, Self::Blank)
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
    prog: &impl GetInstr,
    sim_lim: usize,
) -> RunResult {
    let mut tape = BigTape::init();

    let mut prover = Prover::new(prog);

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

        let Some((color, shift, next_state)) = prog.get_instr(&slot)
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
    pub fn check_inf(&self, block_steps: usize, steps: usize) -> bool {
        let blocks = self.opt_block(block_steps);

        (if blocks == 1 {
            run_for_infrul(self, steps)
        } else {
            run_for_infrul(&self.make_block_macro(blocks), steps)
        })
        .is_infinite()
            || run_for_infrul(&self.make_backsymbol_macro(1), steps)
                .is_infinite()
    }

    pub fn term_or_rec(&self, sim_lim: usize) -> RunResult {
        let mut state = 1;

        let mut tape = HeadTape::init_stepped();

        let init_head = tape.head();

        let (mut ref_state, mut ref_tape, mut leftmost, mut rightmost) =
            (state, tape.clone(), init_head, init_head);

        let mut reset = 1;

        for cycle in 1..sim_lim {
            let slot = (state, tape.scan());

            let Some(&(color, shift, next_state)) = self.get(&slot)
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

            if tape.blank() {
                return RunResult::Blank;
            }

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
use crate::Parse as _;

#[cfg(test)]
const REC_PROGS: [(&str, bool); 8] = [
    ("1RB 0LB  1LA 0RB", true),
    ("1RB 1LB  1LA 1RA", true),
    ("1RB 0RB  1LB 1RA", true),
    ("1RB ...  0LB 0LA", false),
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

#[test]
fn test_mult_rule() {
    assert!(
        run_for_infrul(
            &Prog::read(
                "1RB 0LD  1RC 0RF  1LC 1LA  0LE ...  1LA 0RB  0RC 0RE",
            ),
            10_000,
        )
        .is_mult()
    );
}

#[test]
fn test_check_inf() {
    assert!(
        Prog::read("1RB ... 1RB 3LB  2LB 3LA 3RA 0RB")
            .check_inf(300, 209)
    );

    assert!(
        Prog::read("1RB 0LA 3LB 1RA  2LB 3LA 0RB 2RA")
            .check_inf(300, 756)
    );
}

#[test]
fn test_macro_excess() {
    let prog = Prog::read("1RB 2LA 2RB 3RA  1LB 1RA 3LB 0RB");

    let blocks = prog.opt_block(300);

    assert_eq!(blocks, 7);

    let mac = prog.make_block_macro(blocks);

    mac.assert_params((4, 0x4000));

    assert!(matches!(run_for_infrul(&mac, 976), RunResult::StepLimit));

    assert_eq!(mac.rep_params(), (4, 323));
}

#[test]
#[should_panic(expected = "attempt to multiply with overflow")]
fn test_macro_overflow() {
    assert!(
        !Prog::read("1RB 2LA 3RA 0LA  1LA 2RA 0RB ...")
            .check_inf(3_219, 118)
    );
}

#[test]
fn test_macro_loop() {
    assert!(
        Prog::read(
            "1RB 0LC  0RD 1RA  ... 0LD  1LE 1LA  0LF 1LA  0RE 1LF"
        )
        .check_inf(200, 500)
    );
}
