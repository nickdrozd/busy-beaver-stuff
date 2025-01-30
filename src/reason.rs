use core::{fmt, iter::once};

use std::{
    collections::{BTreeMap, HashSet},
    rc::Rc,
};

use crate::{
    instrs::{Color, CompProg, Instr, Shift, Slot, State},
    tape::{
        Alignment, BasicBlock as Block, Block as _, Pos,
        Span as GenSpan,
    },
};

pub type Step = usize;
pub type Depth = usize;

const MAX_STACK_DEPTH: Depth = 46;

/**************************************/

pub enum BackwardResult {
    Init,
    LinRec,
    Spinout,
    StepLimit,
    DepthLimit,
    Refuted(Step),
}

use BackwardResult::*;

impl BackwardResult {
    pub const fn is_settled(&self) -> bool {
        matches!(self, Refuted(_) | Init)
    }
}

/**************************************/

pub fn cant_halt(comp: &CompProg, depth: Depth) -> BackwardResult {
    cant_reach(comp, depth, halt_configs)
}

pub fn cant_blank(comp: &CompProg, depth: Depth) -> BackwardResult {
    cant_reach(comp, depth, erase_configs)
}

pub fn cant_spin_out(comp: &CompProg, depth: Depth) -> BackwardResult {
    cant_reach(comp, depth, zero_reflexive_configs)
}

/**************************************/

type Configs = Vec<Config>;
type Blanks = HashSet<State>;
type Entrypoints = BTreeMap<State, Vec<(Slot, Instr)>>;

fn cant_reach(
    comp: &CompProg,
    depth: Depth,
    get_configs: impl Fn(&CompProg) -> Configs,
) -> BackwardResult {
    let mut configs = get_configs(comp);

    if configs.is_empty() {
        return Refuted(0);
    }

    let entrypoints = get_entrypoints(comp);

    configs.retain(|config| entrypoints.contains_key(&config.state));

    let mut blanks: Blanks = configs
        .iter()
        .filter(|config| config.tape.blank())
        .map(|config| config.state)
        .collect();

    for step in 0..depth {
        #[cfg(debug_assertions)]
        {
            for config in &configs {
                println!("{step} | {config}");
            }
            println!();
        };

        let valid_steps =
            match get_valid_steps(&mut configs, &entrypoints) {
                Ok(steps) => steps,
                Err(err) => return err,
            };

        match valid_steps.len() {
            0 => return Refuted(step),
            n if MAX_STACK_DEPTH < n => return DepthLimit,
            _ => {},
        }

        configs = match step_configs(valid_steps, &mut blanks) {
            Ok(configs) => configs,
            Err(err) => return err,
        };
    }

    StepLimit
}

type ValidatedSteps = Vec<(Vec<Instr>, Config)>;

fn get_valid_steps(
    configs: &mut Configs,
    entrypoints: &Entrypoints,
) -> Result<ValidatedSteps, BackwardResult> {
    let mut checked = ValidatedSteps::new();

    for config in configs.drain(..) {
        let Config { state, tape, .. } = &config;

        let mut good_steps = vec![];

        for &((next_state, next_color), (print, shift, _)) in
            &entrypoints[state]
        {
            let Some(at_edge) = tape.check_step(shift, print) else {
                continue;
            };

            if at_edge
                && tape.scan == next_color
                && *state == next_state
            {
                return Err(Spinout);
            }

            good_steps.push((next_color, shift, next_state));
        }

        if good_steps.is_empty() {
            continue;
        }

        checked.push((good_steps, config));
    }

    Ok(checked)
}

fn step_configs(
    configs: ValidatedSteps,
    blanks: &mut Blanks,
) -> Result<Configs, BackwardResult> {
    let mut stepped = Configs::new();

    for (instrs, config) in configs {
        let config = Rc::new(config);

        for (color, shift, state) in instrs {
            let mut tape = config.tape.clone();

            tape.backstep(shift, color);

            if tape.blank() {
                if state == 0 {
                    return Err(Init);
                }

                if blanks.contains(&state) {
                    continue;
                }

                blanks.insert(state);
            }

            let mut next_config = Config {
                state,
                tape,
                prev: Some(Rc::clone(&config)),
                recs: config.recs,
            };

            if next_config.lin_rec() {
                next_config.recs += 1;

                if next_config.recs > 1 {
                    return Err(LinRec);
                }
            }

            stepped.push(next_config);
        }
    }

    Ok(stepped)
}

