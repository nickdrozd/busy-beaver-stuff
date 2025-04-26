use core::fmt;

use std::collections::HashSet as Set;

use crate::{
    blocks::opt_block,
    instrs::{
        show_slot, Color, CompProg, GetInstr, Instr, Shift, State, Term,
    },
    macros::make_block_macro,
    tape::{
        Block as BlockTrait, Count, Span as GenSpan, Tape as GenTape,
    },
};

use Term::*;

type Steps = usize;

const OPT_BLOCK: usize = 500;
const COUNT_LIMIT: Count = 2;
const DEPTH_LIMIT: usize = 20;
const CONFIG_LIMIT: usize = 1_000;

/**************************************/

pub trait Ctl {
    fn ctl_cant_halt(&self, steps: Steps) -> bool;
    fn ctl_cant_blank(&self, steps: Steps) -> bool;
    fn ctl_cant_spin_out(&self, steps: Steps) -> bool;
}

impl Ctl for CompProg {
    fn ctl_cant_halt(&self, steps: Steps) -> bool {
        if self.halt_slots().is_empty() {
            return true;
        }

        let blocks = opt_block(self, OPT_BLOCK);

        if blocks == 1 {
            ctl_run(self, steps, &Halt)
        } else {
            ctl_run(
                &make_block_macro(self, self.params(), blocks),
                steps,
                &Halt,
            )
        }
    }

    fn ctl_cant_blank(&self, steps: Steps) -> bool {
        if self.erase_slots().is_empty() {
            return true;
        }

        ctl_run(self, steps, &Blank)
    }

    fn ctl_cant_spin_out(&self, steps: Steps) -> bool {
        if self.zr_shifts().is_empty() {
            return true;
        }

        let blocks = opt_block(self, OPT_BLOCK);

        if blocks == 1 {
            ctl_run(self, steps, &Spinout)
        } else {
            ctl_run(
                &make_block_macro(self, self.params(), blocks),
                steps,
                &Spinout,
            )
        }
    }
}

/**************************************/

enum RunResult {
    Seen,
    Reached,
    StepLimit,
    Unreachable,
    Branch(Config),
}

use RunResult::*;

fn ctl_run(prog: &impl GetInstr, steps: Steps, goal: &Term) -> bool {
    let mut todo = vec![Config::init()];

    let mut seen: Set<Config> = Set::new();

    for _ in 0..CONFIG_LIMIT {
        let Some(mut config) = todo.pop() else {
            return true;
        };

        #[cfg(debug_assertions)]
        {
            println!("todo: {}", todo.len());
            println!("running: {config}");
        }

        if seen.contains(&config) {
            #[cfg(debug_assertions)]
            println!("seen in loop: {config}");

            continue;
        }

        seen.insert(config.clone());

        let result = config.run(prog, steps, goal, &mut seen);

        let branched = match result {
            Seen | Unreachable => continue,
            Reached | StepLimit => return false,
            Branch(branched) => branched,
        };

        if todo.len() > DEPTH_LIMIT {
            #[cfg(debug_assertions)]
            println!("depth limit");

            return false;
        }

        #[cfg(debug_assertions)]
        {
            println!("adding: {config}");
            println!("adding: {branched}");
        }

        todo.push(config);
        todo.push(branched);
    }

    #[cfg(debug_assertions)]
    println!("config limit");

    false
}

/**************************************/

type Tape = GenTape<LimitBlock>;

#[derive(Clone, PartialEq, Eq, Hash)]
struct Config {
    state: State,
    tape: Tape,
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let tape = &self.tape;
        let slot = show_slot((self.state, tape.scan));

        write!(f, "{slot} | {tape}")
    }
}

impl Config {
    const fn init() -> Self {
        Self {
            state: 0,
            tape: Tape::init(0),
        }
    }

