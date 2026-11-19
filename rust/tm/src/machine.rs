use num_traits::ToPrimitive as _;

use crate::{
    Instr, Prog, Slot, Steps,
    config::{BigConfig, Config, MedConfig},
    prover::{Prover, ProverResult},
    rules::{ApplyRule as _, Rule},
    tape::{Block, DynamicTape, HeadTape, LinRec, Tape},
};

/**************************************/

#[expect(variant_size_differences)]
pub enum RunResult {
    Blank,
    Recur(LinRec),
    Spinout,
    MultRule(Rule),
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

    pub const fn is_spinout(&self) -> bool {
        matches!(self, Self::Spinout)
    }

    pub const fn is_recur(&self) -> bool {
        matches!(self, Self::Recur(_) | Self::Spinout)
    }

    pub const fn is_stationary(&self) -> bool {
        matches!(self, Self::Recur(LinRec::Stationary))
    }

    pub const fn is_translated(&self) -> bool {
        matches!(self, Self::Recur(LinRec::Translated))
    }

    pub const fn is_infinite(&self) -> bool {
        matches!(self, Self::InfiniteRule)
    }

    pub const fn is_mult(&self) -> bool {
        matches!(self, Self::MultRule(_))
    }

    pub const fn is_limit(&self) -> bool {
        matches!(self, Self::StepLimit | Self::ConfigLimit)
    }
}

/**************************************/

// The prover now runs only ordinary programs.
fn run_prover_program<const s: usize, const c: usize>(
    prog: &Prog<s, c>,
    sim_lim: Steps,
) -> RunResult {
    let mut config: Config<DynamicTape> = Config::init();
    let mut prover = Prover::new();

    for cycle in 0..sim_lim {
        if let Some(res) = prover.try_rule(cycle, &config, prog) {
            match res {
                ProverResult::ConfigLimit => return ConfigLimit,
                ProverResult::InfiniteRule => return InfiniteRule,
                ProverResult::Got(rule) => {
                    if rule.is_mult() {
                        return MultRule(rule);
                    }
                    if config.tape.apply_rule(&rule).is_some() {
                        config.tape.normalize();
                        continue;
                    }
                },
            }
        }

        let slot = config.slot();
        let Some(&(color, shift, next_state)) = prog.get(&slot) else {
            return Undefined(slot);
        };
        let same = config.state == next_state;
        if same && config.tape.at_edge(shift) {
            return Spinout;
        }
        config.tape.step(shift, color, same);
        config.state = next_state;
    }

    StepLimit
}

/**************************************/

const MED_LIMIT: Steps = 1 << 16;

impl<const s: usize, const c: usize> Prog<s, c> {
    pub fn run_prover(&self, sim_lim: Steps) -> RunResult {
        run_prover_program(self, sim_lim)
    }

    /// Returns true if the current step blanks/spins out, or the next step
    /// halts/spins out, without modifying the configuration.
    #[inline]
    pub fn term_immediate(
        &self,
        config: &MedConfig,
        (color, shift, state): Instr,
    ) -> bool {
        let same = config.state == state;

        if same && config.tape.at_edge(shift) {
            return true;
        }

        let next_tape = config.tape.preview_step(shift, color, same);

        if next_tape.blank {
            return true;
        }

        let next_slot = (state, next_tape.scan);

        let Some(&(_, next_shift, next_state)) = self.get(&next_slot)
        else {
            return true;
        };

        next_state == state && next_tape.at_edge(next_shift)
    }

