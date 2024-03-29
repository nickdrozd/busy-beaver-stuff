use pyo3::prelude::*;

use crate::instrs::{Color, CompThic, CompThin, Instr, Shift, Slot, State};

const UNDF: char = '.';

const LEFT: char = 'L';
const RIGHT: char = 'R';

pub fn parse(prog: &str) -> impl Iterator<Item = impl Iterator<Item = Option<Instr>> + '_> + '_ {
    prog.trim()
        .split("  ")
        .map(|instrs| instrs.split(' ').map(read_instr))
}

#[pyfunction]
pub fn parse_to_vec(prog: &str) -> Vec<Vec<Option<Instr>>> {
    parse(prog).map(std::iter::Iterator::collect).collect()
}

#[pyfunction]
pub fn comp_thin(prog: &str) -> CompThin {
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

#[pyfunction]
pub fn comp_thic(prog: &str) -> CompThic {
    let mut program = CompThic::new();

    for (state, instrs) in parse(prog).enumerate() {
        for (color, instr) in instrs.enumerate() {
            program.insert((state as State, color as Color), instr);
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
        None => "...".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn check_state(state: char) {
        assert_eq!(state, show_state(Some(read_state(state))));
    }

    #[test]
    fn test_state() {
        let states = ['A', 'B', 'C'];

        for state in states {
            check_state(state);
        }
    }

    fn check_slot(slot: &str) {
        assert_eq!(slot, show_slot(read_slot(slot)));
    }

    #[test]
    fn test_slot() {
        let slots = ["A0", "A1", "A2", "B0", "B1", "B2", "C0", "C1", "C2"];

        for slot in slots {
            check_slot(slot);
        }
    }

    fn check_instr(instr: &str) {
        assert_eq!(instr, show_instr(read_instr(instr)));
    }

    #[test]
    fn test_instr() {
        let instrs = ["1RB", "2LC"];

        for instr in instrs {
            check_instr(instr);
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

    fn check_init(states: usize, colors: usize, expected: &str) {
        assert_eq!(init_prog(states, colors), expected);
    }

    #[test]
    fn test_init() {
        check_init(2, 3, "1RB ... ...  ... ... ...");
        check_init(3, 2, "1RB ...  ... ...  ... ...");
        check_init(2, 4, "1RB ... ... ...  ... ... ... ...");
        check_init(4, 2, "1RB ...  ... ...  ... ...  ... ...");
    }
}
