use core::cmp::{max, min};

use rayon::prelude::*;

use crate::{
    instrs::{
        Color, GetInstr as _, Instr, Params, Parse, Prog, Slot, State,
    },
    tape::{MachineTape as _, MedTape as Tape},
    Goal,
};

pub type Step = usize;

type Slots = u8;

/**************************************/

const SHIFTS: [bool; 2] = [false, true];

type InstrTable = Vec<Vec<Vec<Instr>>>;

fn make_instr_table(
    max_states: usize,
    max_colors: usize,
) -> InstrTable {
    let mut table = vec![vec![vec![]; 1 + max_colors]; 1 + max_states];

    #[expect(clippy::needless_range_loop)]
    for states in 2..=max_states {
        for colors in 2..=max_colors {
            let mut instrs = Vec::with_capacity(colors * 2 * states);

            for color in 0..colors {
                for &shift in &SHIFTS {
                    for state in 0..states {
                        instrs.push((
                            color as Color,
                            shift,
                            state as State,
                        ));
                    }
                }
            }

            table[states][colors] = instrs;
        }
    }

    table
}

/**************************************/

enum RunResult {
    Limit,
    Blank,
    Spinout,
    Undefined(Slot),
}

use RunResult::*;

#[derive(Clone)]
struct Config {
    state: State,
    tape: Tape,
}

impl Config {
    fn init_stepped() -> Self {
        Self {
            state: 1,
            tape: Tape::init_stepped(),
        }
    }

    fn run(&mut self, comp: &Prog, sim_lim: Step) -> RunResult {
        for _ in 0..sim_lim {
            let slot = (self.state, self.tape.scan);

            let Some((color, shift, state)) = comp.get_instr(&slot)
            else {
                return Undefined(slot);
            };

            let same = self.state == state;

            if same && self.tape.at_edge(shift) {
                return Spinout;
            }

            self.tape.step(shift, color, same);

            if self.tape.blank() {
                return Blank;
            }

            self.state = state;
        }

        Limit
    }
}

/**************************************/

trait Brancher<'h> {
    fn prog(&self) -> &Prog;
    fn sim_lim(&self) -> Step;
    fn remaining_slots(&self) -> Slots;
    fn harvester(&self) -> &'h dyn Fn(&Prog);

    fn final_slot(&self) -> bool {
        self.remaining_slots() == 0
    }

    fn run(&self, config: &mut Config) -> RunResult {
        config.run(self.prog(), self.sim_lim())
    }

    fn incomplete(&self) -> bool {
        self.prog().states_unreached() || self.prog().colors_unreached()
    }

    fn harvest(&self) {
        (self.harvester())(self.prog());
    }

    fn avail_instrs(&self) -> &'h [Instr];

    fn insert(&mut self, slot: &Slot, instr: &Instr);
    fn remove(&mut self, slot: &Slot);

    fn with_instr(
        &mut self,
        slot: &Slot,
        instr: &Instr,
        body: impl FnOnce(&mut Self),
    ) {
        self.insert(slot, instr);

        body(self);

        self.remove(slot);
    }

    fn branch(&mut self, mut config: Config) {
        let slot @ (slot_state, _) = match self.run(&mut config) {
            Undefined(slot) => slot,
            Blank | Spinout => return,
            Limit => {
                #[expect(clippy::if_not_else)]
                if !self.incomplete() {
                    // println!("{}", self.show());
                    self.harvest();
                } else {
                    // println!("{}", self.show());
                }

                return;
            },
        };

        let instrs = self.avail_instrs();

        if self.final_slot() {
            for next_instr in instrs {
                self.with_instr(&slot, next_instr, |prog| {
                    prog.harvest();
                });
            }

            return;
        }

        config.state = slot_state;

        let (last_instr, instrs) = instrs.split_last().unwrap();

        for next_instr in instrs {
            self.with_instr(&slot, next_instr, |prog| {
                prog.branch(config.clone());
            });
        }

        self.with_instr(&slot, last_instr, |prog| {
            prog.branch(config);
        });
    }
}

