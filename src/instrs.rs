use std::collections::{BTreeMap as Dict, BTreeSet as Set};

use pyo3::pyfunction;

/**************************************/

pub type Color = u64;
pub type State = u64;
pub type Shift = bool;

pub type Slot = (State, Color);
pub type Instr = (Color, Shift, State);

pub type Params = (State, Color);

pub type CompProg = Dict<Slot, Instr>;

/**************************************/

#[derive(PartialEq, Eq)]
pub enum Term {
    Halt,
    Blank,
    Spinout,
}

/**************************************/

pub trait GetInstr {
    fn get_instr(&self, slot: &Slot) -> Option<Instr>;

    fn halt_slots(&self) -> Set<Slot>;
    fn erase_slots(&self) -> Set<Slot>;
    fn zr_shifts(&self) -> Set<(State, Shift)>;

    fn params(&self) -> Params;

    fn params_unreached(&self, params: Params) -> bool;

    #[cfg(test)]
    fn incomplete(&self, params: Params, halt: bool) -> bool;
}

impl GetInstr for CompProg {
    fn get_instr(&self, slot: &Slot) -> Option<Instr> {
        self.get(slot).copied()
    }

    fn params(&self) -> Params {
        let (states, colors) = self
            .keys()
            .fold((0, 0), |acc, &(a, b)| (acc.0.max(a), acc.1.max(b)));

        (1 + states, 1 + colors)
    }

    fn halt_slots(&self) -> Set<Slot> {
        let mut slots = Set::new();

        let (max_state, max_color) = self.params();

        for state in 0..max_state {
            for color in 0..max_color {
                let slot = (state, color);

                if !self.contains_key(&slot) {
                    slots.insert(slot);
                }
            }
        }

        slots
    }

    fn erase_slots(&self) -> Set<Slot> {
        self.iter()
            .filter_map(|(&(state, color), &instr)| match instr {
                (0, _, _) if color != 0 => Some((state, color)),
                _ => None,
            })
            .collect()
    }

    fn zr_shifts(&self) -> Set<(State, Shift)> {
        self.iter()
            .filter_map(|(&slot, &(_, shift, trans))| match slot {
                (state, 0) if trans == state => Some((state, shift)),
                _ => None,
            })
            .collect()
    }

    fn params_unreached(&self, (max_state, max_color): Params) -> bool {
        self.values().all(|(_, _, state)| 1 + state < max_state)
            || self.values().all(|(color, _, _)| 1 + color < max_color)
    }

    #[cfg(test)]
    fn incomplete(&self, params: Params, halt: bool) -> bool {
        let (states, colors) = params;

        let dimension = (states * colors) as usize;

        if self.len() < (if halt { dimension - 1 } else { dimension }) {
            return true;
        }

        let (used_states, used_colors): (Set<State>, Set<Color>) =
            self.values().map(|(pr, _, tr)| (tr, pr)).unzip();

        (colors == 2 && !used_colors.contains(&0))
            || (0..states).any(|state| !used_states.contains(&state))
            || (1..colors).any(|color| !used_colors.contains(&color))
    }
}

/**************************************/

const UNDF: char = '.';

const LEFT: char = 'L';
const RIGHT: char = 'R';

/**************************************/

pub trait Parse {
    fn from_str(prog: &str) -> Self;
    fn show(&self, params: Option<Params>) -> String;
}

impl Parse for CompProg {
    fn from_str(prog: &str) -> Self {
        prog.trim()
            .split("  ")
            .map(|instrs| instrs.split(' ').map(read_instr))
            .enumerate()
            .flat_map(|(state, instrs)| {
                instrs.enumerate().filter_map(move |(color, instr)| {
                    instr.map(|instr| {
                        ((state as State, color as Color), instr)
                    })
                })
            })
            .collect()
    }

