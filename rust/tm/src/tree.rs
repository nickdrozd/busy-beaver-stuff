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

type BlankInstrTable = [InstrTable; 2];

fn make_blank_table(
    max_states: usize,
    max_colors: usize,
) -> BlankInstrTable {
    let table = make_instr_table(max_states, max_colors);

    let mut partial = table.clone();

    #[expect(clippy::needless_range_loop)]
    for states in 2..=max_states {
        for colors in 2..=max_colors {
            partial[states][colors].retain(|&(co, _, _)| co == 0);
        }
    }

    [table, partial]
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
    avail_params: Vec<Params>,
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

        let init_avail = (min(3, states), min(3, colors));

        let avail_params = vec![init_avail];

        let remaining_slots =
            ((states * colors) as Slots) - Slots::from(halt) - 2;

        Self {
            prog,
            sim_lim,
            avail_params,
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

        self.update_avail(slot, instr);
    }

    fn remove(&mut self, slot: &Slot) {
        self.avail_params.pop();

        self.prog.instrs.remove(slot);

        self.remaining_slots += 1;
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

        let prog = &self.prog;

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

    fn avail_instrs(&self, slot: &Slot) -> &'h [Instr];

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

        let instrs = self.avail_instrs(&slot);

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

    instr_table: &'h InstrTable,
}

impl<'h> BasicTree<'h> {
    fn init(
        params: Params,
        halt: bool,
        sim_lim: Step,
        harvester: &'h dyn Fn(&Prog),
        instr_table: &'h InstrTable,
    ) -> Self {
        let core = TreeCore::init(params, halt, sim_lim, harvester);

        Self { core, instr_table }
    }
}

impl<'h> Tree<'h> for BasicTree<'h> {
    fn core(&self) -> &TreeCore<'_> {
        &self.core
    }

    fn insert(&mut self, slot: &Slot, instr: &Instr) {
        self.core.insert(slot, instr);
    }

    fn remove(&mut self, slot: &Slot) {
        self.core.remove(slot);
    }

    fn avail_instrs(&self, _: &Slot) -> &'h [Instr] {
        let (avail_states, avail_colors) = self.core.avail_params();

        &self.instr_table[avail_states as usize][avail_colors as usize]
    }
}

/**************************************/

struct BlankTree<'h> {
    core: TreeCore<'h>,

    avail_blanks: Vec<Option<Slots>>,
    instr_table: &'h BlankInstrTable,
}

impl<'h> BlankTree<'h> {
    fn init(
        params @ (states, colors): Params,
        sim_lim: Step,
        harvester: &'h dyn Fn(&Prog),
        instr_table: &'h BlankInstrTable,
    ) -> Self {
        let core = TreeCore::init(params, false, sim_lim, harvester);

        let blank_slots = Some((states * (colors - 1)) as Slots);

        let avail_blanks = vec![blank_slots];

        Self {
            core,
            avail_blanks,
            instr_table,
        }
    }

    #[expect(clippy::unwrap_in_result)]
    fn avail_blanks(&self) -> Option<Slots> {
        *self.avail_blanks.last().unwrap()
    }

    fn update_blanks(
        &mut self,
        (_, slot_color): &Slot,
        (instr_color, _, _): &Instr,
    ) {
        let mut avail_blanks = self.avail_blanks();

        if let Some(remaining) = avail_blanks
            && *slot_color != 0
        {
            if *instr_color == 0 {
                avail_blanks = None;
            } else {
                avail_blanks = Some(remaining - 1);
            }
        }

        self.avail_blanks.push(avail_blanks);
    }
}

impl<'h> Tree<'h> for BlankTree<'h> {
    fn core(&self) -> &TreeCore<'_> {
        &self.core
    }

    fn insert(&mut self, slot: &Slot, instr: &Instr) {
        self.core.insert(slot, instr);

        self.update_blanks(slot, instr);
    }

    fn remove(&mut self, slot: &Slot) {
        self.core.remove(slot);

        self.avail_blanks.pop();
    }

    fn avail_instrs(&self, &(_, color): &Slot) -> &'h [Instr] {
        let (avail_states, avail_colors) = self.core.avail_params();

        let erase_remaining = self.avail_blanks();

        let erase_required = color != 0 && erase_remaining == Some(1);

        &self.instr_table[usize::from(erase_required)]
            [avail_states as usize][avail_colors as usize]
    }
}

/**************************************/

