use core::cmp::{max, min};

use ahash::HashMap as Dict;
use rayon::prelude::*;

use crate::{
    Color, Colors, Goal, Instr, Params, Prog, Shift, Slot, State,
    States, Steps, config::MedConfig as Config, machine::RunResult,
};

pub use crate::config::PassConfig;

pub type TreeResult<Harv> = Dict<Instr, Harv>;

type Slots = u8;

use RunResult::*;

/**************************************/

const SHIFTS: [Shift; 2] = [false, true];

type Instrs = Vec<Instr>;
type InstrTable = Vec<Vec<Instrs>>;

#[expect(clippy::needless_range_loop)]
#[expect(clippy::cast_possible_truncation)]
fn make_instr_table(
    max_states: States,
    max_colors: Colors,
) -> (Instrs, InstrTable) {
    let mut table = vec![vec![vec![]; 1 + max_colors]; 1 + max_states];

    for states in 2..=max_states {
        for colors in 2..=max_colors {
            let mut instrs = Vec::with_capacity(colors * 2 * states);

            for color in 0..colors as Color {
                for shift in SHIFTS {
                    for state in 0..states as State {
                        instrs.push((color, shift, state));
                    }
                }
            }

            table[states][colors] = instrs;
        }
    }

    let init_states = min(3, max_states);
    let init_colors = min(3, max_colors);

    let mut init_instrs = table[init_states][init_colors].clone();

    init_instrs.retain(|instr| !matches!(instr, (_, true, 0 | 1)));

    (init_instrs, table)
}

type BlankInstrTable = [InstrTable; 2];

#[expect(clippy::needless_range_loop)]
fn make_blank_table(
    max_states: States,
    max_colors: Colors,
) -> (Instrs, BlankInstrTable) {
    let (init_instrs, table) = make_instr_table(max_states, max_colors);

    let mut partial = table.clone();

    for states in 2..=max_states {
        for colors in 2..=max_colors {
            partial[states][colors].retain(|&(co, _, _)| co == 0);
        }
    }

    (init_instrs, [table, partial])
}

type SpinoutInstrTable = [InstrTable; 2];

#[expect(clippy::needless_range_loop)]
#[expect(clippy::cast_possible_truncation)]
fn make_spinout_table(
    max_states: States,
    max_colors: Colors,
) -> (Instrs, SpinoutInstrTable) {
    let (init_instrs, plain) = make_instr_table(max_states, max_colors);

    let mut spins: InstrTable =
        vec![vec![Vec::new(); 1 + max_colors]; 1 + max_states];

    for read in 0..max_states {
        for colors in 2..=max_colors {
            let mut instrs = Vec::with_capacity(colors * 2);

            for color in 0..colors as Color {
                for shift in SHIFTS {
                    instrs.push((color, shift, read as State));
                }
            }

            spins[1 + read][colors] = instrs;
        }
    }

    (init_instrs, [plain, spins])
}

/**************************************/

trait AvailInstrs<'h> {
    fn avail_instrs(&self, slot: &Slot, params: Params) -> &'h [Instr];

    fn on_insert(&mut self, _: &Slot, _: &Instr) {}
    fn on_remove(&mut self) {}
}

struct AvailStack<T>(Vec<T>);

impl<T: Copy> AvailStack<T> {
    fn new(val: T) -> Self {
        Self(vec![val])
    }

    fn top(&self) -> T {
        *self.0.last().unwrap()
    }

    fn push(&mut self, val: T) {
        self.0.push(val);
    }

    fn pop(&mut self) {
        self.0.pop();
    }
}

struct BasicInstrs<'h> {
    instr_table: &'h InstrTable,
}

impl<'h> AvailInstrs<'h> for BasicInstrs<'h> {
    fn avail_instrs(&self, _: &Slot, (st, co): Params) -> &'h [Instr] {
        &self.instr_table[st][co]
    }
}

struct BlankInstrs<'h> {
    instr_table: &'h BlankInstrTable,
    avail_blanks: AvailStack<Option<Slots>>,
}

impl<'h> AvailInstrs<'h> for BlankInstrs<'h> {
    fn avail_instrs(
        &self,
        &(_, pr): &Slot,
        (st, co): Params,
    ) -> &'h [Instr] {
        &self.instr_table
            [usize::from(pr != 0 && self.avail_blanks.top() == Some(1))]
            [st][co]
    }

    fn on_insert(&mut self, &(_, sc): &Slot, &(pr, _, _): &Instr) {
        let next = if pr == 0 && sc != 0 {
            None
        } else {
            self.avail_blanks
                .top()
                .map(|rem| if sc != 0 { rem - 1 } else { rem })
        };

        self.avail_blanks.push(next);
    }

    fn on_remove(&mut self) {
        self.avail_blanks.pop();
    }
}

struct SpinoutInstrs<'h> {
    instr_table: &'h SpinoutInstrTable,
    avail_spinouts: AvailStack<Option<Slots>>,
}

