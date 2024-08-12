use core::{fmt, iter::once};

use std::collections::{BTreeMap, HashSet};

use crate::instrs::{Color, CompProg, Instr, Shift, Slot, State};

pub type Depth = usize;

const MAX_STACK_DEPTH: Depth = 30;

/**************************************/

pub fn cant_halt(comp: &CompProg, depth: Depth) -> bool {
    cant_reach(comp, depth, halt_configs)
}

pub fn cant_blank(comp: &CompProg, depth: Depth) -> bool {
    cant_reach(comp, depth, erase_configs)
}

pub fn cant_spin_out(comp: &CompProg, depth: Depth) -> bool {
    cant_reach(comp, depth, zero_reflexive_configs)
}

/**************************************/

type Config = (State, Backstepper);

type Configs = Vec<Config>;
type Blanks = HashSet<State>;
type Entrypoints = BTreeMap<State, Vec<(Slot, Instr)>>;

fn cant_reach(
    comp: &CompProg,
    depth: Depth,
    get_configs: impl Fn(&CompProg) -> Configs,
) -> bool {
    let mut configs = get_configs(comp);

    if configs.is_empty() {
        return true;
    }

    let mut blanks = Blanks::new();

    let entrypoints = get_entrypoints(comp);

    configs.retain(|(state, _)| entrypoints.contains_key(state));

    for _level in 0..depth {
        // for (state, tape) in &configs {
        //     println!("{_level} | {state} | {tape}");
        // }
        // println!("");

        let Some(valid_steps) =
            get_valid_steps(&mut configs, &mut blanks, &entrypoints)
        else {
            return false;
        };

        match valid_steps.len() {
            0 => return true,
            n if MAX_STACK_DEPTH < n => return false,
            _ => {},
        }

        configs = step_configs(valid_steps);
    }

    false
}

type ValidatedSteps = Vec<(Vec<(State, Color, Shift)>, Backstepper)>;

fn get_valid_steps(
    configs: &mut Configs,
    blanks: &mut Blanks,
    entrypoints: &Entrypoints,
) -> Option<ValidatedSteps> {
    let mut checked = ValidatedSteps::new();

    for (state, tape) in configs.drain(..) {
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

        checked.push((good_steps, tape));
    }

    Some(checked)
}

fn step_configs(configs: ValidatedSteps) -> Configs {
    let mut stepped = Configs::new();

    for (instrs, mut tape) in configs {
        let (last_instr, instrs) = instrs.split_last().unwrap();

        for &(next_state, next_color, shift) in instrs {
            let mut next_tape = tape.clone();

            next_tape.backstep(shift, next_color);

            stepped.push((next_state, next_tape));
        }

        {
            let &(next_state, next_color, shift) = last_instr;

            tape.backstep(shift, next_color);

            stepped.push((next_state, tape));
        }
    }

    stepped
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
                configs.push((state, Backstepper::init_halt(color)));
            }
        }
    }

    configs
}

fn erase_configs(comp: &CompProg) -> Configs {
    comp.iter()
        .filter_map(|(&(state, color), &instr)| match instr {
            (0, _, _) if color != 0 => {
                Some((state, Backstepper::init_blank(color)))
            },
            _ => None,
        })
        .collect()
}

fn zero_reflexive_configs(comp: &CompProg) -> Configs {
    comp.iter()
        .filter_map(|(&slot, &(_, shift, trans))| match slot {
            (state, 0) if trans == state => {
                Some((state, Backstepper::init_spinout(shift)))
            },
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
use crate::parse::tcompile;

#[test]
fn test_entrypoints() {
    assert_eq!(
        get_entrypoints(&tcompile(
            "1RB ...  0LC ...  1RC 1LD  0LC 0LD"
        )),
        Entrypoints::from([
            (2, vec![((2, 0), (1, true, 2)), ((3, 0), (0, false, 2))]),
            (3, vec![((2, 1), (1, false, 3)), ((3, 1), (0, false, 3))]),
        ]),
    );

    assert_eq!(
        get_entrypoints(&tcompile(
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
    Known(u64),
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
        }
    }

    fn init_blank(scan: Color) -> Self {
        Self {
            scan,
            lspan: vec![Square::Blanks],
            rspan: vec![Square::Blanks],
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
            (&mut self.lspan, &mut self.rspan)
        } else {
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

#[cfg(test)]
impl Backstepper {
    fn assert(&self, exp: &str) {
        assert_eq!(self.to_string(), exp);
    }

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
