use core::{fmt, iter::once};

use std::collections::{BTreeMap as Dict, HashSet as Set};

use crate::instrs::{show_slot, Color, CompProg, Shift, State};

type Segments = usize;

const MAX_STEPS: usize = 1_000;

/**************************************/

#[derive(PartialEq, Eq)]
enum Goal {
    Halt,
    Blank,
    Spinout,
}

use Goal::*;

pub fn cps_cant_halt(prog: &CompProg, segs: Segments) -> bool {
    cps_run(prog, segs, &Halt)
}

pub fn cps_cant_blank(prog: &CompProg, segs: Segments) -> bool {
    cps_run(prog, segs, &Blank)
}

pub fn cps_cant_spin_out(prog: &CompProg, segs: Segments) -> bool {
    cps_run(prog, segs, &Spinout)
}

/**************************************/

fn cps_run(prog: &CompProg, segs: Segments, goal: &Goal) -> bool {
    assert!(segs > 1);

    (2..segs).any(|seg| cps_cant_reach(prog, seg, goal))
}

fn cps_cant_reach(
    prog: &CompProg,
    segs: Segments,
    goal: &Goal,
) -> bool {
    let mut configs = Configs::init(segs);

    for _ in 1..MAX_STEPS {
        let mut todo: Vec<Config> =
            configs.seen.clone().into_iter().collect();

        let mut update = false;

        while let Some(config) = todo.pop() {
            let Config { state, mut tape } = config;

            let Some(&(print, shift, next_state)) =
                prog.get(&(state, tape.scan))
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

            for color in colors {
                let push_clone = push.clone();
                let mut pull_clone = pull.clone();
                pull_clone.last = color;

                let next_config = Config {
                    state: next_state,
                    tape: Tape::from_spans(
                        tape.scan, push_clone, pull_clone, shift,
                    ),
                };

                if configs.seen.contains(&next_config) {
                    #[cfg(debug_assertions)]
                    println!("xxx {next_config}");

                    continue;
                }

                #[cfg(debug_assertions)]
                println!("--> {next_config}");

                configs.seen.insert(next_config.clone());
                todo.push(next_config.clone());
                update = true;
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
    fn init(segs: Segments) -> Self {
        let mut configs = Self {
            seen: Set::new(),
            lspans: Dict::new(),
            rspans: Dict::new(),
        };

        let init = Config::init(segs);

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
        self.entry(span.span.clone()).or_default().insert(span.last);
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
    fn init(segs: Segments) -> Self {
        Self {
            state: 0,
            tape: Tape::init(segs),
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
    fn init(segs: Segments) -> Self {
        Self {
            scan: 0,
            lspan: Span::init(segs),
            rspan: Span::init(segs),
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
    fn init(segs: Segments) -> Self {
        assert!(segs > 0);

        Self {
            span: vec![0; segs - 1],
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
