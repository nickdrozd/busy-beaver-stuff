use core::fmt::{self, Display};

use std::collections::BTreeSet as Set;

use crate::{Color, Instr, Shift, Slot, State, instrs::Parse};

/**************************************/

pub struct Prog<const states: usize, const colors: usize> {
    table: [[Option<Instr>; colors]; states],
}

impl<const s: usize, const c: usize> From<&str> for Prog<s, c> {
    fn from(prog: &str) -> Self {
        let mut out = Self::new();

        for (i, colors) in prog.trim().split("  ").enumerate().take(s) {
            for (j, instr) in colors.split(' ').enumerate().take(c) {
                if let Some(parsed) = Parse::read(instr) {
                    #[expect(clippy::cast_possible_truncation)]
                    let slot = (i as State, j as Color);
                    out.insert(&slot, &parsed);
                }
            }
        }

        out
    }
}

impl<const s: usize, const c: usize> Display for Prog<s, c> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (states, colors) = self.max_reached();

        for state in 0..=states {
            if 0 < state {
                write!(f, "  ")?;
            }

            for color in 0..=colors {
                if 0 < color {
                    write!(f, " ")?;
                }

                write!(f, "{}", self.get(&(state, color)).show())?;
            }
        }

        Ok(())
    }
}

impl<const states: usize, const colors: usize> Prog<states, colors> {
    const fn new() -> Self {
        Self {
            table: [[None; colors]; states],
        }
    }

    pub const fn init_norm() -> Self {
        let mut prog = Self::new();

        prog.insert(&(0, 0), &(1, true, 1));

        prog
    }

    pub fn print(&self) {
        println!("{self}");
    }

    pub fn get(&self, &(state, color): &Slot) -> Option<&Instr> {
        unsafe {
            self.table
                .get_unchecked(state as usize)
                .get_unchecked(color as usize)
        }
        .as_ref()
    }

    pub const fn insert(
        &mut self,
        &(state, color): &Slot,
        instr: &Instr,
    ) {
        self.table[state as usize][color as usize] = Some(*instr);
    }

    pub const fn remove(&mut self, &(state, color): &Slot) {
        self.table[state as usize][color as usize] = None;
    }

    #[expect(clippy::cast_possible_truncation)]
    pub fn iter(&self) -> impl Iterator<Item = (Slot, &Instr)> {
        self.table.iter().enumerate().flat_map(|(state, cos)| {
            cos.iter().enumerate().filter_map(move |(color, maybe)| {
                maybe
                    .as_ref()
                    .map(|instr| ((state as u8, color as u8), instr))
            })
        })
    }

    pub fn instrs(&self) -> impl Iterator<Item = &Instr> {
        self.table
            .iter()
            .flat_map(|cos| cos.iter().filter_map(Option::as_ref))
    }

    pub fn max_reached(&self) -> (State, Color) {
        let mut st = 1;
        let mut co = 1;

        for &(pr, _, tr) in self.instrs() {
            if co < pr {
                co = pr;
            }

            if st < tr {
                st = tr;
            }
        }

        (st, co)
    }

    pub fn halt_slots(&self) -> Set<Slot> {
        let (states_reached, colors_reached) = self.max_reached();

        (0..=states_reached)
            .flat_map(|state| {
                (0..=colors_reached).map(move |color| (state, color))
            })
            .filter(|slot @ &(state, color)| {
                self.get(slot).is_none()
                    && (color == 0
                        || self.reaches_both_sides(state, color))
            })
            .collect()
    }

    pub fn erase_slots(&self) -> Set<Slot> {
        self.iter()
            .filter_map(|(slot @ (_, co), &(pr, _, _))| {
                (co != 0 && pr == 0).then_some(slot)
            })
            .collect()
    }

    pub fn zr_shifts(&self) -> Set<(State, Shift)> {
        self.iter()
            .filter_map(|(slot, &(_, sh, st))| {
                (slot == (st, 0)).then_some((st, sh))
            })
            .collect()
    }

    pub fn reaches_both_sides(&self, st: State, co: Color) -> bool {
        let mut side = None;

        for &(pr, sh, tr) in self.instrs() {
            if pr != co && tr != st {
                continue;
            }

            let Some(sd) = side else {
                side = Some(sh);
                continue;
            };

            if sd != sh {
                return true;
            }
        }

        false
    }
}

/**************************************/

#[cfg(test)]
macro_rules! assert_show {
    ($prog:literal, ($s:literal, $c:literal), $show:literal) => {
        assert_eq!(Prog::<$s, $c>::from($prog).to_string(), $show);
    };
}

#[test]
fn test_sparse_show() {
    assert_show!(
        "1RB 2LB 2LB ...  1LA 2RB 2LA ...",
        (2, 4),
        "1RB 2LB 2LB  1LA 2RB 2LA"
    );

    assert_show!(
        "1RB 0LB  0LC 0RA  1LA 1LC  ... ...",
        (4, 2),
        "1RB 0LB  0LC 0RA  1LA 1LC"
    );

    assert_show!(
        "1RB 1LD ... ... ... ... ... ...  0RC 0LA ... ... ... ... ... ...  0LC 1LA ... ... ... ... ... ...  0LA 0LA ... ... ... ... ... ...  ... ... ... ... ... ... ... ...  ... ... ... ... ... ... ... ...  ... ... ... ... ... ... ... ...  ... ... ... ... ... ... ... ...",
        (8, 8),
        "1RB 1LD  0RC 0LA  0LC 1LA  0LA 0LA"
    );

    assert_show!(
        "1RB ... ... ... ... ... ... ...  2LB 0LC ... ... ... ... ... ...  0LD ... ... ... ... ... ... ...  0LE ... ... ... ... ... ... ...  0LF ... ... ... ... ... ... ...  1LG ... ... ... ... ... ... ...  3RC ... ... ... ... ... ... ...  ... ... ... ... ... ... ... ...",
        (8, 8),
        "1RB ... ... ...  2LB 0LC ... ...  0LD ... ... ...  0LE ... ... ...  0LF ... ... ...  1LG ... ... ...  3RC ... ... ..."
    );
}

#[test]
fn test_halt_slots() {
    assert_eq!(
        Prog::<3, 2>::from("1RB ...  0RC ...  0LA ...").halt_slots(),
        Set::from([(0, 1)]),
    );

    assert_eq!(
        Prog::<2, 3>::from("1RB 0LA ...  2LA ... ...").halt_slots(),
        Set::from([(1, 2)]),
    );

    assert_eq!(
        Prog::<8, 8>::from("1RB 1LD ... ... ... ... ... ...  0RC 0LA ... ... ... ... ... ...  0LC 1LA ... ... ... ... ... ...  0LA 0LA ... ... ... ... ... ...  ... ... ... ... ... ... ... ...  ... ... ... ... ... ... ... ...  ... ... ... ... ... ... ... ...  ... ... ... ... ... ... ... ...").halt_slots(),
        Set::from([]),
    );

    assert_eq!(
        Prog::<8, 8>::from("1RB ... ... ... ... ... ... ...  2LB 0LC ... ... ... ... ... ...  0LD ... ... ... ... ... ... ...  0LE ... ... ... ... ... ... ...  0LF ... ... ... ... ... ... ...  1LG ... ... ... ... ... ... ...  3RC ... ... ... ... ... ... ...  ... ... ... ... ... ... ... ...").halt_slots(),
        Set::from([(0, 1), (1, 2), (1, 3), (2, 1), (2, 2), (2, 3), (3, 1), (3, 3), (4, 1), (4, 3), (5, 1), (5, 3), (6, 1), (6, 3)]),
    );
}