/**************************************/

fn halt_configs(comp: &CompProg) -> Configs {
    let mut configs = Configs::new();

    let (max_state, max_color) = comp
        .keys()
        .fold((0, 0), |acc, &(a, b)| (acc.0.max(a), acc.1.max(b)));

    for state in 0..=max_state {
        for color in 0..=max_color {
            if !comp.contains_key(&(state, color)) {
                configs.push(Config::new(
                    state,
                    Backstepper::init_halt(color),
                ));
            }
        }
    }

    configs
}

fn erase_configs(comp: &CompProg) -> Configs {
    comp.iter()
        .filter_map(|(&(state, color), &instr)| match instr {
            (0, _, _) if color != 0 => {
                Some(Config::new(state, Backstepper::init_blank(color)))
            },
            _ => None,
        })
        .collect()
}

fn zero_reflexive_configs(comp: &CompProg) -> Configs {
    comp.iter()
        .filter_map(|(&slot, &(_, shift, trans))| match slot {
            (state, 0) if trans == state => Some(Config::new(
                state,
                Backstepper::init_spinout(shift),
            )),
            _ => None,
        })
        .collect()
}

fn get_entrypoints(comp: &CompProg) -> Entrypoints {
    let mut entrypoints = Entrypoints::new();

    for (&slot, &instr) in comp {
        let (_, _, state) = instr;
        entrypoints.entry(state).or_default().push((slot, instr));
    }

    for _ in 0..entrypoints.len() {
        let reached: Vec<State> = entrypoints.keys().copied().collect();

        for instrs in entrypoints.values_mut() {
            instrs.retain(|((state, _), _)| reached.contains(state));
        }

        entrypoints.retain(|_, instrs| !instrs.is_empty());

        if entrypoints.len() == reached.len() {
            break;
        }
    }

    entrypoints
}

#[cfg(test)]
use crate::instrs::Parse as _;

#[cfg(test)]
macro_rules! assert_entrypoints {
    ($prog:expr, [$($state:expr => $instrs:expr),*]) => {
        assert_eq!(
            get_entrypoints(&CompProg::from_str($prog)),
            Entrypoints::from([ $(($state, $instrs)),* ])
        );
    };
}

#[test]
fn test_entrypoints() {
    assert_entrypoints!(
        "1RB ...  1LB 0RB",
        [
            1 => vec![
                ((1, 0), (1, false, 1)),
                ((1, 1), (0, true, 1)),
            ]
        ]
    );

    assert_entrypoints!(
        "1RB ...  0LC ...  1RC 1LD  0LC 0LD",
        [
            2 => vec![
                ((2, 0), (1, true, 2)),
                ((3, 0), (0, false, 2)),
            ],
            3 => vec![
                ((2, 1), (1, false, 3)),
                ((3, 1), (0, false, 3)),
            ]
        ]
    );

    assert_entrypoints!(
        "1RB ...  0LC ...  1RC 1LD  0LC 0LB",
        [
            1 => vec![((3, 1), (0, false, 1))],
            2 => vec![
                ((1, 0), (0, false, 2)),
                ((2, 0), (1, true, 2)),
                ((3, 0), (0, false, 2))
            ],
            3 => vec![((2, 1), (1, false, 3))]
        ]
    );
}

/**************************************/

struct Config {
    state: State,
    tape: Backstepper,
    recs: usize,
    prev: Option<Rc<Config>>,
}

impl Config {
    const fn new(state: State, tape: Backstepper) -> Self {
        Self {
            state,
            tape,
            recs: 0,
            prev: None,
        }
    }

    fn lin_rec(&self) -> bool {
        let head = self.tape.head();
        let mut leftmost = head;
        let mut rightmost = head;

        let mut current = self.prev.clone();

        #[expect(clippy::assigning_clones)]
        while let Some(config) = current {
            let pos = config.tape.head();

            if pos < leftmost {
                leftmost = pos;
            } else if rightmost < pos {
                rightmost = pos;
            }

            if self.state == config.state
                && self.tape.aligns_with(
                    &config.tape,
                    leftmost,
                    rightmost,
                )
            {
                return true;
            }

            current = config.prev.clone();
        }

        false
    }
}

#[cfg(debug_assertions)]
use crate::instrs::show_slot;

