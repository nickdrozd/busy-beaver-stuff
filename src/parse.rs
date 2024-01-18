use pyo3::prelude::*;
use std::collections::HashMap;

use crate::instrs::{Color, Instr, Prog, Shift, Slot, State};

const HALT: char = '_';
const UNDF: char = '.';

const LEFT: char = 'L';
const RIGHT: char = 'R';

#[pyfunction]
pub fn parse(program: &str) -> Vec<Vec<Option<Instr>>> {
    program
        .trim()
        .split("  ")
        .map(|instrs| instrs.split(' ').map(read_instr).collect())
        .collect()
}

#[pyfunction]
pub fn tcompile(program: &str) -> Prog {
    let mut prog = HashMap::new();

    for (state, instrs) in parse(program).iter().enumerate() {
        for (color, instr) in instrs.iter().enumerate() {
            prog.insert((state as State, color as Color), *instr);
        }
    }

    prog
}

fn read_color(color: char) -> Color {
    color.to_digit(10).unwrap().into()
}

const fn read_shift(shift: char) -> Shift {
    shift == RIGHT
}

#[pyfunction]
pub const fn show_state(state: Option<State>) -> char {
    match state {
        Some(-1) => HALT,
        None => UNDF,
        Some(s) => (s as u8 + 65) as char,
    }
}

#[pyfunction]
pub fn read_state(state: char) -> State {
    if state == HALT {
        -1
    } else {
        State::from(state as u8 - 65)
    }
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
