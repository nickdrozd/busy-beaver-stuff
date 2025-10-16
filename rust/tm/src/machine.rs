use crate::{
    Prog, Slot, State, Steps,
    config::Config,
    macros::GetInstr,
    prover::{Prover, ProverResult},
    rules::ApplyRule as _,
    tape::{
        Alignment as _, BigTape, HeadTape, MedTape, Pos, Scan as _,
    },
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

use RunResult::*;

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

trait RunProver {
    fn run_prover(&self, sim_lim: Steps) -> RunResult;
}

impl<T: GetInstr> RunProver for T {
    fn run_prover(&self, sim_lim: Steps) -> RunResult {
        let mut tape = BigTape::init();

        let mut prover = Prover::new(self);

        let mut state: State = 0;

        for cycle in 0..sim_lim {
            if let Some(res) = prover.try_rule(cycle, state, &tape) {
                match res {
                    ProverResult::ConfigLimit => {
                        return ConfigLimit;
                    },
                    ProverResult::InfiniteRule => {
                        return InfiniteRule;
                    },
                    ProverResult::MultRule => {
                        return MultRule;
                    },
                    ProverResult::Got(rule) => {
                        if tape.apply_rule(&rule).is_some() {
                            // println!("--> applying rule: {:?}", rule);
                            continue;
                        }
                    },
                }
            }

            let slot = (state, tape.scan());

            let Some((color, shift, next_state)) =
                self.get_instr(&slot)
            else {
                return Undefined(slot);
            };

            let same = state == next_state;

            if same && tape.at_edge(shift) {
                return Spinout;
            }

            tape.step(shift, color, same);

            state = next_state;
        }

        StepLimit
    }
}

/**************************************/

impl Prog {
    pub fn run_basic(
        &self,
        sim_lim: Steps,
        config: &mut Config<MedTape>,
    ) -> RunResult {
        for _ in 0..sim_lim {
            let slot = config.slot();

            let Some(&(color, shift, state)) = self.get(&slot) else {
                return Undefined(slot);
            };

            let same = config.state == state;

            if same && config.tape.at_edge(shift) {
                return Spinout;
            }

            config.tape.step(shift, color, same);

            if config.tape.blank() {
                return Blank;
            }

            config.state = state;
        }

        StepLimit
    }

    pub fn check_inf(&self, steps: Steps, block_steps: Steps) -> bool {
        let blocks = self.opt_block(block_steps);

        (if blocks == 1 {
            self.run_prover(steps)
        } else {
            self.make_block_macro(blocks).run_prover(steps)
        })
        .is_infinite()
    }

    pub fn term_or_rec(&self, sim_lim: Steps) -> RunResult {
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
                return Undefined(slot);
            };

            let curr_state = state;

            state = next_state;

            let same = curr_state == next_state;

            if same && tape.at_edge(shift) {
                return Spinout;
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
                return Blank;
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
                return Recur;
            }
        }

        StepLimit
    }

    pub fn run_transcript_fresh(&self, sim_lim: Steps) -> RunResult {
        self.run_transcript(sim_lim, &mut Config::init_stepped())
    }

    pub fn run_transcript(
        &self,
        sim_lim: Steps,
        config: &mut Config<MedTape>,
    ) -> RunResult {
        let mut pos = 0;

        let mut transcript = vec![];

        for _ in 0..sim_lim {
            let slot = config.slot();

            transcript.push((slot, pos));

            let Some(&(color, shift, state)) = self.get(&slot) else {
                return Undefined(slot);
            };

            if state == config.state && config.tape.at_edge(shift) {
                return Spinout;
            }

            #[expect(clippy::cast_possible_wrap)]
            let stepped = config.tape.step(shift, color, false) as Pos;

            if shift {
                pos += stepped;
            } else {
                pos -= stepped;
            }

            if config.tape.blank() {
                return Blank;
            }

            config.state = state;
        }

        if has_recurrence(&transcript) {
            return Recur;
        }

        StepLimit
    }
}

type Transcript = Vec<(Slot, Pos)>;

