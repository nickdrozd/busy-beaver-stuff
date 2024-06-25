use core::{fmt, iter::once};
use std::collections::{BTreeMap as Dict, HashSet as Set};

use pyo3::pyfunction;

use crate::{
    instrs::{Color, CompProg, Instr, Shift, Slot, State},
    parse::tcompile,
};

/**************************************/

#[pyfunction]
pub fn cant_halt_py(prog: &str) -> bool {
    cant_halt(&tcompile(prog))
}

#[pyfunction]
pub fn cant_blank_py(prog: &str) -> bool {
    cant_blank(&tcompile(prog))
}

#[pyfunction]
pub fn cant_spin_out_py(prog: &str) -> bool {
    cant_spin_out(&tcompile(prog))
}

/**************************************/

pub fn cant_halt(comp: &CompProg) -> bool {
    cant_reach(comp, halt_configs)
}

pub fn cant_blank(comp: &CompProg) -> bool {
    cant_reach(comp, erase_configs)
}

pub fn cant_spin_out(comp: &CompProg) -> bool {
    cant_reach(comp, zero_reflexive_configs)
}

/**************************************/

type Config = (State, Backstepper);

fn cant_reach(
    comp: &CompProg,
    get_configs: impl Fn(&CompProg) -> Vec<Config>,
) -> bool {
    let mut configs: Vec<(u16, State, Backstepper)> = get_configs(comp)
        .into_iter()
        .map(|(state, tape)| (0, state, tape))
        .collect();

    if configs.is_empty() {
        return true;
    }

    let max_steps = 10;
    let max_cycles = 25;

    let mut seen: Dict<State, Set<Backstepper>> = Dict::new();

    let entrypoints = get_entrypoints(comp);

    for _ in 0..max_cycles {
        let Some((step, state, mut tape)) = configs.pop() else {
            return true;
        };

        let next_step = 1 + step;

        if next_step > max_steps {
            return false;
        }

        if seen.entry(state).or_default().contains(&tape) {
            continue;
        }

        seen.get_mut(&state).unwrap().insert(tape.clone());

        if state == 0 && tape.blank() {
            return false;
        }

        // println!("{step} | {state} | {tape}");

        let Some(instrs) = entrypoints.get(&state) else {
            continue;
        };

        let (last_instr, instrs) = instrs.split_last().unwrap();

        for &((next_state, next_color), (print, shift, _)) in instrs {
            match tape.check_step(shift, print) {
                None => continue,
                Some(at_edge) => {
                    if at_edge
                        && state == next_state
                        && tape.scan == next_color
                    {
                        return false;
                    }
                },
            }

            let mut next_tape = tape.clone();

            next_tape.backstep(shift, next_color);

            configs.push((next_step, next_state, next_tape));
        }

        {
            let &((next_state, next_color), (print, shift, _)) =
                last_instr;

            match tape.check_step(shift, print) {
                None => continue,
                Some(at_edge) => {
                    if at_edge
                        && state == next_state
                        && tape.scan == next_color
                    {
                        return false;
                    }
                },
            }

            tape.backstep(shift, next_color);

            configs.push((next_step, next_state, tape));
        }
    }

    false
}

/**************************************/

fn halt_configs(comp: &CompProg) -> Vec<Config> {
    let mut configs = vec![];

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

fn erase_configs(comp: &CompProg) -> Vec<Config> {
    comp.iter()
        .filter_map(|(&(state, color), &instr)| match instr {
            (0, _, _) if color != 0 => {
                Some((state, Backstepper::init_blank(color)))
            },
            _ => None,
        })
        .collect()
}

fn zero_reflexive_configs(comp: &CompProg) -> Vec<Config> {
    comp.iter()
        .filter_map(|(&slot, &(_, shift, trans))| match slot {
            (state, 0) if trans == state => {
                Some((state, Backstepper::init_spinout(shift)))
            },
            _ => None,
        })
        .collect()
}

fn get_entrypoints(comp: &CompProg) -> Dict<State, Vec<(Slot, Instr)>> {
    let mut entrypoints: Dict<State, Vec<(Slot, Instr)>> = Dict::new();

    for (&slot, &instr) in comp {
        let (_, _, state) = instr;
        entrypoints.entry(state).or_default().push((slot, instr));
    }

    entrypoints
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
            Self::Blanks => true,
            Self::Unknown => false,
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
        let pull = if !shift { &self.rspan } else { &self.lspan };

        let (required, at_edge) = match &pull[0] {
            Square::Unknown => {
                return Some(false);
            },
            Square::Blanks => (0, true),
            Square::Known(color) => (*color, false),
        };

        if print != required {
            None
        } else {
            Some(at_edge)
        }
    }

    fn backstep(&mut self, shift: Shift, read: Color) {
        let (pull, push) = if !shift {
            (&mut self.rspan, &mut self.lspan)
        } else {
            (&mut self.lspan, &mut self.rspan)
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