#[cfg(debug_assertions)]
impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let tape = &self.tape;
        let slot = show_slot((self.state, tape.scan));

        write!(f, "{slot} | {tape}")
    }
}

/**************************************/

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
enum TapeEnd {
    Blanks,
    Unknown,
}

impl fmt::Display for TapeEnd {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Blanks => write!(f, "0+"),
            Self::Unknown => write!(f, "?"),
        }
    }
}

/**************************************/

#[derive(Clone, PartialEq, Eq, Hash)]
struct Span {
    span: GenSpan<Block>,
    end: TapeEnd,
}

impl Span {
    const fn new(blocks: Vec<Block>, end: TapeEnd) -> Self {
        Self {
            span: GenSpan(blocks),
            end,
        }
    }

    fn blank(&self) -> bool {
        self.span.0.iter().all(|block| block.color == 0)
    }

    fn len(&self) -> usize {
        self.span.len()
    }
}

/**************************************/

#[derive(Clone, PartialEq, Eq, Hash)]
struct Backstepper {
    scan: Color,
    lspan: Span,
    rspan: Span,
    head: Pos,
}

impl fmt::Display for Backstepper {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} {} {}",
            self.lspan.end,
            self.lspan
                .span
                .str_iter()
                .rev()
                .chain(once(format!("[{}]", self.scan)))
                .chain(self.rspan.span.str_iter())
                .collect::<Vec<_>>()
                .join(" "),
            self.rspan.end,
        )
    }
}

impl Backstepper {
    const fn init_halt(scan: Color) -> Self {
        Self {
            scan,
            lspan: Span::new(vec![], TapeEnd::Unknown),
            rspan: Span::new(vec![], TapeEnd::Unknown),
            head: 0,
        }
    }

    const fn init_blank(scan: Color) -> Self {
        Self {
            scan,
            lspan: Span::new(vec![], TapeEnd::Blanks),
            rspan: Span::new(vec![], TapeEnd::Blanks),
            head: 0,
        }
    }

    const fn init_spinout(dir: Shift) -> Self {
        let (l_end, r_end) = if dir {
            (TapeEnd::Unknown, TapeEnd::Blanks)
        } else {
            (TapeEnd::Blanks, TapeEnd::Unknown)
        };

        Self {
            scan: 0,
            lspan: Span::new(vec![], l_end),
            rspan: Span::new(vec![], r_end),
            head: 0,
        }
    }

    fn blank(&self) -> bool {
        self.scan == 0 && self.lspan.blank() && self.rspan.blank()
    }

    fn check_step(&self, shift: Shift, print: Color) -> Option<bool> {
        let (pull, push) = if shift {
            (&self.lspan, &self.rspan)
        } else {
            (&self.rspan, &self.lspan)
        };

        let (required, at_edge) =
            if let Some(block) = pull.span.0.first() {
                (block.color, false)
            } else if pull.end == TapeEnd::Blanks {
                (0, true)
            } else {
                return Some(!push.span.0.is_empty());
            };

        (print == required).then_some(at_edge)
    }

    fn backstep(&mut self, shift: Shift, read: Color) {
        let (stepped, pull, push) = if shift {
            (-1, &mut self.lspan, &mut self.rspan)
        } else {
            (1, &mut self.rspan, &mut self.lspan)
        };

        if let Some(block) = pull.span.0.first_mut() {
            if block.count == 1 {
                pull.span.0.remove(0);
            } else {
                block.decrement();
            }
        }

        if self.scan != 0
            || !push.span.0.is_empty()
            || push.end == TapeEnd::Unknown
        {
            let color = self.scan;

            if !push.span.0.is_empty() && push.span.0[0].color == color
            {
                push.span.0[0].increment();
            } else {
                push.span.0.insert(0, Block::new(color, 1));
            };
        }

        self.scan = read;

        self.head += stepped;
    }
}

impl Alignment for Backstepper {
    fn scan(&self) -> Color {
        self.scan
    }

    fn head(&self) -> Pos {
        self.head
    }

    fn l_len(&self) -> usize {
        self.lspan.len()
    }

    fn r_len(&self) -> usize {
        self.rspan.len()
    }

    fn l_eq(&self, prev: &Self) -> bool {
        self.lspan == prev.lspan
    }

