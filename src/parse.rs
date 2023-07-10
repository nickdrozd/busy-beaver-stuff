use pyo3::prelude::*;
use std::collections::HashMap;

use crate::instrs::{Color, Instr, Prog, State};

const HALT: char = '_';
const UNDF: char = '.';

const LEFT: char = 'L';
const RIGHT: char = 'R';

#[pyfunction]
pub fn parse(program: &str) -> Vec<Vec<Option<Instr>>> {
    program
        .trim()
        .split("  ")
        .map(|instrs| {
            instrs
                .split(' ')
                .map(|instr| {
                    if instr.contains(UNDF) {
                        None
                    } else {
                        Some((
                            Color::from(instr.chars().next().unwrap().to_digit(10).unwrap()),
                            instr.chars().nth(1).unwrap() == RIGHT,
                            read_state(instr.chars().nth(2).unwrap()),
                        ))
                    }
                })
                .collect()
        })
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