    pub fn run_basic(
        &self,
        sim_lim: Steps,
        config: &mut MedConfig,
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

    pub fn prover_settled(&self, steps: Steps) -> bool {
        self.run_prover(steps).is_settled()
    }

    pub fn check_inf(&self, steps: Steps) -> bool {
        self.run_prover(steps).is_infinite()
    }

    pub fn term_or_rec_fresh(&self, sim_lim: Steps) -> RunResult {
        self.term_or_rec(sim_lim, &mut MedConfig::init_stepped())
    }

    pub fn term_or_rec(
        &self,
        sim_lim: Steps,
        config: &mut MedConfig,
    ) -> RunResult {
        if sim_lim < MED_LIMIT {
            self.term_or_rec_config(sim_lim, config)
        } else {
            self.term_or_rec_config(
                sim_lim,
                &mut BigConfig::from(&*config),
            )
        }
    }

    fn term_or_rec_config<B: Block>(
        &self,
        sim_lim: Steps,
        config: &mut Config<Tape<B>>,
    ) -> RunResult {
        let mut head = 1;

        let (mut ref_config, mut ref_head, mut leftmost, mut rightmost) =
            (config.clone(), head, head, head);

        let mut reset = 1;

        for cycle in 1..sim_lim {
            let slot = config.slot();

            let Some(&(color, shift, next_state)) = self.get(&slot)
            else {
                return Undefined(slot);
            };

            let same = config.state == next_state;

            if same && config.tape.at_edge(shift) {
                return Spinout;
            }

            if 0 < reset {
                reset -= 1;
            } else {
                ref_config = config.clone();
                ref_head = head;
                leftmost = head;
                rightmost = head;
                reset = cycle - 1;
            }

            let stepped = config
                .tape
                .step(shift, color, same)
                .to_isize()
                .unwrap();

            if shift {
                head += stepped;
            } else {
                head -= stepped;
            }

            if config.tape.blank() {
                return Blank;
            }

            if head < leftmost {
                leftmost = head;
            } else if rightmost < head {
                rightmost = head;
            }

            if next_state == ref_config.state
                && let Some(linrec) = HeadTape::new(head, &config.tape)
                    .aligns_with(
                        &HeadTape::new(ref_head, &ref_config.tape),
                        leftmost,
                        rightmost,
                    )
            {
                return Recur(linrec);
            }

            config.state = next_state;
        }

        StepLimit
    }
}

/**************************************/

#[test]
fn test_mult_rule() {
    assert!(
        Prog::<6, 2>::from(
            "1RB 0LD  1RC 0RF  1LC 1LA  0LE ...  1LA 0RB  0RC 0RE",
        )
        .run_prover(10_000)
        .is_mult()
    );

    assert!(
        Prog::<3, 3>::from("1RB 0LB 2LA  1LA 0RC 0LB  2RC 2RB 0LC")
            .run_prover(300)
            .is_mult()
    );

    assert!(
        Prog::<3, 3>::from("1RB 1RC 2RA  0RC 2RA 0RB  2LC 2LB 0LA")
            .run_prover(400)
            .is_mult()
    );

    assert!(
        Prog::<2, 6>::from(
            "1RB 3RB 5RA 1LB 5LA 2LB  2LA 2RA 4RB ... 3LB 2LA",
        )
        .run_prover(2_100)
        .is_mult()
    );

    assert!(
        Prog::<2, 6>::from(
            "1RB 2LA ... 1RB 5RB 0RB  2LA 4RA 3LB 5LB 5RA 4LB",
        )
        .run_prover(2_100)
        .is_mult()
    );
}

#[test]
fn test_check_inf() {
    assert!(
        Prog::<2, 4>::from("1RB ... 1RB 3LB  2LB 3LA 3RA 0RB")
            .check_inf(1000)
    );

    assert!(
        Prog::<2, 4>::from("1RB 0LA 3LB 1RA  2LB 3LA 0RB 2RA")
            .check_inf(2000)
    );
}

#[test]
fn test_macro_loop() {
    assert!(
        Prog::<6, 2>::from(
            "1RB 0LC  0RD 1RA  ... 0LD  1LE 1LA  0LF 1LA  0RE 1LF"
        )
        .run_prover(1000)
        .is_infinite()
    );
}

#[test]
fn test_prover_failure() {
    let progs = &[
        "1RB ...  1LC 0LD  1RA 0LD  1RE 0LC  0RE 1RF  1LF 1LB",
        "1RB 1LB  1LC ...  1RA 0LD  1RE 0LC  0RE 1RF  1LA 1LC",
    ];

    for &prog in progs {
        assert!(Prog::<6, 2>::from(prog).run_prover(1000).is_limit());
    }
}

#[test]
fn test_simple_case() {
    let prog = Prog::<3, 2>::from("1RB 0LB  0LC 0LA  1RC 1LB");

    assert!(prog.run_prover(1000).is_infinite());
}

#[test]
fn test_overflow() {
    assert!(
        !Prog::<5, 2>::from(
            "1RB 1LE  0LC 0LB  0LD 1LC  1RD 1RA  0RC 0LA"
        )
        .term_or_rec_fresh(1_000_000)
        .is_settled()
    );
}
