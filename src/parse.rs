use pyo3::pyfunction;

use crate::instrs::{Color, CompThin, Instr, Shift, Slot, State};

const UNDF: char = '.';

const LEFT: char = 'L';
const RIGHT: char = 'R';

pub fn parse(
    prog: &str,
) -> impl Iterator<Item = impl Iterator<Item = Option<Instr>> + '_> + '_
{
    prog.trim()
        .split("  ")
        .map(|instrs| instrs.split(' ').map(read_instr))
}

#[pyfunction]
pub fn parse_to_vec(prog: &str) -> Vec<Vec<Option<Instr>>> {
    parse(prog).map(Iterator::collect).collect()
}

#[pyfunction]
pub fn tcompile(prog: &str) -> CompThin {
    let mut program = CompThin::new();

    for (state, instrs) in parse(prog).enumerate() {
        for (color, instr) in instrs.enumerate() {
            if let Some(instr) = instr {
                program.insert((state as State, color as Color), instr);
            }
        }
    }

    program
}

/**************************************/

fn read_color(color: char) -> Color {
    color.to_digit(10).unwrap().into()
}

const fn read_shift(shift: char) -> Shift {
    shift == RIGHT
}

#[pyfunction]
pub const fn show_state(state: Option<State>) -> char {
    match state {
        None => UNDF,
        Some(s) => (s as u8 + 65) as char,
    }
}

fn read_state(state: char) -> State {
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

fn read_instr(instr: &str) -> Option<Instr> {
    if instr.contains(UNDF) {
        return None;
    }

    let mut chars = instr.chars();
    let color = chars.next().unwrap();
    let shift = chars.next().unwrap();
    let state = chars.next().unwrap();

    Some((read_color(color), read_shift(shift), read_state(state)))
}

#[pyfunction]
#[allow(clippy::needless_pass_by_value)]
pub fn show_comp(comp: CompThin) -> String {
    let max_state =
        comp.keys().map(|(state, _)| *state).max().unwrap_or(0);
    let max_color =
        comp.keys().map(|(_, color)| *color).max().unwrap_or(0);

    let mut result = vec![];

    for state in 0..=max_state {
        let mut state_instrs = vec![];

        for color in 0..=max_color {
            state_instrs
                .push(show_instr(comp.get(&(state, color)).copied()));
        }

        result.push(state_instrs.join(" "));
    }

    result.join("  ")
}

/**************************************/

#[pyfunction]
pub fn init_prog(states: usize, colors: usize) -> String {
    let mut prog = vec![vec!["..."; colors]; states];

    prog[0][0] = "1RB";

    prog.iter()
        .map(|state| state.join(" "))
        .collect::<Vec<String>>()
        .join("  ")
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
fn test_parse() {
    assert_eq!(
        vec![
            vec![Some((1, true, 1)), None],
            vec![Some((1, false, 1)), Some((0, true, 2))],
            vec![Some((1, false, 2)), Some((1, false, 0))],
        ],
        parse_to_vec("1RB ...  1LB 0RC  1LC 1LA"),
    );
}

#[test]
fn test_init() {
    let inits = [
        (2, 3, "1RB ... ...  ... ... ..."),
        (3, 2, "1RB ...  ... ...  ... ..."),
        (2, 4, "1RB ... ... ...  ... ... ... ..."),
        (4, 2, "1RB ...  ... ...  ... ...  ... ..."),
    ];

    for (states, colors, expected) in inits {
        assert_eq!(init_prog(states, colors), expected);
    }
}

#[test]
fn test_comp() {
    let progs = [
        "1RB 1LB  1LA ...",
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
        assert_eq!(show_comp(tcompile(prog)), prog);
    }
}
