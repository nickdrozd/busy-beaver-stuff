use std::collections::BTreeSet as Set;

use crate::{Color, Instr, Parse, Shift, Slot, State};

/**************************************/

type Table = Vec<Vec<Option<Instr>>>;

#[expect(clippy::partial_pub_fields)]
pub struct Prog {
    table: Table,

    pub states: State,
    pub colors: Color,

    pub dimension: u8,
}

impl From<Table> for Prog {
    #[expect(clippy::cast_possible_truncation)]
    fn from(table: Table) -> Self {
        let states = table.len() as State;
        let colors = table.first().map_or(0, Vec::len) as Color;

        Self {
            table,
            states,
            colors,
            dimension: states * colors,
        }
    }
}

impl Prog {
    fn new(states: State, colors: Color) -> Self {
        Self {
            table: vec![vec![None; colors as usize]; states as usize],
            states,
            colors,
            dimension: states * colors,
        }
    }

    pub fn init_norm(states: State, colors: Color) -> Self {
        let mut prog = Self::new(states, colors);

        prog.insert(&(0, 0), &(1, true, 1));

        prog
    }

    pub fn print(&self) {
        println!("{}", self.show());
    }

    pub fn get(&self, &(state, color): &Slot) -> Option<&Instr> {
        self.table[state as usize][color as usize].as_ref()
    }

    pub fn insert(&mut self, &(state, color): &Slot, instr: &Instr) {
        self.table[state as usize][color as usize] = Some(*instr);
    }

    pub fn remove(&mut self, &(state, color): &Slot) {
        self.table[state as usize][color as usize] = None;
    }

    #[expect(clippy::cast_possible_truncation)]
    pub fn iter(&self) -> impl Iterator<Item = (Slot, &Instr)> {
        self.table.iter().enumerate().flat_map(|(state, colors)| {
            colors.iter().enumerate().filter_map(
                move |(color, maybe)| {
                    maybe.as_ref().map(|instr| {
                        ((state as u8, color as u8), instr)
                    })
                },
            )
        })
    }

    pub fn instrs(&self) -> impl Iterator<Item = &Instr> {
        self.table
            .iter()
            .flat_map(|colors| colors.iter().filter_map(Option::as_ref))
    }

    #[expect(clippy::cast_possible_truncation)]
    pub fn halt_slots(&self) -> Set<Slot> {
        let mut slots = Set::new();

        for (state, colors) in self.table.iter().enumerate() {
            for (color, entry) in colors.iter().enumerate() {
                if entry.is_some() {
                    continue;
                }

                if color != 0
                    && !self.reaches_both_sides(
                        state as State,
                        color as Color,
                    )
                {
                    continue;
                }

                slots.insert((state as State, color as Color));
            }
        }

        slots
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

    pub fn states_unreached(&self) -> bool {
        self.states > 2
            && self.instrs().all(|(_, _, tr)| 1 + tr < self.states)
    }

    pub fn colors_unreached(&self) -> bool {
        self.colors > 2
            && self.instrs().all(|(pr, _, _)| 1 + pr < self.colors)
    }

    pub fn incomplete(&self) -> bool {
        self.states_unreached() || self.colors_unreached()
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

impl Parse for Prog {
    fn read(prog: &str) -> Self {
        prog.trim()
            .split("  ")
            .map(|colors| colors.split(' ').map(Parse::read).collect())
            .collect::<Vec<_>>()
            .into()
    }

    fn show(&self) -> String {
        (0..self.states)
            .map(|state| {
                (0..self.colors)
                    .map(|color| self.get(&(state, color)).show())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect::<Vec<_>>()
            .join("  ")
    }
}

/**************************************/

#[cfg(test)]
use crate::Params;

#[cfg(test)]
impl Prog {
    pub const fn params(&self) -> Params {
        (self.states, self.colors)
    }
}

#[test]
fn test_prog() {
    let progs = [
        "1RB ...  ... ...",
        "1RB ...  1LA ...",
        "1RB 1LB  1LA ...",
        "1RB ... ...  2LB ... ...",
        "1RB ... ...  2LB 1RB 1LB",
        "1RB 0RB ...  2LA ... 0LB",
        "1RB ...  1LB 0RC  1LC 1LA",
        "1RB 2LB 2LB ...  1LA 2RB 2LA ...",
        "1RB 0LB  0LC 0RA  1LA 1LC  ... ...",
        "1RB 1RC  1LC 1RD  1RA 1LD  0RD 0LB",
        "1RB 1LC  1RC 1RB  1RD 0LE  1LA 1LD  ... 0LA",
        "1RB 1RC  0LC 1RD  1LB 1LE  1RD 0RA  1LA 0LE",
        "1RB 2RB 3RB 4RB 5LA 4RA  0LA 1RB 5RA ... ... 1LB",
        "1RB ...  1RC ...  1LC 1LD  1RE 1LF  1RC 1RE  0RC 0RF",
    ];

    for prog in progs {
        assert_eq!(Prog::read(prog).show(), prog);
    }
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
        assert_eq!(Prog::read(prog).params(), params);
    }
}

#[test]
fn test_halt_slots() {
    assert_eq!(
        Prog::read("1RB ...  0RC ...  0LA ...").halt_slots(),
        Set::from([(0, 1)])
    );

    assert_eq!(
        Prog::read("1RB 0LA ...  2LA ... ...").halt_slots(),
        Set::from([(1, 2)])
    );
}
