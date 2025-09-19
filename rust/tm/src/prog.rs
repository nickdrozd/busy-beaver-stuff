use std::collections::BTreeSet as Set;

use crate::{Color, Instr, Parse, Shift, Slot, State};

pub type Params = (State, Color);

/**************************************/

struct Instrs {
    table: Vec<Vec<Option<Instr>>>,
}

impl Instrs {
    fn new((states, colors): Params) -> Self {
        Self {
            table: vec![vec![None; colors as usize]; states as usize],
        }
    }

    fn get(&self, &(state, color): &Slot) -> Option<&Instr> {
        self.table[state as usize][color as usize].as_ref()
    }

    fn insert(&mut self, (state, color): Slot, instr: Instr) {
        self.table[state as usize][color as usize] = Some(instr);
    }

    fn remove(&mut self, &(state, color): &Slot) {
        self.table[state as usize][color as usize] = None;
    }

    #[expect(clippy::cast_possible_truncation)]
    fn iter(&self) -> impl Iterator<Item = (Slot, &Instr)> + '_ {
        self.table.iter().enumerate().flat_map(|(s, row)| {
            row.iter().enumerate().filter_map(move |(c, opt)| {
                opt.as_ref().map(|instr| ((s as u8, c as u8), instr))
            })
        })
    }

    fn values(&self) -> impl Iterator<Item = &Instr> + '_ {
        self.table
            .iter()
            .flat_map(|row| row.iter().filter_map(|opt| opt.as_ref()))
    }
}

/**************************************/

pub struct Prog {
    instrs: Instrs,

    states: State,
    colors: Color,
}

impl Prog {
    const fn new(instrs: Instrs, (states, colors): Params) -> Self {
        Self {
            instrs,
            states,
            colors,
        }
    }

    pub const fn states(&self) -> State {
        self.states
    }

    pub const fn params(&self) -> Params {
        (self.states, self.colors)
    }

    pub fn init_stepped(params: Params) -> Self {
        let mut instrs = Instrs::new(params);

        instrs.insert((0, 0), (1, true, 1));

        Self::new(instrs, params)
    }

    pub fn print(&self) {
        println!("{}", self.show());
    }

    pub fn get(&self, slot: &Slot) -> Option<&Instr> {
        self.instrs.get(slot)
    }

    pub fn insert(&mut self, slot: &Slot, instr: &Instr) {
        self.instrs.insert(*slot, *instr);
    }

    pub fn remove(&mut self, slot: &Slot) {
        self.instrs.remove(slot);
    }

    pub fn iter(&self) -> impl Iterator<Item = (Slot, &Instr)> {
        self.instrs.iter()
    }

    pub fn values(&self) -> impl Iterator<Item = &Instr> {
        self.instrs.values()
    }

    pub fn halt_slots(&self) -> Set<Slot> {
        let mut slots = Set::new();

        for (state, colors) in self.instrs.table.iter().enumerate() {
            for (color, entry) in colors.iter().enumerate() {
                if entry.is_none() {
                    #[expect(clippy::cast_possible_truncation)]
                    slots.insert((state as State, color as Color));
                }
            }
        }

        slots
    }

    pub fn erase_slots(&self) -> Set<Slot> {
        self.instrs
            .iter()
            .filter_map(|(slot @ (_, co), &(pr, _, _))| {
                (co != 0 && pr == 0).then_some(slot)
            })
            .collect()
    }

    pub fn zr_shifts(&self) -> Set<(State, Shift)> {
        self.instrs
            .iter()
            .filter_map(|(slot, &(_, sh, st))| {
                (slot == (st, 0)).then_some((st, sh))
            })
            .collect()
    }

    pub fn states_unreached(&self) -> bool {
        self.states > 2
            && self
                .instrs
                .values()
                .all(|(_, _, state)| 1 + state < self.states)
    }

    pub fn colors_unreached(&self) -> bool {
        self.colors > 2
            && self
                .instrs
                .values()
                .all(|(color, _, _)| 1 + color < self.colors)
    }

    pub fn incomplete(&self) -> bool {
        self.states_unreached() || self.colors_unreached()
    }

    pub fn reaches_from_both_sides(&self, (st, co): Slot) -> bool {
        let mut side = None;

        for &(pr, sh, tr) in self.values() {
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
    #[expect(clippy::cast_possible_truncation)]
    fn read(prog: &str) -> Self {
        let rows: Vec<Vec<Option<Instr>>> = prog
            .trim()
            .split("  ")
            .map(|row| {
                row.split(' ').map(Option::<Instr>::read).collect()
            })
            .collect();

        let states: State = rows.len() as State;

        let colors: Color = rows.first().map_or(0, Vec::len) as Color;

        let mut instrs = Instrs::new((states, colors));

        for (s, row) in rows.into_iter().enumerate() {
            for (c, cell) in row.into_iter().enumerate() {
                if let Some(instr) = cell {
                    instrs.insert((s as State, c as Color), instr);
                }
            }
        }

        Self {
            instrs,
            states,
            colors,
        }
    }

    fn show(&self) -> String {
        (0..self.states)
            .map(|state| {
                (0..self.colors)
                    .map(|color| {
                        self.instrs.get(&(state, color)).show()
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect::<Vec<_>>()
            .join("  ")
    }
}

/**************************************/
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
        Set::from([(0, 1), (1, 1), (2, 1)])
    );

    assert_eq!(
        Prog::read("1RB 0LA ...  2LA ... ...").halt_slots(),
        Set::from([(0, 2), (1, 1), (1, 2)])
    );
}
