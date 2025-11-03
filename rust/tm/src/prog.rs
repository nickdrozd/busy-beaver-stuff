use std::collections::BTreeSet as Set;

use crate::{Color, Colors, Instr, Parse, Shift, Slot, State, States};

/**************************************/

#[expect(clippy::partial_pub_fields)]
pub struct Prog {
    table: Vec<Option<Instr>>,

    pub states: States,
    pub colors: Colors,

    pub dimension: usize,
}

impl Prog {
    fn new(states: States, colors: Colors) -> Self {
        Self {
            table: vec![None; states * colors],
            states,
            colors,
            dimension: states * colors,
        }
    }

    pub fn init_norm(states: States, colors: Colors) -> Self {
        let mut prog = Self::new(states, colors);

        prog.insert(&(0, 0), &(1, true, 1));

        prog
    }

    pub fn print(&self) {
        println!("{}", self.show());
    }

    const fn index(&self, state: State, color: Color) -> usize {
        (state as usize) * self.colors + (color as usize)
    }

    pub fn get(&self, &(state, color): &Slot) -> Option<&Instr> {
        self.table[self.index(state, color)].as_ref()
    }

    pub fn insert(&mut self, &(state, color): &Slot, instr: &Instr) {
        let index = self.index(state, color);

        self.table[index] = Some(*instr);
    }

    pub fn remove(&mut self, &(state, color): &Slot) {
        let index = self.index(state, color);

        self.table[index] = None;
    }

    #[expect(clippy::cast_possible_truncation)]
    pub fn iter(&self) -> impl Iterator<Item = (Slot, &Instr)> {
        self.table.chunks_exact(self.colors).enumerate().flat_map(
            move |(st, colors)| {
                colors.iter().enumerate().filter_map(
                    move |(co, maybe)| {
                        maybe
                            .as_ref()
                            .map(|instr| ((st as u8, co as u8), instr))
                    },
                )
            },
        )
    }

    pub fn instrs(&self) -> impl Iterator<Item = &Instr> {
        self.table.iter().filter_map(Option::as_ref)
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
        let mut slots = Set::new();

        let (states, colors) = self.max_reached();

        for state in 0..=states {
            for color in 0..=colors {
                let slot = (state, color);

                if self.get(&slot).is_some() {
                    continue;
                }

                if color != 0 && !self.reaches_both_sides(state, color)
                {
                    continue;
                }

                slots.insert(slot);
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
        let instrs: Vec<Vec<Option<Instr>>> = prog
            .trim()
            .split("  ")
            .map(|colors| colors.split(' ').map(Parse::read).collect())
            .collect();

        let states = instrs.len();
        let colors = instrs.first().unwrap().len();

        let mut table = Vec::with_capacity(states * colors);

        for entries in instrs {
            table.extend(entries);
        }

        Self {
            table,
            states,
            colors,
            dimension: states * colors,
        }
    }

    fn show(&self) -> String {
        let (states, colors) = self.max_reached();

        (0..=states)
            .map(|state| {
                (0..=colors)
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

#[cfg(test)]
const SPARSE_SHOW: &[(&str, &str)] = &[
    (
        "1RB 2LB 2LB  1LA 2RB 2LA",
        "1RB 2LB 2LB ...  1LA 2RB 2LA ...",
    ),
    (
        "1RB 0LB  0LC 0RA  1LA 1LC",
        "1RB 0LB  0LC 0RA  1LA 1LC  ... ...",
    ),
    (
        "1RB 1LD  0RC 0LA  0LC 1LA  0LA 0LA",
        "1RB 1LD ... ... ... ... ... ...  0RC 0LA ... ... ... ... ... ...  0LC 1LA ... ... ... ... ... ...  0LA 0LA ... ... ... ... ... ...  ... ... ... ... ... ... ... ...  ... ... ... ... ... ... ... ...  ... ... ... ... ... ... ... ...  ... ... ... ... ... ... ... ...",
    ),
    (
        "1RB ... ... ...  2LB 0LC ... ...  0LD ... ... ...  0LE ... ... ...  0LF ... ... ...  1LG ... ... ...  3RC ... ... ...",
        "1RB ... ... ... ... ... ... ...  2LB 0LC ... ... ... ... ... ...  0LD ... ... ... ... ... ... ...  0LE ... ... ... ... ... ... ...  0LF ... ... ... ... ... ... ...  1LG ... ... ... ... ... ... ...  3RC ... ... ... ... ... ... ...  ... ... ... ... ... ... ... ...",
    ),
];

#[test]
fn test_sparse_show() {
    for &(show, prog) in SPARSE_SHOW {
        assert_eq!(Prog::read(prog).show(), show);
    }
}

#[test]
fn test_halt_slots() {
    assert_eq!(
        Prog::read("1RB ...  0RC ...  0LA ...").halt_slots(),
        Set::from([(0, 1)]),
    );

    assert_eq!(
        Prog::read("1RB 0LA ...  2LA ... ...").halt_slots(),
        Set::from([(1, 2)]),
    );

    assert_eq!(
        Prog::read("1RB 1LD ... ... ... ... ... ...  0RC 0LA ... ... ... ... ... ...  0LC 1LA ... ... ... ... ... ...  0LA 0LA ... ... ... ... ... ...  ... ... ... ... ... ... ... ...  ... ... ... ... ... ... ... ...  ... ... ... ... ... ... ... ...  ... ... ... ... ... ... ... ...").halt_slots(),
        Set::from([]),
    );

    assert_eq!(
        Prog::read("1RB ... ... ... ... ... ... ...  2LB 0LC ... ... ... ... ... ...  0LD ... ... ... ... ... ... ...  0LE ... ... ... ... ... ... ...  0LF ... ... ... ... ... ... ...  1LG ... ... ... ... ... ... ...  3RC ... ... ... ... ... ... ...  ... ... ... ... ... ... ... ...").halt_slots(),
        Set::from([(0, 1), (1, 2), (1, 3), (2, 1), (2, 2), (2, 3), (3, 1), (3, 3), (4, 1), (4, 3), (5, 1), (5, 3), (6, 1), (6, 3)]),
    );
}