    fn show(&self, params: Option<Params>) -> String {
        let (max_state, max_color) = params.unwrap_or_else(|| {
            let (ms, mx) = self.iter().fold(
                (1, 1),
                |(ms, mc), (&(ss, sc), &(ic, _, is))| {
                    (ms.max(ss).max(is), mc.max(sc).max(ic))
                },
            );

            (1 + ms, 1 + mx)
        });

        (0..max_state)
            .map(|state| {
                (0..max_color)
                    .map(|color| {
                        show_instr(self.get(&(state, color)).copied())
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect::<Vec<_>>()
            .join("  ")
    }
}

/**************************************/

pub fn read_color(color: char) -> Color {
    color.to_digit(10).unwrap().into()
}

pub const fn read_shift(shift: char) -> Shift {
    shift == RIGHT
}

#[pyfunction]
#[pyo3(signature = (state))]
pub const fn show_state(state: Option<State>) -> char {
    match state {
        None => UNDF,
        Some(s) => (s as u8 + 65) as char,
    }
}

pub fn read_state(state: char) -> State {
    State::from(state as u8 - 65)
}

#[pyfunction]
pub fn show_slot(slot: Slot) -> String {
    let (state, color) = slot;
    format!("{}{}", show_state(Some(state)), color)
}

#[pyfunction]
pub fn read_slot(slot: &str) -> Slot {
    let mut chars = slot.chars();
    let state = chars.next().unwrap();
    let color = chars.next().unwrap();

    (read_state(state), read_color(color))
}

#[pyfunction]
#[pyo3(signature = (instr))]
pub fn show_instr(instr: Option<Instr>) -> String {
    match instr {
        None => "...".to_owned(),
        Some((color, shift, trans)) => format!(
            "{}{}{}",
            color,
            if shift { RIGHT } else { LEFT },
            show_state(Some(trans))
        ),
    }
}

#[pyfunction]
pub fn read_instr(instr: &str) -> Option<Instr> {
    if instr.contains(UNDF) {
        return None;
    }

    let mut chars = instr.chars();
    let color = chars.next().unwrap();
    let shift = chars.next().unwrap();
    let state = chars.next().unwrap();

    Some((read_color(color), read_shift(shift), read_state(state)))
}

/**************************************/

#[test]
fn test_state() {
    let states = ['A', 'B', 'C'];

    for state in states {
        assert_eq!(state, show_state(Some(read_state(state))));
    }
}

#[test]
fn test_slot() {
    let slots = ["A0", "A1", "A2", "B0", "B1", "B2", "C0", "C1", "C2"];

    for slot in slots {
        assert_eq!(slot, show_slot(read_slot(slot)));
    }
}

#[test]
fn test_instr() {
    let instrs = ["1RB", "2LC"];

    for instr in instrs {
        assert_eq!(instr, show_instr(read_instr(instr)));
    }
}

#[test]
fn test_comp() {
    let progs = [
        "1RB ...  ... ...",
        "1RB ...  1LA ...",
        "1RB 1LB  1LA ...",
        "1RB ... ...  2LB ... ...",
        "1RB ... ...  2LB 1RB 1LB",
        "1RB 0RB ...  2LA ... 0LB",
        "1RB ...  1LB 0RC  1LC 1LA",
        "1RB 1RC  1LC 1RD  1RA 1LD  0RD 0LB",
        "1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  ... 0LA",
        "1RB 1RC  0LC 1RD  1LB 1LE  1RD 0RA  1LA 0LE",
        "1RB 2RB 3RB 4RB 5LA 4RA  0LA 1RB 5RA ... ... 1LB",
        "1RB ...  1RC ...  1LC 1LD  1RE 1LF  1RC 1RE  0RC 0RF",
    ];

    for prog in progs {
        assert_eq!(CompProg::from_str(prog).show(None), prog);
    }

    let underspecified = [
        ("1RB ... ...  ... ... ...", (2, 3)),
        ("1RB ... ... ...  ... ... ... ...", (2, 4)),
        ("1RB ...  ... ...  ... ...", (3, 2)),
        ("1RB ...  ... ...  ... ...  ... ...", (4, 2)),
    ];

    for (prog, params) in underspecified {
        assert_eq!(CompProg::from_str(prog).show(Some(params)), prog);
    }

    let prog_11_4 = "1RB ... ... ...  2LC 3RD ... ...  1LA 3RD 1LE 4RD  ... ... 1RF ...  1RF 2LG 2LE 2RH  3RI 2RH 3RJ ...  1LE ... ... 2LC  2LE 2RK 2RH ...  1LE ... ... ...  0RI 1RF 0RJ ...  2RB ... 2RF ...";

    assert_eq!(
        CompProg::from_str(prog_11_4).show(Some((11, 4))),
        prog_11_4,
    );

    assert_eq!(
        CompProg::from_str(prog_11_4).show(Some((11, 5))),
        "1RB ... ... ... ...  2LC 3RD ... ... ...  1LA 3RD 1LE 4RD ...  ... ... 1RF ... ...  1RF 2LG 2LE 2RH ...  3RI 2RH 3RJ ... ...  1LE ... ... 2LC ...  2LE 2RK 2RH ... ...  1LE ... ... ... ...  0RI 1RF 0RJ ... ...  2RB ... 2RF ... ...",
    );
}

#[cfg(test)]
const PARAMS: &[(&str, Params)] = &[
    ("1RB 2LA 1RA 1RA  1LB 1LA 3RB ...", (2, 4)),
    ("1RB 1LB  1LA 0LC  ... 1LD  1RD 0RA", (4, 2)),
    ("1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  ... 0LA", (5, 2)),
];

#[test]
fn test_params() {
    for &(prog, params) in PARAMS {
        assert_eq!(CompProg::from_str(prog).params(), params);
    }
}