impl<'h> AvailInstrs<'h> for SpinoutInstrs<'h> {
    fn avail_instrs(
        &self,
        &(read_state, read_color): &Slot,
        (states, colors): Params,
    ) -> &'h [Instr] {
        let spinout =
            read_color == 0 && self.avail_spinouts.top() == Some(1);

        &(if spinout {
            &self.instr_table[1][1 + read_state as usize]
        } else {
            &self.instr_table[0][states]
        })[colors]
    }

    fn on_insert(&mut self, &(st, co): &Slot, &(_, _, tr): &Instr) {
        let next = if co != 0 {
            self.avail_spinouts.top()
        } else if st == tr {
            None
        } else {
            self.avail_spinouts.top().map(|rem| rem - 1)
        };

        self.avail_spinouts.push(next);
    }

    fn on_remove(&mut self) {
        self.avail_spinouts.pop();
    }
}

/**************************************/

struct Tree<AvIn, Harv> {
    prog: Prog,
    instrs: AvIn,
    sim_lim: Steps,
    avail_params: AvailStack<Params>,
    remaining_slots: Slots,
    harvester: Harv,
}

impl<'i, AvIn: AvailInstrs<'i>, Harv: Harvester> Tree<AvIn, Harv> {
    fn init(
        (states, colors): Params,
        halt: Slots,
        instrs: AvIn,
        sim_lim: Steps,
        harvester: Harv,
    ) -> Self {
        let prog = Prog::init_norm(states, colors);

        let init_avail = (min(3, states), min(3, colors));

        let avail_params = AvailStack::new(init_avail);

        #[expect(clippy::cast_possible_truncation)]
        let remaining_slots = prog.dimension as Slots - halt - 2;

        Self {
            prog,
            instrs,
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
        self.prog.run_basic(self.sim_lim, config)
    }

    fn harvest(&mut self, config: PassConfig<'_>) {
        self.harvester.harvest(&self.prog, config);
    }

    fn insert_and_update(&mut self, slot: &Slot, instr: &Instr) {
        self.remaining_slots -= 1;

        self.prog.insert(slot, instr);

        self.update_avail(slot, instr);

        self.instrs.on_insert(slot, instr);
    }

    fn remove_and_update(&mut self, slot: &Slot) {
        self.avail_params.pop();

        self.prog.remove(slot);

        self.remaining_slots += 1;

        self.instrs.on_remove();
    }

    fn avail_instrs(&self, slot: &Slot) -> &'i [Instr] {
        self.instrs.avail_instrs(slot, self.avail_params.top())
    }

    #[expect(clippy::cast_possible_truncation)]
    fn update_avail(
        &mut self,
        (slot_st, slot_co): &Slot,
        (instr_co, _, instr_st): &Instr,
    ) {
        let (mut av_st, mut av_co) = self.avail_params.top();

        if av_st < self.prog.states
            && 1 + max(slot_st, instr_st) == av_st as State
        {
            av_st += 1;
        }

        if av_co < self.prog.colors
            && 1 + max(slot_co, instr_co) == av_co as Color
        {
            av_co += 1;
        }

        self.avail_params.push((av_st, av_co));
    }

    fn with_update(
        &mut self,
        slot: &Slot,
        instr: &Instr,
        body: impl FnOnce(&mut Self),
    ) {
        self.insert_and_update(slot, instr);

        body(self);

        self.remove_and_update(slot);
    }

    fn with_insert(
        &mut self,
        slot: &Slot,
        instr: &Instr,
        body: impl FnOnce(&mut Self),
    ) {
        self.prog.insert(slot, instr);

        body(self);

        self.prog.remove(slot);
    }

    fn branch(&mut self, mut config: Config) {
        let slot @ (slot_state, _) = match self.run(&mut config) {
            Undefined(slot) => slot,
            Blank | Spinout => return,
            StepLimit => {
                self.harvest(PassConfig::Owned(config));
                return;
            },
            _ => {
                unreachable!()
            },
        };

        let mut avail_instrs: Vec<_> = self.avail_instrs(&slot).into();

        if config.tape.scan == 0 {
            avail_instrs.retain(|&(_, shift, state)| {
                !(config.state == state && config.tape.at_edge(shift))
            });
        } else if config.tape.lspan.blank() && config.tape.rspan.blank()
        {
            avail_instrs.retain(|&(pr, _, _)| pr != 0);
        }

        let (last_instr, instrs) = avail_instrs.split_last().unwrap();

        if self.final_slot() {
            for next_instr in instrs {
                self.with_insert(&slot, next_instr, |prog| {
                    prog.harvest(PassConfig::Borrowed(&config));
                });
            }

            self.with_insert(&slot, last_instr, |tree| {
                if matches!(
                    tree.prog
                        .run_basic(tree.prog.dimension, &mut config),
                    StepLimit
                ) {
                    tree.harvest(PassConfig::Owned(config));
                }
            });

            return;
        }

        config.state = slot_state;

        for next_instr in instrs {
            self.with_update(&slot, next_instr, |prog| {
                prog.branch(config.clone());
            });
        }

        self.with_update(&slot, last_instr, |prog| {
            prog.branch(config);
        });
    }
}