fn build_all(
    params @ (states, colors): Params,
    halt: bool,
    sim_lim: Step,
    harvester: &(impl Fn(&Prog) + Sync),
) {
    let init_states = min(3, states);
    let init_colors = min(3, colors);

    let instr_table =
        make_instr_table(states as usize, colors as usize);

    let mut init_instrs =
        instr_table[init_states as usize][init_colors as usize].clone();

    init_instrs.retain(|instr| !matches!(instr, (_, true, 0 | 1)));

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

fn build_blank(
    params @ (states, colors): Params,
    sim_lim: Step,
    harvester: &(impl Fn(&Prog) + Sync),
) {
    let init_states = min(3, states);
    let init_colors = min(3, colors);

    let instr_table =
        make_blank_table(states as usize, colors as usize);

    let mut init_instrs = instr_table[usize::from(false)]
        [init_states as usize][init_colors as usize]
        .clone();

    init_instrs.retain(|instr| !matches!(instr, (_, true, 0 | 1)));

    init_instrs.par_iter().for_each(|&next_instr| {
        let mut prog =
            BlankTree::init(params, sim_lim, harvester, &instr_table);

        prog.with_instr(&(1, 0), &next_instr, |prog| {
            prog.branch(Config::init_stepped());
        });
    });
}

fn build_spinout(
    params @ (states, colors): Params,
    sim_lim: Step,
    harvester: &(impl Fn(&Prog) + Sync),
) {
    let init_states = min(3, states);
    let init_colors = min(3, colors);

    let instr_table =
        make_instr_table(states as usize, colors as usize);

    let mut init_instrs =
        instr_table[init_states as usize][init_colors as usize].clone();

    init_instrs.retain(|instr| !matches!(instr, (_, true, 0 | 1)));

    if states == 2 {
        init_instrs.retain(|instr| matches!(instr, (_, _, 1)));
    }

    init_instrs.par_iter().for_each(|&next_instr| {
        let mut prog = BasicTree::init(
            params,
            false,
            sim_lim,
            harvester,
            &instr_table,
        );

        prog.with_instr(&(1, 0), &next_instr, |prog| {
            prog.branch(Config::init_stepped());
        });
    });
}

/**************************************/

pub fn build_tree(
    params: Params,
    goal: Option<Goal>,
    sim_lim: Step,
    harvester: &(impl Fn(&Prog) + Sync),
) {
    match goal {
        Some(Goal::Halt) | None => {
            build_all(params, goal.is_some(), sim_lim, harvester);
        },
        Some(Goal::Blank) => {
            build_blank(params, sim_lim, harvester);
        },
        Some(Goal::Spinout) => {
            build_spinout(params, sim_lim, harvester);
        },
    }
}

/**************************************/

#[cfg(test)]
impl BlankTree<'_> {
    fn tinsert(&mut self, slot: &Slot, instr: &str) {
        self.insert(slot, &Instr::read(instr));
    }

    #[track_caller]
    fn assert_erase(&self, erase: Option<Slots>) {
        assert_eq!(erase, self.avail_blanks());
    }

    #[track_caller]
    fn assert_instrs(&self, len: usize, slot: &Slot) {
        assert_eq!(len, self.avail_instrs(slot).len());
    }

    #[track_caller]
    fn insert_assert(
        &mut self,
        slot: &str,
        instr: &str,
        erase: Option<Slots>,
        len: usize,
    ) {
        let slot = Slot::read(slot);

        self.tinsert(&slot, instr);
        self.assert_erase(erase);
        self.assert_instrs(len, &slot);
    }
}

#[test]
fn test_blank_tree_1() {
    // "1RB 1LD  1RC 0RB  0LD 1RA  1LB 1LC": {
    //     "1RB ...  ... ...  ... ...  ... ...": ( 1, 'B0'),
    //     "1RB ...  1RC ...  ... ...  ... ...": ( 2, 'C0'),
    //     "1RB ...  1RC ...  0LD ...  ... ...": ( 3, 'D1'),
    //     "1RB ...  1RC ...  0LD ...  ... 1LC": ( 4, 'C1'),
    //     "1RB ...  1RC ...  0LD 1RA  ... 1LC": ( 5, 'A1'),
    //     "1RB 1LD  1RC ...  0LD 1RA  ... 1LC": ( 8, 'D0'),
    //     "1RB 1LD  1RC ...  0LD 1RA  1LB 1LC": (12, 'B1'),
    // },

    let table = make_blank_table(4, 2);

    let mut tree = BlankTree::init((4, 2), 100, &|_| {}, &table);

    tree.insert_assert("B0", "1RC", Some(4), 16);
    tree.insert_assert("C0", "0LD", Some(4), 16);
    tree.insert_assert("D1", "1LC", Some(3), 16);
    tree.insert_assert("C1", "1RA", Some(2), 16);
    tree.insert_assert("A1", "1LD", Some(1), 8);
    tree.insert_assert("D0", "1LB", Some(1), 16);
}

#[test]
fn test_blank_tree_2() {
    // "1RB 1LA  1RC 1LB  0LA 1LD  ... 1LA": {
    //     "1RB ...  ... ...  ... ...  ... ...": ( 1, 'B0'),
    //     "1RB ...  1RC ...  ... ...  ... ...": ( 2, 'C0'),
    //     "1RB ...  1RC ...  0LA ...  ... ...": ( 3, 'A1'),
    //     "1RB 1LA  1RC ...  0LA ...  ... ...": ( 6, 'B1'),
    //     "1RB 1LA  1RC 1LB  0LA ...  ... ...": ( 9, 'C1'),
    //     "1RB 1LA  1RC 1LB  0LA 1LD  ... ...": (10, 'D1'),
    // },

    let table = make_blank_table(4, 2);

    let mut tree = BlankTree::init((4, 2), 100, &|_| {}, &table);

    tree.insert_assert("B0", "1RC", Some(4), 16);
    tree.insert_assert("C0", "0LA", Some(4), 16);
    tree.insert_assert("A1", "1LA", Some(3), 16);
    tree.insert_assert("B1", "1LB", Some(2), 16);
    tree.insert_assert("C1", "1LD", Some(1), 8);
    tree.insert_assert("D1", "1LA", Some(0), 16);
}