fn has_recurrence(transcript: &Transcript) -> bool {
    let t = transcript.len();

    let min_max = |e: usize| -> (Pos, Pos) {
        assert!(0 < e);

        let mut it = transcript[..e].iter().map(|&(_, p)| p);
        let first = it.next().unwrap();

        let (mut min_p, mut max_p) = (first, first);

        for p in it {
            if p < min_p {
                min_p = p;
            } else if max_p < p {
                max_p = p;
            }
        }

        (min_p, max_p)
    };

    for l in 1..=t / 2 {
        let k = t - l;

        let mut offset = l - 1;

        for i in 0..k {
            let end_l = t - i;
            let end_k = k - i;

            let (slot_l, pos_l) = transcript[end_l - 1];
            let (slot_k, pos_k) = transcript[end_k - 1];

            if slot_l != slot_k {
                break;
            }

            if i == offset {
                if pos_l == pos_k {
                    return true;
                }

                let (min_l, max_l) = min_max(end_l);
                let (min_k, max_k) = min_max(end_k);

                if pos_l == min_l && pos_k == min_k {
                    return true;
                }

                if pos_l == max_l && pos_k == max_k {
                    return true;
                }

                offset += 1;

                if k <= offset {
                    break;
                }
            }
        }
    }

    false
}

/**************************************/

#[cfg(test)]
use crate::Parse as _;

#[cfg(test)]
const REC_PROGS: &[(&str, bool)] = &[
    ("1RB 0LB  1LA 0RB", true),
    ("1RB 1LB  1LA 1RA", true),
    ("1RB 0RB  1LB 1RA", true),
    ("1RB ...  0LB 0LA", false),
    ("1RB 1LA  0LA 0RB", false),
    ("1RB 1LA  0LA 1RB", false),
    ("1RB 0LB  1LA 0RA", false),
    ("1RB 1LA  1LA 1RB", false),
    ("1RB 0LA ...  1LB 2LA 0RB", true),
    ("1RB 1LB 2LA  1LA 2RB 0RA", true),
    ("1RB 1LB  0RC 0LA  1LC 0LA", true),
    ("1RB 1LA  1LC 1RC  1LA 0RB", true),
    ("1RB 0LB  1LA 0RC  1RB ...", false),
    ("1RB 0LC  1LA 1RC  1RC 1RB", false),
    ("1RB 1LA  0LA 1RC  1LA 0RB", false),
    ("1RB 1LA  1LA 1RC  ... 1RB", false),
    ("1RB 1LB  0LC 0RC  1RA 1LA", false),
    ("1RB 1RC  1LC ...  0RA 0LB", false),
    ("1RB 1RD  0LB 1LC  1RC 1LD  0LC 1LA", true),
];

#[test]
fn test_rec() {
    for &(prog, expected) in REC_PROGS {
        assert_eq!(
            Prog::read(prog).term_or_rec(301).is_recur(),
            expected,
            "{prog}",
        );

        assert_eq!(
            Prog::read(prog).run_transcript_fresh(280).is_recur(),
            expected,
            "{prog}",
        );
    }
}

#[test]
fn test_transcript_config() {
    let prog = Prog::read("1RB 0LC  1LA 1RC  1RC 1RB");

    for steps in 0..100 {
        assert!(!prog.run_transcript_fresh(steps).is_settled());

        let mut config = Config::init_stepped();
        config.tape.lspan[0].count += 4;

        assert_eq!(config.to_string(), "B0 | 1^5 [0]");

        assert_eq!(
            (7..=14).contains(&steps),
            prog.run_transcript(steps, &mut config).is_settled(),
        );
    }
}

/**************************************/

#[test]
fn test_mult_rule() {
    assert!(
        Prog::read(
            "1RB 0LD  1RC 0RF  1LC 1LA  0LE ...  1LA 0RB  0RC 0RE",
        )
        .run_prover(10_000)
        .is_mult()
    );
}

#[test]
fn test_check_inf() {
    assert!(
        Prog::read("1RB ... 1RB 3LB  2LB 3LA 3RA 0RB")
            .check_inf(209, 209)
    );

    assert!(
        Prog::read("1RB 0LA 3LB 1RA  2LB 3LA 0RB 2RA")
            .check_inf(756, 300)
    );
}

#[test]
fn test_macro_excess() {
    let prog = Prog::read("1RB 2LA 2RB 3RA  1LB 1RA 3LB 0RB");

    let blocks = prog.opt_block(300);

    assert_eq!(blocks, 7);

    let mac = prog.make_block_macro(blocks);

    mac.assert_params((4, 0x4000));

    assert!(matches!(mac.run_prover(976), StepLimit));

    assert_eq!(mac.rep_params(), (4, 323));
}

#[test]
#[should_panic(expected = "attempt to multiply with overflow")]
fn test_macro_overflow() {
    assert!(
        !Prog::read("1RB 2LA 3RA 0LA  1LA 2RA 0RB ...")
            .check_inf(118, 3_219)
    );
}

#[test]
fn test_macro_loop() {
    assert!(
        Prog::read(
            "1RB 0LC  0RD 1RA  ... 0LD  1LE 1LA  0LF 1LA  0RE 1LF"
        )
        .make_backsymbol_macro(1)
        .run_prover(500)
        .is_infinite()
    );
}