/**************************************/

impl<'i, Harv: Harvester> Tree<BasicInstrs<'i>, Harv> {
    fn all(
        params: Params,
        halt: Slots,
        sim_lim: Steps,
        harvester: Harv,
        instr_table: &'i InstrTable,
    ) -> Self {
        let instrs = BasicInstrs { instr_table };

        Self::init(params, halt, instrs, sim_lim, harvester)
    }
}

impl<'i, Harv: Harvester> Tree<BlankInstrs<'i>, Harv> {
    fn blank(
        params @ (states, colors): Params,
        sim_lim: Steps,
        harvester: Harv,
        instr_table: &'i BlankInstrTable,
    ) -> Self {
        #[expect(clippy::cast_possible_truncation)]
        let avail_blanks =
            AvailStack::new(Some((states * (colors - 1)) as Slots));

        let instrs = BlankInstrs {
            instr_table,
            avail_blanks,
        };

        Self::init(params, 0, instrs, sim_lim, harvester)
    }
}

impl<'i, Harv: Harvester> Tree<SpinoutInstrs<'i>, Harv> {
    fn spinout(
        params @ (states, _): Params,
        sim_lim: Steps,
        harvester: Harv,
        instr_table: &'i SpinoutInstrTable,
    ) -> Self {
        #[expect(clippy::cast_possible_truncation)]
        let avail_spinouts =
            AvailStack::new(Some((states - 1) as Slots));

        let instrs = SpinoutInstrs {
            instr_table,
            avail_spinouts,
        };

        Self::init(params, 0, instrs, sim_lim, harvester)
    }
}

/**************************************/

impl<'i, AvIn: AvailInstrs<'i>, Harv: Harvester> Tree<AvIn, Harv> {
    fn init_branch(&mut self, instr: &Instr) {
        self.with_update(&(1, 0), instr, |tree: &mut Self| {
            tree.branch(Config::init_stepped());
        });
    }

    fn run_branch(
        init_instrs: &Instrs,
        make_tree: impl Sync + Fn() -> Self,
    ) -> TreeResult<Harv> {
        init_instrs
            .par_iter()
            .map(|instr| {
                let mut tree = make_tree();

                tree.init_branch(instr);

                (*instr, tree.harvester)
            })
            .collect()
    }
}

/**************************************/

pub trait Harvester: Send + Sized {
    fn harvest(&mut self, prog: &Prog, config: PassConfig<'_>);

    type Output;

    fn combine(results: &TreeResult<Self>) -> Self::Output;

    fn run_params(
        params: Params,
        goal: Option<Goal>,
        sim_lim: Steps,
        harvester: impl Sync + Fn() -> Self,
    ) -> Self::Output {
        let results = match goal {
            Some(Goal::Halt) | None => Self::run_all(
                params,
                Slots::from(goal.is_some()),
                sim_lim,
                harvester,
            ),
            Some(Goal::Blank) => {
                Self::run_blank(params, sim_lim, harvester)
            },
            Some(Goal::Spinout) => {
                Self::run_spinout(params, sim_lim, harvester)
            },
        };

        Self::combine(&results)
    }

    fn run_instrs(
        instrs: Slots,
        sim_lim: Steps,
        harvester: impl Sync + Fn() -> Self,
    ) -> Self::Output {
        let results = Self::run_all(
            (instrs.into(), instrs.into()),
            (instrs * instrs) - instrs,
            sim_lim,
            harvester,
        );

        Self::combine(&results)
    }

    fn run_all(
        params @ (states, colors): Params,
        halt: Slots,
        sim_lim: Steps,
        harvester: impl Sync + Fn() -> Self,
    ) -> TreeResult<Self> {
        let (init_instrs, instr_table) =
            make_instr_table(states, colors);

        Tree::run_branch(&init_instrs, || {
            Tree::all(params, halt, sim_lim, harvester(), &instr_table)
        })
    }

    fn run_blank(
        params @ (states, colors): Params,
        sim_lim: Steps,
        harvester: impl Sync + Fn() -> Self,
    ) -> TreeResult<Self> {
        let (init_instrs, instr_table) =
            make_blank_table(states, colors);

        Tree::run_branch(&init_instrs, || {
            Tree::blank(params, sim_lim, harvester(), &instr_table)
        })
    }

    fn run_spinout(
        params @ (states, colors): Params,
        sim_lim: Steps,
        harvester: impl Sync + Fn() -> Self,
    ) -> TreeResult<Self> {
        let (init_instrs, instr_table) =
            make_spinout_table(states, colors);

        let (init_spins, init_other) =
            init_instrs.into_iter().partition(|&(_, _, tr)| tr == 1);

        let mut spins_result = Tree::run_branch(&init_spins, || {
            Tree::all(params, 0, sim_lim, harvester(), &instr_table[0])
        });

        if states == 2 {
            return spins_result;
        }

        let other_result = Tree::run_branch(&init_other, || {
            Tree::spinout(params, sim_lim, harvester(), &instr_table)
        });

        spins_result.extend(other_result);

        spins_result
    }
}