impl<'h, T: Brancher<'h>> Parse for T {
    fn read(_: &str) -> Self {
        unreachable!()
    }

    fn show(&self) -> String {
        self.prog().show()
    }
}

/**************************************/

struct TreeProg<'h> {
    prog: Prog,
    sim_lim: Step,
    remaining_slots: Slots,
    harvester: &'h dyn Fn(&Prog),

    avail_params: Vec<Params>,
    instr_table: &'h InstrTable,
}

impl<'h> TreeProg<'h> {
    fn init(
        params @ (states, colors): Params,
        goal: Option<Goal>,
        sim_lim: Step,
        harvester: &'h dyn Fn(&Prog),
        instr_table: &'h InstrTable,
    ) -> Self {
        let prog = Prog::init_stepped(params);

        let halt = goal.is_some_and(|goal| goal.is_halt());

        let remaining_slots =
            ((states * colors) as Slots) - Slots::from(halt) - 2;

        let init_avail = (min(3, states), min(3, colors));

        let avail_params = vec![init_avail];

        Self {
            prog,
            sim_lim,
            remaining_slots,
            harvester,
            avail_params,
            instr_table,
        }
    }

    fn avail_params(&self) -> Params {
        *self.avail_params.last().unwrap()
    }

    fn update_avail(
        &mut self,
        (slot_state, slot_color): &Slot,
        (instr_color, _, instr_state): &Instr,
    ) {
        let (mut avail_states, mut avail_colors) = self.avail_params();

        if avail_states < self.prog.states
            && 1 + max(slot_state, instr_state) == avail_states
        {
            avail_states += 1;
        }

        if avail_colors < self.prog.colors
            && 1 + max(slot_color, instr_color) == avail_colors
        {
            avail_colors += 1;
        }

        self.avail_params.push((avail_states, avail_colors));
    }
}

impl<'h> Brancher<'h> for TreeProg<'h> {
    fn prog(&self) -> &Prog {
        &self.prog
    }

    fn sim_lim(&self) -> Step {
        self.sim_lim
    }

    fn remaining_slots(&self) -> Slots {
        self.remaining_slots
    }

    fn harvester(&self) -> &'h dyn Fn(&Prog) {
        self.harvester
    }

    fn insert(&mut self, slot: &Slot, instr: &Instr) {
        self.remaining_slots -= 1;

        self.update_avail(slot, instr);

        self.prog.instrs.insert(*slot, *instr);
    }

    fn remove(&mut self, slot: &Slot) {
        self.prog.instrs.remove(slot);

        self.avail_params.pop();

        self.remaining_slots += 1;
    }

    fn avail_instrs(&self) -> &'h [Instr] {
        let (avail_states, avail_colors) = self.avail_params();

        &self.instr_table[avail_states as usize][avail_colors as usize]
    }
}

/**************************************/

pub fn build_tree(
    params @ (states, colors): Params,
    goal: Option<u8>,
    sim_lim: Step,
    harvester: &(impl Fn(&Prog) + Sync),
) {
    let goal = goal.map(Goal::from);

    let init_states = min(3, states);
    let init_colors = min(3, colors);

    let instr_table =
        make_instr_table(states as usize, colors as usize);

    let mut init_instrs =
        instr_table[init_states as usize][init_colors as usize].clone();

    init_instrs.retain(|instr| !matches!(instr, (_, true, 0 | 1)));

    if states == 2 && goal.is_some_and(|goal| goal.is_spinout()) {
        init_instrs.retain(|instr| matches!(instr, (_, _, 1)));
    }

    let init_slot = (1, 0);

    init_instrs.par_iter().for_each(|&next_instr| {
        let mut prog = TreeProg::init(
            params,
            goal,
            sim_lim,
            harvester,
            &instr_table,
        );

        prog.with_instr(&init_slot, &next_instr, |prog| {
            prog.branch(Config::init_stepped());
        });
    });
}
