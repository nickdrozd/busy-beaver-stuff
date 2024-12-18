use core::{fmt, iter::once};

use std::{
    collections::{BTreeMap, HashSet},
    rc::Rc,
};

use crate::{
    instrs::{Color, CompProg, Instr, Shift, Slot, State},
    tape::{
        Alignment, BasicBlock as Block, Block as _, Count, Pos,
        TapeSlice,
    },
};

pub type Step = usize;
pub type Depth = usize;

const MAX_STACK_DEPTH: Depth = 46;

/**************************************/

pub fn cant_halt(comp: &CompProg, depth: Depth) -> Option<Step> {
    cant_reach(comp, depth, halt_configs)
}

pub fn cant_blank(comp: &CompProg, depth: Depth) -> Option<Step> {
    cant_reach(comp, depth, erase_configs)
}

pub fn cant_spin_out(comp: &CompProg, depth: Depth) -> Option<Step> {
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
) -> Option<Step> {
    let mut configs = get_configs(comp);

    if configs.is_empty() {
        return Some(0);
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

        let valid_steps = get_valid_steps(&mut configs, &entrypoints)?;

        match valid_steps.len() {
            0 => return Some(step),
            n if MAX_STACK_DEPTH < n => return None,
            _ => {},
        }

        configs = step_configs(valid_steps, &mut blanks)?;
    }

    None
}

type ValidatedSteps = Vec<(Vec<Instr>, Config)>;

fn get_valid_steps(
    configs: &mut Configs,
    entrypoints: &Entrypoints,
) -> Option<ValidatedSteps> {
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
                return None;
            }

            good_steps.push((next_color, shift, next_state));
        }

        if good_steps.is_empty() {
            continue;
        }

        checked.push((good_steps, config));
    }

    Some(checked)
}

fn step_configs(
    configs: ValidatedSteps,
    blanks: &mut Blanks,
) -> Option<Configs> {
    let mut stepped = Configs::new();

    for (instrs, config) in configs {
        let config = Rc::new(config);

        for (color, shift, state) in instrs {
            let mut tape = config.tape.clone();

            tape.backstep(shift, color);

            if tape.blank() {
                if state == 0 {
                    return None;
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
                    return None;
                }
            }

            stepped.push(next_config);
        }
    }

    Some(stepped)
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

#[derive(Clone, PartialEq, Eq, Hash)]
struct Backstepper {
    scan: Color,
    lspan: Vec<Block>,
    rspan: Vec<Block>,
    head: Pos,
    l_end: TapeEnd,
    r_end: TapeEnd,
}

impl fmt::Display for Backstepper {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} {} {}",
            self.l_end,
            self.lspan
                .iter()
                .rev()
                .map(ToString::to_string)
                .chain(once(format!("[{}]", self.scan)))
                .chain(self.rspan.iter().map(ToString::to_string))
                .collect::<Vec<_>>()
                .join(" "),
            self.r_end,
        )
    }
}

impl Backstepper {
    const fn init_halt(scan: Color) -> Self {
        Self {
            scan,
            lspan: vec![],
            rspan: vec![],
            head: 0,
            l_end: TapeEnd::Unknown,
            r_end: TapeEnd::Unknown,
        }
    }

    const fn init_blank(scan: Color) -> Self {
        Self {
            scan,
            lspan: vec![],
            rspan: vec![],
            head: 0,
            l_end: TapeEnd::Blanks,
            r_end: TapeEnd::Blanks,
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
            lspan: vec![],
            rspan: vec![],
            head: 0,
            l_end,
            r_end,
        }
    }

    fn blank(&self) -> bool {
        self.scan == 0
            && self
                .lspan
                .iter()
                .chain(self.rspan.iter())
                .all(|block| block.color == 0)
    }

    fn check_step(&self, shift: Shift, print: Color) -> Option<bool> {
        let (pull, pull_end, push) = if shift {
            (&self.lspan, self.l_end, &self.rspan)
        } else {
            (&self.rspan, self.r_end, &self.lspan)
        };

        let (required, at_edge) = if let Some(block) = pull.first() {
            (block.color, false)
        } else if pull_end == TapeEnd::Blanks {
            (0, true)
        } else {
            return Some(!push.is_empty());
        };

        (print == required).then_some(at_edge)
    }

    fn backstep(&mut self, shift: Shift, read: Color) {
        let (stepped, pull, push, push_end) = if shift {
            (-1, &mut self.lspan, &mut self.rspan, self.r_end)
        } else {
            (1, &mut self.rspan, &mut self.lspan, self.l_end)
        };

        if let Some(block) = pull.first_mut() {
            if block.count == 1 {
                pull.remove(0);
            } else {
                block.decrement();
            }
        }

        if self.scan != 0
            || !push.is_empty()
            || push_end == TapeEnd::Unknown
        {
            let color = self.scan;

            if !push.is_empty() && push[0].color == color {
                push[0].increment();
            } else {
                push.insert(0, Block::new(color, 1));
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

    fn get_slice(&self, start: Pos, ltr: bool) -> TapeSlice {
        let (lspan, rspan, diff) = if ltr {
            (&self.lspan, &self.rspan, self.head() - start)
        } else {
            (&self.rspan, &self.lspan, start - self.head())
        };

        let mut tape = TapeSlice::new();

        if diff > 0 {
            #[expect(clippy::cast_sign_loss)]
            let mut remaining = diff as Count;
            for block in lspan {
                let count = (block.count).min(remaining);
                tape.extend(vec![block.color; count as usize]);
                remaining -= count;
            }
            if remaining > 0 {
                tape.extend(vec![0; remaining as usize]);
            }
        }

        for block in rspan {
            tape.extend(vec![block.color; block.count as usize]);
        }

        tape
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

        lspan: vec![],
        l_end: TapeEnd::Blanks,

        rspan: vec![Block::new(1, 1), Block::new(0, 1)],
        r_end: TapeEnd::Unknown,

        head: 0,
    };

    tape.assert("0+ [1] 1 0 ?");

    tape.tbackstep(0, 1, 0, true);

    tape.assert("0+ 1 [0] 0 ?");
}