    fn run(
        &mut self,
        prog: &impl GetInstr,
        steps: Steps,
        goal: &Term,
        seen: &mut Set<Self>,
    ) -> RunResult {
        for _ in 0..steps {
            #[cfg(debug_assertions)]
            println!("{self}");

            let slot = (self.state, self.tape.scan);

            let Some(instr) = prog.get_instr(&slot) else {
                return if matches!(goal, &Halt) {
                    #[cfg(debug_assertions)]
                    println!("halt reached");

                    Reached
                } else {
                    #[cfg(debug_assertions)]
                    println!("unreachable");

                    Unreachable
                };
            };

            let (_, shift, state) = instr;

            if state == self.state && self.tape.at_edge(shift) {
                return match goal {
                    Spinout => {
                        #[cfg(debug_assertions)]
                        println!("spinout reached");

                        Reached
                    },
                    Blank if self.tape.blank() => {
                        #[cfg(debug_assertions)]
                        println!("blank reached");

                        unreachable!();
                        // return Reached;
                    },
                    _ => Unreachable,
                };
            }

            if self.step(&instr) {
                if self.tape.blank() && matches!(goal, &Blank) {
                    #[cfg(debug_assertions)]
                    println!("blank reached");

                    return Reached;
                }

                if seen.contains(self) {
                    return Seen;
                }

                seen.insert(self.clone());

                continue;
            }

            let branched = self.branch_step(&instr);

            return Branch(branched);
        }

        #[cfg(debug_assertions)]
        println!("step limit");

        StepLimit
    }

    fn step(&mut self, &(print, shift, state): &Instr) -> bool {
        if !self.tape.step_with_limit(shift, print) {
            return false;
        }

        self.state = state;

        true
    }

    fn branch_step(&mut self, &(print, shift, state): &Instr) -> Self {
        let branch = self.branch_clone(shift);

        self.tape.step_no_pull(shift, print);

        self.state = state;

        branch
    }

    fn branch_clone(&self, shift: Shift) -> Self {
        let mut clone = self.clone();

        let pull = if shift {
            &mut clone.tape.rspan
        } else {
            &mut clone.tape.lspan
        };

        let block = pull.0.first_mut().unwrap();

        assert!(block.count > COUNT_LIMIT);

        block.set_to_limit();

        assert!(block.count == COUNT_LIMIT);

        clone
    }
}

/**************************************/

impl Tape {
    fn step_with_limit(&mut self, shift: Shift, print: Color) -> bool {
        let (pull, push) = if shift {
            (&mut self.rspan, &mut self.lspan)
        } else {
            (&mut self.lspan, &mut self.rspan)
        };

        let Some(next_scan) = pull.pull_with_limit() else {
            return false;
        };

        push.push_with_limit(print);

        self.scan = next_scan;

        true
    }

    fn step_no_pull(&mut self, shift: Shift, print: Color) {
        let (pull, push) = if shift {
            (&self.rspan, &mut self.lspan)
        } else {
            (&self.lspan, &mut self.rspan)
        };

        push.push_with_limit(print);

        self.scan = pull.0[0].color;
    }
}

/**************************************/

type Span = GenSpan<LimitBlock>;

impl Span {
    fn pull_with_limit(&mut self) -> Option<Color> {
        let Some(block) = self.0.first_mut() else {
            return Some(0);
        };

        if block.count > COUNT_LIMIT {
            return None;
        }

        let color = block.color;

        if block.count == 1 {
            self.0.remove(0);
        } else {
            block.decrement();
        }

        Some(color)
    }

    fn push_with_limit(&mut self, print: Color) {
        match self.0.first_mut() {
            Some(block) if block.color == print => {
                block.inc_with_limit();
            },
            None if print == 0 => {},
            _ => {
                self.push_block(print, 1);
            },
        }
    }
}

/**************************************/

#[derive(Clone, PartialEq, Eq, Hash)]
struct LimitBlock {
    color: Color,
    count: Count,
}

impl BlockTrait for LimitBlock {
    fn new(color: Color, count: Count) -> Self {
        Self { color, count }
    }

    fn get_color(&self) -> Color {
        self.color
    }

    fn get_count(&self) -> Count {
        self.count
    }

    fn set_count(&mut self, _: Count) {
        unimplemented!()
    }

    fn add_count(&mut self, _: Count) {
        unimplemented!()
    }

    fn decrement(&mut self) {
        assert!(2 <= self.count);
        assert!(self.count <= COUNT_LIMIT);

        self.count -= 1;
    }

    fn show(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (color, count) = (self.get_color(), self.get_count());

        write!(
            f,
            "{}",
            match count {
                1 => format!("{color}"),
                0 => format!("{color}.."),
                c if c >= COUNT_LIMIT => format!("{color}+"),
                _ => format!("{color}^{count}"),
            }
        )
    }
}

impl fmt::Display for LimitBlock {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.show(f)
    }
}

impl LimitBlock {
    const fn inc_with_limit(&mut self) {
        if self.count > COUNT_LIMIT {
            return;
        }

        self.count += 1;
    }

    fn set_to_limit(&mut self) {
        assert!(self.count > COUNT_LIMIT);

        self.count = COUNT_LIMIT;
    }
}
