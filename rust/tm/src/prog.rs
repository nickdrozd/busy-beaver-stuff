use std::collections::BTreeSet as Set;

use crate::instrs::{
    Color, GetInstr, Instr, Instrs, Params, Parse, Shift, Slot, State,
};

/**************************************/

pub struct Prog {
    pub instrs: Instrs,

    pub states: State,
    pub colors: Color,
}

impl Prog {
    pub const fn new(instrs: Instrs, (states, colors): Params) -> Self {
        Self {
            instrs,
            states,
            colors,
        }
    }

    pub fn init_stepped(params: Params) -> Self {
        let instrs = Instrs::from([((0, 0), (1, true, 1))]);

        Self::new(instrs, params)
    }

    pub fn get(&self, slot: &Slot) -> Option<&Instr> {
        self.instrs.get(slot)
    }

    pub fn halt_slots(&self) -> Set<Slot> {
        let mut slots = Set::new();

        for state in 0..self.states {
            for color in 0..self.colors {
                let slot = (state, color);

                if !self.instrs.contains_key(&slot) {
                    slots.insert(slot);
                }
            }
        }

        slots
    }

    pub fn erase_slots(&self) -> Set<Slot> {
        self.instrs
            .iter()
            .filter_map(|(&slot @ (_, co), &(pr, _, _))| {
                (co != 0 && pr == 0).then_some(slot)
            })
            .collect()
    }

    pub fn zr_shifts(&self) -> Set<(State, Shift)> {
        self.instrs
            .iter()
            .filter_map(|(&slot, &(_, sh, st))| {
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

    pub fn incomplete(&self, halt: bool) -> bool {
        let (states, colors) = self.params();

        let dim = (states * colors) as usize;

        if self.instrs.len() < (if halt { dim - 1 } else { dim }) {
            return true;
        }

        let (used_states, used_colors): (Set<State>, Set<Color>) =
            self.instrs.values().map(|(pr, _, tr)| (tr, pr)).unzip();

        (colors == 2 && !used_colors.contains(&0))
            || (0..states).any(|state| !used_states.contains(&state))
            || (1..colors).any(|color| !used_colors.contains(&color))
    }
}

/**************************************/

impl GetInstr for Prog {
    fn get_instr(&self, slot: &Slot) -> Option<Instr> {
        self.instrs.get(slot).copied()
    }

    fn params(&self) -> Params {
        (self.states, self.colors)
    }
}

impl Parse for Prog {
    fn read(prog: &str) -> Self {
        let split = prog
            .trim()
            .split("  ")
            .map(|instrs| {
                instrs
                    .split(' ')
                    .map(Option::<Instr>::read)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        let params = (split.len() as State, split[0].len() as Color);

        let instrs = split
            .iter()
            .enumerate()
            .flat_map(|(state, instrs)| {
                instrs.iter().enumerate().filter_map(
                    move |(color, instr)| {
                        instr.map(|instr| {
                            ((state as State, color as Color), instr)
                        })
                    },
                )
            })
            .collect();

        Self::new(instrs, params)
    }

    fn show(&self) -> String {
        (0..self.states)
            .map(|state| {
                (0..self.colors)
                    .map(|color| self.get_instr(&(state, color)).show())
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
