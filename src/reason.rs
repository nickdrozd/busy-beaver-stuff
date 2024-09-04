use core::{fmt, iter::once};

use std::{
    collections::{BTreeMap, HashSet},
    rc::Rc,
};

use crate::{
    instrs::{Color, CompProg, Instr, Shift, Slot, State},
    tape::{Alignment, Pos, TapeSlice},
};

pub type Step = usize;
pub type Depth = usize;

const MAX_STACK_DEPTH: Depth = 31;

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

struct Config {
    state: State,
    tape: Backstepper,
    recs: usize,
    prev: Option<Rc<Config>>,
}

impl Alignment for Config {
    fn head(&self) -> Pos {
        self.tape.head
    }

    fn aligns_with(
        &self,
        prev: &Self,
        leftmost: Pos,
        rightmost: Pos,
    ) -> bool {
        self.state == prev.state
            && self.tape.aligns_with(&prev.tape, leftmost, rightmost)
    }
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
        let head = self.head();
        let mut leftmost = head;
        let mut rightmost = head;

        let mut current = self.prev.clone();

        #[expect(clippy::assigning_clones)]
        while let Some(config) = current {
            let pos = config.head();

            if pos < leftmost {
                leftmost = pos;
            } else if rightmost < pos {
                rightmost = pos;
            }

            if self.aligns_with(&config, leftmost, rightmost) {
                return true;
            }

            current = config.prev.clone();
        }

        false
    }
}

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

    let mut blanks = Blanks::new();

    let entrypoints = get_entrypoints(comp);

    configs.retain(|config| entrypoints.contains_key(&config.state));

    for step in 0..depth {
        // for config in &configs {
        //     println!("{step} | {} | {}", config.state, config.tape);
        // }
        // println!("");

        let valid_steps =
            get_valid_steps(&mut configs, &mut blanks, &entrypoints)?;

        match valid_steps.len() {
            0 => return Some(step),
            n if MAX_STACK_DEPTH < n => return None,
            _ => {},
        }

        configs = step_configs(valid_steps)?;
    }

    None
}

type ValidatedSteps = Vec<(Vec<(State, Color, Shift)>, Config)>;

fn get_valid_steps(
    configs: &mut Configs,
    blanks: &mut Blanks,
    entrypoints: &Entrypoints,
) -> Option<ValidatedSteps> {
    let mut checked = ValidatedSteps::new();

    for config in configs.drain(..) {
        let Config {
            state,
            ref tape,
            prev: _,
            recs: _,
        } = config;

        if tape.blank() {
            if state == 0 {
                return None;
            }

            if blanks.contains(&state) {
                continue;
            }

            blanks.insert(state);
        }

        let mut good_steps = vec![];

        for &((next_state, next_color), (print, shift, _)) in
            &entrypoints[&state]
        {
            let Some(at_edge) = tape.check_step(shift, print) else {
                continue;
            };

            if at_edge && tape.scan == next_color && state == next_state
            {
                return None;
            }

            good_steps.push((next_state, next_color, shift));
        }

        if good_steps.is_empty() {
            continue;
        }

        checked.push((good_steps, config));
    }

    Some(checked)
}

