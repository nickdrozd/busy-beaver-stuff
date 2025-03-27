use core::{fmt, iter::once};

use std::collections::{BTreeMap as Dict, HashSet as Set};

use crate::instrs::{show_slot, Color, GetInstr, Shift, State, Term};

use Term::*;

type Radius = usize;

const MAX_LOOPS: usize = 1_000;
const MAX_DEPTH: usize = 100_000;

/**************************************/

pub trait Cps {
    fn cps_cant_halt(&self, rad: Radius) -> bool;
    fn cps_cant_blank(&self, rad: Radius) -> bool;
    fn cps_cant_spin_out(&self, rad: Radius) -> bool;
}

impl<T: GetInstr> Cps for T {
    fn cps_cant_halt(&self, rad: Radius) -> bool {
        if self.halt_slots().is_empty() {
            return true;
        }

        cps_run(self, rad, &Halt)
    }

    fn cps_cant_blank(&self, rad: Radius) -> bool {
        if self.erase_slots().is_empty() {
            return true;
        }

        cps_run(self, rad, &Blank)
    }

    fn cps_cant_spin_out(&self, rad: Radius) -> bool {
        if self.zr_shifts().is_empty() {
            return true;
        }

        cps_run(self, rad, &Spinout)
    }
}

/**************************************/

fn cps_run(prog: &impl GetInstr, rad: Radius, goal: &Term) -> bool {
    assert!(rad > 1);

    (2..rad).any(|seg| cps_cant_reach(prog, seg, goal))
}

fn cps_cant_reach(
    prog: &impl GetInstr,
    rad: Radius,
    goal: &Term,
) -> bool {
    let mut configs = Configs::init(rad);

    for _ in 0..MAX_LOOPS {
        let mut todo: Vec<Config> =
            configs.seen.clone().into_iter().collect();

        let mut update = false;

        while let Some(config) = todo.pop() {
            let Config { state, mut tape } = config;

            let Some((print, shift, next_state)) =
                prog.get_instr(&(state, tape.scan))
            else {
                match goal {
                    Halt => return false,
                    _ => continue,
                };
            };

            let ((pull, pull_spans), (push, push_spans)) = if shift {
                (
                    (&mut tape.rspan, &configs.rspans),
                    (&mut tape.lspan, &mut configs.lspans),
                )
            } else {
                (
                    (&mut tape.lspan, &configs.lspans),
                    (&mut tape.rspan, &mut configs.rspans),
                )
            };

            push_spans.add_span(push);

            push.push(print);

            tape.scan = pull.pull();

            let colors = pull_spans.get_colors(pull);

            if *goal != Halt
                && colors.contains(&0)
                && tape.scan == 0
                && pull.blank_span()
                && match goal {
                    Blank => push.all_blank(),
                    Spinout => state == next_state,
                    Halt => false,
                }
            {
                return false;
            }

            let (last_color, colors) = colors.split_last().unwrap();

            for color in colors {
                let next_config = Config {
                    state: next_state,
                    tape: Tape::from_spans(
                        tape.scan,
                        push.clone(),
                        {
                            let mut pull_clone = pull.clone();
                            pull_clone.last = *color;
                            pull_clone
                        },
                        shift,
                    ),
                };

                if configs.seen.contains(&next_config) {
                    continue;
                }

                configs.seen.insert(next_config.clone());
                todo.push(next_config);
                update = true;
            }

            {
                let next_config = Config {
                    state: next_state,
                    tape: {
                        pull.last = *last_color;
                        tape
                    },
                };

                if configs.seen.contains(&next_config) {
                    continue;
                }

                configs.seen.insert(next_config.clone());
                todo.push(next_config);
                update = true;
            };

            if configs.seen.len() > MAX_DEPTH {
                return false;
            }
        }

        if !update {
            return true;
        }
    }

    false
}

/**************************************/

struct Configs {
    seen: Set<Config>,
    lspans: Spans,
    rspans: Spans,
}

impl Configs {
    fn init(rad: Radius) -> Self {
        let mut configs = Self {
            seen: Set::new(),
            lspans: Dict::new(),
            rspans: Dict::new(),
        };

        let init = Config::init(rad);

        configs.lspans.add_span(&init.tape.lspan);
        configs.rspans.add_span(&init.tape.rspan);

        configs.seen.insert(init);

        configs
    }
}

type Spans = Dict<Vec<Color>, Set<Color>>;

trait AddSpan {
    fn add_span(&mut self, span: &Span);
    fn get_colors(&self, span: &Span) -> Vec<Color>;
}

impl AddSpan for Spans {
    fn add_span(&mut self, span: &Span) {
        if let Some(colors) = self.get_mut(&span.span) {
            colors.insert(span.last);
        } else {
            self.insert(span.span.clone(), Set::from([span.last]));
        }
    }

    fn get_colors(&self, span: &Span) -> Vec<Color> {
        let mut colors: Vec<Color> =
            self.get(&span.span).unwrap().iter().copied().collect();
        colors.sort_unstable();
        colors
    }
}

/**************************************/

#[derive(Clone, PartialEq, Eq, Hash)]
struct Config {
    state: State,
    tape: Tape,
}

impl Config {
    fn init(rad: Radius) -> Self {
        Self {
            state: 0,
            tape: Tape::init(rad),
        }
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let tape = &self.tape;
        let slot = show_slot((self.state, tape.scan));

        write!(f, "{slot} | {tape}")
    }
}

/**************************************/

#[derive(Clone, PartialEq, Eq, Hash)]
struct Tape {
    scan: Color,
    lspan: Span,
    rspan: Span,
}

impl Tape {
    fn init(rad: Radius) -> Self {
        Self {
            scan: 0,
            lspan: Span::init(rad),
            rspan: Span::init(rad),
        }
    }

    fn from_spans(
        scan: Color,
        push: Span,
        pull: Span,
        shift: Shift,
    ) -> Self {
        let (lspan, rspan) =
            if shift { (push, pull) } else { (pull, push) };

        Self { scan, lspan, rspan }
    }
}

impl fmt::Display for Tape {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            once(format!("{}", self.lspan.last))
                .chain(
                    self.lspan
                        .span
                        .iter()
                        .rev()
                        .map(ToString::to_string)
                )
                .chain(once(format!("[{}]", self.scan)))
                .chain(self.rspan.span.iter().map(ToString::to_string))
                .chain(once(format!("{}", self.rspan.last)))
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}

/**************************************/

#[derive(Clone, PartialEq, Eq, Hash)]
struct Span {
    span: Vec<Color>,
    last: Color,
}

impl Span {
    fn init(rad: Radius) -> Self {
        assert!(rad > 0);

        Self {
            span: vec![0; rad - 1],
            last: 0,
        }
    }

    fn push(&mut self, color: Color) {
        self.span.insert(0, color);

        self.last = self.span.pop().unwrap();
    }

    fn pull(&mut self) -> Color {
        self.span.push(self.last);

        self.span.remove(0)
    }

    fn blank_span(&self) -> bool {
        self.span.iter().all(|&c| c == 0)
    }

    fn all_blank(&self) -> bool {
        self.last == 0 && self.blank_span()
    }
}

#[test]
fn test_span() {
    let mut span = Span::init(3);

    assert!(
        span == Span {
            span: vec![0, 0],
            last: 0,
        }
    );

    span.push(1);
    span.push(1);
    span.push(0);

    assert!(
        span == Span {
            span: vec![0, 1],
            last: 1,
        }
    );
}
