use core::cmp::{max, min};

use rayon::prelude::*;

use crate::{
    instrs::{
        Color, GetInstr as _, Instr, Params, Parse, Prog, Shift, Slot,
        State,
    },
    tape::{MachineTape as _, MedTape as Tape},
    Goal,
};

pub type Step = usize;

type Slots = u8;

/**************************************/

const SHIFTS: [Shift; 2] = [false, true];

type Instrs = Vec<Instr>;
type InstrTable = Vec<Vec<Instrs>>;

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
                for shift in SHIFTS {
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

struct TreeCore<'h> {
    prog: Prog,
    sim_lim: Step,
    remaining_slots: Slots,
    harvester: &'h dyn Fn(&Prog),
}

impl<'h> TreeCore<'h> {
    fn init(
        params @ (states, colors): Params,
        halt: bool,
        sim_lim: Step,
        harvester: &'h dyn Fn(&Prog),
    ) -> Self {
        let prog = Prog::init_stepped(params);

        let remaining_slots =
            ((states * colors) as Slots) - Slots::from(halt) - 2;

        Self {
            prog,
            sim_lim,
            remaining_slots,
            harvester,
        }
    }

    const fn final_slot(&self) -> bool {
        self.remaining_slots == 0
    }

    fn run(&self, config: &mut Config) -> RunResult {
        config.run(&self.prog, self.sim_lim)
    }

    fn incomplete(&self) -> bool {
        self.prog.states_unreached() || self.prog.colors_unreached()
    }

    fn harvest(&self) {
        (self.harvester)(&self.prog);
    }

    fn insert(&mut self, slot: &Slot, instr: &Instr) {
        self.remaining_slots -= 1;

        self.prog.instrs.insert(*slot, *instr);
    }

    fn remove(&mut self, slot: &Slot) {
        self.prog.instrs.remove(slot);

        self.remaining_slots += 1;
    }
}

/**************************************/

trait Tree<'h> {
    fn core(&self) -> &TreeCore<'_>;

    fn final_slot(&self) -> bool {
        self.core().final_slot()
    }

    fn run(&self, config: &mut Config) -> RunResult {
        self.core().run(config)
    }

    fn incomplete(&self) -> bool {
        self.core().incomplete()
    }

    fn harvest(&self) {
        self.core().harvest();
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

impl<'h, T: Tree<'h>> Parse for T {
    fn read(_: &str) -> Self {
        unreachable!()
    }

    fn show(&self) -> String {
        self.core().prog.show()
    }
}

/**************************************/

struct BasicTree<'h> {
    core: TreeCore<'h>,

    avail_params: Vec<Params>,
    instr_table: &'h InstrTable,
}

impl<'h> BasicTree<'h> {
    fn init(
        params @ (states, colors): Params,
        halt: bool,
        sim_lim: Step,
        harvester: &'h dyn Fn(&Prog),
        instr_table: &'h InstrTable,
    ) -> Self {
        let core = TreeCore::init(params, halt, sim_lim, harvester);

        let init_avail = (min(3, states), min(3, colors));

        let avail_params = vec![init_avail];

        Self {
            core,
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

        let prog = &self.core.prog;

        if avail_states < prog.states
            && 1 + max(slot_state, instr_state) == avail_states
        {
            avail_states += 1;
        }

        if avail_colors < prog.colors
            && 1 + max(slot_color, instr_color) == avail_colors
        {
            avail_colors += 1;
        }

        self.avail_params.push((avail_states, avail_colors));
    }
}

impl<'h> Tree<'h> for BasicTree<'h> {
    fn core(&self) -> &TreeCore<'_> {
        &self.core
    }

    fn insert(&mut self, slot: &Slot, instr: &Instr) {
        self.core.insert(slot, instr);

        self.update_avail(slot, instr);
    }

    fn remove(&mut self, slot: &Slot) {
        self.core.remove(slot);

        self.avail_params.pop();
    }

    fn avail_instrs(&self) -> &'h [Instr] {
        let (avail_states, avail_colors) = self.avail_params();

        &self.instr_table[avail_states as usize][avail_colors as usize]
    }
}

/**************************************/

pub fn build_tree(
    params @ (states, colors): Params,
    goal: Option<Goal>,
    sim_lim: Step,
    harvester: &(impl Fn(&Prog) + Sync),
) {
    let init_states = min(3, states);
    let init_colors = min(3, colors);

    let halt = goal.is_some_and(|goal| goal.is_halt());

    let instr_table =
        make_instr_table(states as usize, colors as usize);

    let mut init_instrs =
        instr_table[init_states as usize][init_colors as usize].clone();

    init_instrs.retain(|instr| !matches!(instr, (_, true, 0 | 1)));

    if states == 2 && goal.is_some_and(|goal| goal.is_spinout()) {
        init_instrs.retain(|instr| matches!(instr, (_, _, 1)));
    }

    init_instrs.par_iter().for_each(|&next_instr| {
        let mut prog = BasicTree::init(
            params,
            halt,
            sim_lim,
            harvester,
            &instr_table,
        );

        prog.with_instr(&(1, 0), &next_instr, |prog| {
            prog.branch(Config::init_stepped());
        });
    });
}