fn step_configs(configs: ValidatedSteps) -> Option<Configs> {
    let mut stepped = Configs::new();

    for (instrs, config) in configs {
        let config_rc = Rc::new(config);

        for (next_state, next_color, shift) in instrs {
            let mut next_tape = config_rc.tape.clone();

            next_tape.backstep(shift, next_color);

            let mut next_config = Config {
                state: next_state,
                tape: next_tape,
                prev: Some(config_rc.clone()),
                recs: config_rc.recs,
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
use crate::instrs::Parse;

#[test]
fn test_entrypoints() {
    assert_eq!(
        get_entrypoints(&CompProg::from_str(
            "1RB ...  0LC ...  1RC 1LD  0LC 0LD"
        )),
        Entrypoints::from([
            (2, vec![((2, 0), (1, true, 2)), ((3, 0), (0, false, 2))]),
            (3, vec![((2, 1), (1, false, 3)), ((3, 1), (0, false, 3))]),
        ]),
    );

    assert_eq!(
        get_entrypoints(&CompProg::from_str(
            "1RB ...  0LC ...  1RC 1LD  0LC 0LB"
        )),
        Entrypoints::from([
            (1, vec![((3, 1), (0, false, 1))]),
            (
                2,
                vec![
                    ((1, 0), (0, false, 2)),
                    ((2, 0), (1, true, 2)),
                    ((3, 0), (0, false, 2))
                ]
            ),
            (3, vec![((2, 1), (1, false, 3))]),
        ]),
    );
}

/**************************************/

#[derive(Clone, PartialEq, Eq, Hash)]
enum Square {
    Blanks,
    Unknown,
    Known(Color),
}

impl Square {
    const fn blank(&self) -> bool {
        match self {
            Self::Blanks | Self::Unknown => true,
            Self::Known(color) => *color == 0,
        }
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Blanks => write!(f, "0+"),
            Self::Unknown => write!(f, "?"),
            Self::Known(color) => write!(f, "{color}"),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
struct Backstepper {
    scan: Color,
    lspan: Vec<Square>,
    rspan: Vec<Square>,
    head: Pos,
}

impl fmt::Display for Backstepper {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            self.lspan
                .iter()
                .rev()
                .map(ToString::to_string)
                .chain(once(format!("[{}]", self.scan)))
                .chain(self.rspan.iter().map(ToString::to_string))
                .collect::<Vec<_>>()
                .join(" ")
        )
    }
}

impl Backstepper {
    fn init_halt(scan: Color) -> Self {
        Self {
            scan,
            lspan: vec![Square::Unknown],
            rspan: vec![Square::Unknown],
            head: 0,
        }
    }

    fn init_blank(scan: Color) -> Self {
        Self {
            scan,
            lspan: vec![Square::Blanks],
            rspan: vec![Square::Blanks],
            head: 0,
        }
    }

    fn init_spinout(dir: Shift) -> Self {
        let (l_val, r_val) = if dir {
            (Square::Unknown, Square::Blanks)
        } else {
            (Square::Blanks, Square::Unknown)
        };

        Self {
            scan: 0,
            lspan: vec![l_val],
            rspan: vec![r_val],
            head: 0,
        }
    }

    fn blank(&self) -> bool {
        self.scan == 0
            && self
                .lspan
                .iter()
                .chain(self.rspan.iter())
                .all(Square::blank)
    }

    fn check_step(&self, shift: Shift, print: Color) -> Option<bool> {
        let pull = if shift { &self.lspan } else { &self.rspan };

        let (required, at_edge) = match &pull[0] {
            Square::Unknown => {
                return Some(matches!(
                    (if shift { &self.rspan } else { &self.lspan })[0],
                    Square::Known(_)
                ));
            },
            Square::Blanks => (0, true),
            Square::Known(color) => (*color, false),
        };

        (print == required).then_some(at_edge)
    }

    fn backstep(&mut self, shift: Shift, read: Color) {
        let (pull, push) = if shift {
            self.head -= 1;
            (&mut self.lspan, &mut self.rspan)
        } else {
            self.head += 1;
            (&mut self.rspan, &mut self.lspan)
        };

        if let Square::Known(_) = &pull[0] {
            pull.remove(0);
        }

        if !(self.scan == 0 && push[0] == Square::Blanks) {
            push.insert(0, Square::Known(self.scan));
        }

        self.scan = read;
    }
}

impl Alignment for Backstepper {
    fn scan(&self) -> Color {
        self.scan
    }

    fn head(&self) -> Pos {
        self.head
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
            let diff = diff as usize;

            for square in lspan.iter().take(diff) {
                tape.push(match square {
                    Square::Blanks => 0,
                    Square::Known(color) => *color,
                    Square::Unknown => continue,
                });
            }

            let rem = diff - tape.iter().len();

            if rem > 0 {
                tape.extend(vec![0; rem]);
            }
        }

        tape.push(self.scan());

        for square in rspan {
            tape.push(match square {
                Square::Blanks => 0,
                Square::Known(color) => *color,
                Square::Unknown => continue,
            });
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

    tape.assert("? [0] 0 1 2 2 0+");
}