    fn r_eq(&self, prev: &Self) -> bool {
        self.rspan == prev.rspan
    }

    fn l_compare_take(&self, prev: &Self, take: usize) -> bool {
        self.lspan.span.compare_take(&prev.lspan.span, take)
    }

    fn r_compare_take(&self, prev: &Self, take: usize) -> bool {
        self.rspan.span.compare_take(&prev.rspan.span, take)
    }
}

/**************************************/

#[cfg(test)]
impl Backstepper {
    #[track_caller]
    fn assert(&self, exp: &str) {
        assert_eq!(self.to_string(), exp);
    }

    #[track_caller]
    fn tbackstep(
        &mut self,
        shift: u8,
        print: Color,
        read: Color,
        success: bool,
    ) {
        assert!(matches!(shift, 0 | 1));

        let shift = shift != 0;

        let result = self.check_step(shift, print).is_some();

        assert_eq!(result, success);

        if !result {
            return;
        }

        self.backstep(shift, read);
    }
}

#[test]
fn test_backstep_halt() {
    let mut tape = Backstepper::init_halt(2);

    tape.assert("? [2] ?");

    tape.tbackstep(0, 2, 1, true);

    tape.assert("? 2 [1] ?");

    tape.tbackstep(1, 1, 2, false);

    tape.assert("? 2 [1] ?");

    tape.tbackstep(1, 2, 0, true);

    tape.assert("? [0] 1 ?");

    tape.tbackstep(1, 0, 2, true);

    tape.assert("? [2] 0 1 ?");
}

#[test]
fn test_backstep_blank() {
    let mut tape = Backstepper::init_blank(2);

    tape.assert("0+ [2] 0+");

    tape.tbackstep(0, 1, 1, false);
    tape.tbackstep(0, 2, 1, false);
    tape.tbackstep(0, 0, 1, true);

    tape.assert("0+ 2 [1] 0+");

    tape.tbackstep(1, 0, 0, false);
    tape.tbackstep(1, 1, 0, false);
    tape.tbackstep(1, 2, 0, true);

    tape.assert("0+ [0] 1 0+");

    tape.tbackstep(1, 1, 0, false);
    tape.tbackstep(1, 2, 0, false);
    tape.tbackstep(1, 0, 0, true);

    tape.assert("0+ [0] 0 1 0+");
}

#[test]
fn test_backstep_spinout() {
    let mut tape = Backstepper::init_spinout(true);

    tape.assert("? [0] 0+");

    tape.tbackstep(0, 1, 1, false);
    tape.tbackstep(0, 2, 1, false);
    tape.tbackstep(0, 0, 1, true);

    tape.assert("? 0 [1] 0+");

    tape.tbackstep(0, 1, 2, false);
    tape.tbackstep(0, 2, 2, false);
    tape.tbackstep(0, 0, 2, true);

    tape.assert("? 0 1 [2] 0+");

    tape.tbackstep(1, 1, 2, true);
    tape.tbackstep(1, 0, 1, true);
    tape.tbackstep(1, 0, 0, true);
    tape.tbackstep(1, 0, 0, true);

    tape.assert("? [0] 0 1 2^2 0+");
}

#[test]
fn test_backstep_required() {
    let mut tape = Backstepper {
        scan: 1,
        lspan: Span::new(vec![], TapeEnd::Blanks),
        rspan: Span::new(
            vec![Block::new(1, 1), Block::new(0, 1)],
            TapeEnd::Unknown,
        ),
        head: 0,
    };

    tape.assert("0+ [1] 1 0 ?");

    tape.tbackstep(0, 1, 0, true);

    tape.assert("0+ 1 [0] 0 ?");
}

#[test]
fn test_spinout() {
    let mut tape = Backstepper {
        scan: 1,
        head: 0,
        lspan: Span::new(vec![], TapeEnd::Blanks),
        rspan: Span::new(vec![Block::new(0, 2)], TapeEnd::Unknown),
    };

    tape.assert("0+ [1] 0^2 ?");

    assert_eq!(tape.check_step(false, 1), None);
    assert_eq!(tape.check_step(true, 0), Some(true));

    tape.rspan.span.0.insert(0, Block::new(1, 0));

    tape.assert("0+ [1] 1^0 0^2 ?");

    assert_eq!(tape.check_step(false, 1), Some(false));
    assert_eq!(tape.check_step(true, 0), Some(true));
}
