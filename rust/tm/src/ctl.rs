use ahash::AHashSet as Set;

use crate::{
    Color, Goal, Instr, Prog, Shift, config,
    macros::GetInstr,
    tape::{
        Block as _, LilBlock as Block, LilCount as Count,
        LilTape as Tape, Span,
    },
};

use Goal::*;

type Steps = usize;

const OPT_BLOCK: usize = 500;
const COUNT_LIMIT: Count = 2;
const DEPTH_LIMIT: usize = 20;
const CONFIG_LIMIT: usize = 3_000;

/**************************************/

impl Prog {
    pub fn ctl_cant_halt(&self, steps: Steps) -> bool {
        let blocks = self.opt_block(OPT_BLOCK);

        if blocks == 1 {
            ctl_run(self, steps, Halt)
        } else {
            ctl_run(&self.make_block_macro(blocks), steps, Halt)
        }
    }

    pub fn ctl_cant_blank(&self, steps: Steps) -> bool {
        ctl_run(self, steps, Blank)
    }

    pub fn ctl_cant_spin_out(&self, steps: Steps) -> bool {
        let blocks = self.opt_block(OPT_BLOCK);

        if blocks == 1 {
            ctl_run(self, steps, Spinout)
        } else {
            ctl_run(&self.make_block_macro(blocks), steps, Spinout)
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

fn ctl_run(prog: &impl GetInstr, steps: Steps, goal: Goal) -> bool {
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

        if seen.len() > CONFIG_LIMIT {
            #[cfg(debug_assertions)]
            println!("seen limit");

            return false;
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

type Config = config::Config<Tape>;

impl Config {
    const fn init() -> Self {
        Self {
            state: 0,
            tape: Tape::init(),
        }
    }

    fn run(
        &mut self,
        prog: &impl GetInstr,
        steps: Steps,
        goal: Goal,
        seen: &mut Set<Self>,
    ) -> RunResult {
        for _ in 0..steps {
            #[cfg(debug_assertions)]
            println!("{self}");

            let Some(instr) = prog.get_instr(&self.slot()) else {
                return if goal.is_halt() {
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
                if self.tape.blank() && goal.is_blank() {
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

        let block = &mut pull[0];

        assert!(block.is_indef());

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

        self.scan = pull[0].color;
    }
}

/**************************************/

impl Span<Count, Block> {
    fn pull_with_limit(&mut self) -> Option<Color> {
        let Some(block) = self.first_mut() else {
            return Some(0);
        };

        if block.is_indef() {
            return None;
        }

        let color = block.color;

        if block.count == 1 {
            self.pop_block();
        } else {
            block.decrement();
        }

        Some(color)
    }

    fn push_with_limit(&mut self, print: Color) {
        match self.first_mut() {
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

impl Block {
    const fn inc_with_limit(&mut self) {
        match self.count {
            0 => {},
            c if c >= COUNT_LIMIT => {
                self.count = 0;
            },
            _ => {
                self.count += 1;
            },
        }
    }

    fn set_to_limit(&mut self) {
        assert!(self.is_indef());

        self.count = COUNT_LIMIT;
    }
}
