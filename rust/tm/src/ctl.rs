use ahash::AHashSet as Set;

use crate::{
    Color, Goal, Instr, Prog, Shift, Steps,
    config::LilConfig as Config,
    macros::GetInstr,
    tape::{
        Block as _, LilBlock as Block, LilCount as Count,
        LilTape as Tape, Span,
    },
};

use Goal::*;

const OPT_BLOCK: usize = 500;
const COUNT_LIMIT: Count = 8;
const DEPTH_LIMIT: usize = 20;
const CONFIG_LIMIT: usize = 3_000;

/**************************************/

impl<const s: usize, const c: usize> Prog<s, c> {
    pub fn ctl_cant_halt(&self, steps: Steps) -> bool {
        self.ctl_loop_with_blocks(steps, Halt, OPT_BLOCK)
    }

    pub fn ctl_cant_blank(&self, steps: Steps) -> bool {
        self.ctl_loop(steps, Blank)
    }

    pub fn ctl_cant_spin_out(&self, steps: Steps) -> bool {
        self.ctl_loop_with_blocks(steps, Spinout, OPT_BLOCK)
    }

    fn ctl_loop_with_blocks(
        &self,
        steps: Steps,
        goal: Goal,
        block_steps: usize,
    ) -> bool {
        let blocks = self.opt_block(block_steps);

        if blocks == 1 {
            self.ctl_loop(steps, goal)
        } else {
            self.make_block_macro(blocks).ctl_loop(steps, goal)
        }
    }
}

trait Ctl: GetInstr {
    fn ctl_loop(&self, steps: Steps, goal: Goal) -> bool {
        (2..COUNT_LIMIT)
            .any(|count_limit| ctl_run(self, steps, goal, count_limit))
    }
}

impl<P: GetInstr> Ctl for P {}

/**************************************/

enum RunResult {
    Seen,
    Reached,
    StepLimit,
    Unreachable,
    Branch(Config),
    Exception,
}

use RunResult::*;

fn ctl_run(
    prog: &impl GetInstr,
    steps: Steps,
    goal: Goal,
    count_limit: Count,
) -> bool {
    let mut todo = vec![Config::init()];

    let mut seen: Set<Config> = Set::new();

    for _ in 0..CONFIG_LIMIT {
        let Some(mut config) = todo.pop() else {
            return true;
        };

        if seen.len() > CONFIG_LIMIT {
            return false;
        }

        if seen.contains(&config) {
            continue;
        }

        seen.insert(config.clone());

        let result =
            config.run(prog, steps, goal, &mut seen, count_limit);

        let branched = match result {
            Seen | Unreachable => continue,
            Reached | StepLimit | Exception => return false,
            Branch(branched) => branched,
        };

        if todo.len() > DEPTH_LIMIT {
            return false;
        }

        todo.push(config);
        todo.push(branched);
    }

    false
}

/**************************************/

impl Config {
    fn run(
        &mut self,
        prog: &impl GetInstr,
        steps: Steps,
        goal: Goal,
        seen: &mut Set<Self>,
        count_limit: Count,
    ) -> RunResult {
        for _ in 0..steps {
            let instr @ (_, shift, state) =
                match prog.get_instr(&self.slot()) {
                    Err(_) => return Exception,
                    Ok(None) => {
                        return if goal.is_halt() {
                            Reached
                        } else {
                            Unreachable
                        };
                    },
                    Ok(Some(instr)) => instr,
                };

            if state == self.state && self.tape.at_edge(shift) {
                return match goal {
                    Spinout => Reached,
                    Blank if self.tape.blank() => {
                        unreachable!();
                        // return Reached;
                    },
                    _ => Unreachable,
                };
            }

            if self.step(&instr, count_limit) {
                if self.tape.blank() && goal.is_blank() {
                    return Reached;
                }

                if seen.contains(self) {
                    return Seen;
                }

                seen.insert(self.clone());

                continue;
            }

            let branched = self.branch_step(&instr, count_limit);

            return Branch(branched);
        }

        StepLimit
    }

    fn step(
        &mut self,
        &(print, shift, state): &Instr,
        count_limit: Count,
    ) -> bool {
        if !self.tape.step_with_limit(shift, print, count_limit) {
            return false;
        }

        self.state = state;

        true
    }

    fn branch_step(
        &mut self,
        &(print, shift, state): &Instr,
        count_limit: Count,
    ) -> Self {
        let branch = self.branch_clone(shift, count_limit);

        self.tape.step_no_pull(shift, print, count_limit);

        self.state = state;

        branch
    }

    fn branch_clone(&self, shift: Shift, count_limit: Count) -> Self {
        let mut clone = self.clone();

        let pull = if shift {
            &mut clone.tape.rspan
        } else {
            &mut clone.tape.lspan
        };

        let block = &mut pull[0];

        assert!(block.is_indef());

        block.set_to_limit(count_limit);

        assert!(block.count == count_limit);

        clone
    }
}

/**************************************/

impl Tape {
    fn step_with_limit(
        &mut self,
        shift: Shift,
        print: Color,
        count_limit: Count,
    ) -> bool {
        let (pull, push) = if shift {
            (&mut self.rspan, &mut self.lspan)
        } else {
            (&mut self.lspan, &mut self.rspan)
        };

        let Some(next_scan) = pull.pull_with_limit() else {
            return false;
        };

        push.push_with_limit(print, count_limit);

        self.scan = next_scan;

        true
    }

    fn step_no_pull(
        &mut self,
        shift: Shift,
        print: Color,
        count_limit: Count,
    ) {
        let (pull, push) = if shift {
            (&self.rspan, &mut self.lspan)
        } else {
            (&self.lspan, &mut self.rspan)
        };

        push.push_with_limit(print, count_limit);

        self.scan = pull[0].color;
    }
}

/**************************************/

impl Span<Block> {
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

    fn push_with_limit(&mut self, print: Color, count_limit: Count) {
        match self.first_mut() {
            Some(block) if block.color == print => {
                block.inc_with_limit(count_limit);
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
    const fn inc_with_limit(&mut self, count_limit: Count) {
        match self.count {
            0 => {},
            c if c >= count_limit => {
                self.count = 0;
            },
            _ => {
                self.count += 1;
            },
        }
    }

    fn set_to_limit(&mut self, count_limit: Count) {
        assert!(self.is_indef());

        self.count = count_limit;
    }
}
