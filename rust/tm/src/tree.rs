use core::cmp::{max, min};

use ahash::HashMap as Dict;
use rayon::prelude::*;

use crate::{
    Color, Goal, Instr, Prog, Shift, Slot, State, Steps,
    config::MedConfig as Config, machine::RunResult,
};

pub use crate::config::PassConfig;

pub type TreeResult<Harv> = Dict<Instr, Harv>;

type Slots = usize;

type Params = (usize, usize);

use RunResult::*;

/**************************************/

const SHIFTS: [Shift; 2] = [false, true];

type Instrs = Vec<Instr>;
type InstrTable = Vec<Vec<Instrs>>;

#[expect(clippy::needless_range_loop)]
#[expect(clippy::cast_possible_truncation)]
fn make_instr_table<
    const max_states: usize,
    const max_colors: usize,
>() -> (Instrs, InstrTable) {
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
fn make_blank_table<
    const max_states: usize,
    const max_colors: usize,
>() -> (Instrs, BlankInstrTable) {
    let (init_instrs, table) =
        make_instr_table::<max_states, max_colors>();

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
fn make_spinout_table<
    const max_states: usize,
    const max_colors: usize,
>() -> (Instrs, SpinoutInstrTable) {
    let (init_instrs, plain) =
        make_instr_table::<max_states, max_colors>();

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

type AvailParams = AvailStack<Params>;

impl AvailParams {
    fn init(states: usize, colors: usize) -> Self {
        Self::new((min(3, states), min(3, colors)))
    }

    fn avail(&self) -> Params {
        self.top()
    }

    fn on_remove(&mut self) {
        self.pop();
    }

    #[expect(clippy::cast_possible_truncation)]
    fn on_insert<const states: usize, const colors: usize>(
        &mut self,
        (slot_st, slot_co): &Slot,
        (instr_co, _, instr_st): &Instr,
    ) {
        let (mut av_st, mut av_co) = self.top();

        if av_st < states
            && 1 + max(slot_st, instr_st) == av_st as State
        {
            av_st += 1;
        }

        if av_co < colors
            && 1 + max(slot_co, instr_co) == av_co as Color
        {
            av_co += 1;
        }

        self.push((av_st, av_co));
    }
}

/**************************************/

trait AvailInstrs<'h, const states: usize, const colors: usize> {
    type Table: Sync;

    fn new(instr_table: &'h Self::Table) -> Self;

    fn avail_instrs(&self, slot: &Slot) -> &'h [Instr];

    fn on_insert(&mut self, _: &Slot, _: &Instr) {}
    fn on_remove(&mut self) {}
}

struct BasicInstrs<'h> {
    instr_table: &'h InstrTable,
    avail_params: AvailParams,
}

impl<'h, const states: usize, const colors: usize>
    AvailInstrs<'h, states, colors> for BasicInstrs<'h>
{
    type Table = InstrTable;

    fn new(instr_table: &'h Self::Table) -> Self {
        let avail_params = AvailStack::init(states, colors);

        Self {
            instr_table,
            avail_params,
        }
    }

    fn avail_instrs(&self, _: &Slot) -> &'h [Instr] {
        let (st, co) = self.avail_params.avail();

        &self.instr_table[st][co]
    }

    fn on_remove(&mut self) {
        self.avail_params.on_remove();
    }

    fn on_insert(&mut self, slot: &Slot, instr: &Instr) {
        self.avail_params.on_insert::<states, colors>(slot, instr);
    }
}

struct AvailBlanks(AvailStack<Option<Slots>>);

impl AvailBlanks {
    fn init(states: usize, colors: usize) -> Self {
        let init_blanks = states * (colors - 1);

        Self(AvailStack::new(Some(init_blanks)))
    }

    fn must_erase(&self, print: Color) -> bool {
        print != 0 && self.0.top() == Some(1)
    }

    fn on_remove(&mut self) {
        self.0.pop();
    }

    fn on_insert(&mut self, &(_, sc): &Slot, &(pr, _, _): &Instr) {
        let next = if pr == 0 && sc != 0 {
            None
        } else {
            self.0.top().map(|rem| if sc != 0 { rem - 1 } else { rem })
        };

        self.0.push(next);
    }
}

struct BlankInstrs<'h> {
    instr_table: &'h BlankInstrTable,
    avail_params: AvailParams,
    avail_blanks: AvailBlanks,
}

impl<'h, const states: usize, const colors: usize>
    AvailInstrs<'h, states, colors> for BlankInstrs<'h>
{
    type Table = BlankInstrTable;

    fn new(instr_table: &'h Self::Table) -> Self {
        let avail_params = AvailStack::init(states, colors);

        let avail_blanks = AvailBlanks::init(states, colors);

        Self {
            instr_table,
            avail_params,
            avail_blanks,
        }
    }

    fn avail_instrs(&self, &(_, pr): &Slot) -> &'h [Instr] {
        let (st, co) = self.avail_params.avail();

        let must_erase = self.avail_blanks.must_erase(pr);

        &self.instr_table[usize::from(must_erase)][st][co]
    }

    fn on_insert(&mut self, slot: &Slot, instr: &Instr) {
        self.avail_params.on_insert::<states, colors>(slot, instr);

        self.avail_blanks.on_insert(slot, instr);
    }

    fn on_remove(&mut self) {
        self.avail_blanks.on_remove();

        self.avail_params.on_remove();
    }
}

struct AvailSpinouts(AvailStack<Option<Slots>>);

impl AvailSpinouts {
    fn init(states: usize, _: usize) -> Self {
        let init_spins = states - 1;

        Self(AvailStack::new(Some(init_spins)))
    }

    fn must_spin(&self, read_color: Color) -> bool {
        read_color == 0 && self.0.top() == Some(1)
    }

    fn on_remove(&mut self) {
        self.0.pop();
    }

    fn on_insert(&mut self, &(st, co): &Slot, &(_, _, tr): &Instr) {
        let next = if co != 0 {
            self.0.top()
        } else if st == tr {
            None
        } else {
            self.0.top().map(|rem| rem - 1)
        };

        self.0.push(next);
    }
}

struct SpinoutInstrs<'h> {
    instr_table: &'h SpinoutInstrTable,
    avail_params: AvailParams,
    avail_spinouts: AvailSpinouts,
}

impl<'h, const states: usize, const colors: usize>
    AvailInstrs<'h, states, colors> for SpinoutInstrs<'h>
{
    type Table = SpinoutInstrTable;

    fn new(instr_table: &'h Self::Table) -> Self {
        let avail_params = AvailStack::init(states, colors);

        let avail_spinouts = AvailSpinouts::init(states, colors);

        Self {
            instr_table,
            avail_params,
            avail_spinouts,
        }
    }

    fn avail_instrs(
        &self,
        &(read_state, read_color): &Slot,
    ) -> &'h [Instr] {
        let (st, co) = self.avail_params.avail();

        &(if self.avail_spinouts.must_spin(read_color) {
            &self.instr_table[1][1 + read_state as usize]
        } else {
            &self.instr_table[0][st]
        })[co]
    }

    fn on_insert(&mut self, slot: &Slot, instr: &Instr) {
        self.avail_params.on_insert::<states, colors>(slot, instr);

        self.avail_spinouts.on_insert(slot, instr);
    }

    fn on_remove(&mut self) {
        self.avail_spinouts.on_remove();

        self.avail_params.on_remove();
    }
}

struct BasicInstrsSmall<'h, const STATES: usize, const COLORS: usize> {
    instrs: &'h [Instr],
}

impl<'h, const states: usize, const colors: usize>
    AvailInstrs<'h, states, colors>
    for BasicInstrsSmall<'h, states, colors>
{
    type Table = InstrTable;

    fn new(instr_table: &'h Self::Table) -> Self {
        Self {
            instrs: &instr_table[states][colors],
        }
    }

    fn avail_instrs(&self, _slot: &Slot) -> &'h [Instr] {
        self.instrs
    }
}

struct BlankInstrsSmall<'h, const states: usize, const colors: usize> {
    instrs: [&'h [Instr]; 2],
    avail_blanks: AvailBlanks,
}

impl<'h, const states: usize, const colors: usize>
    AvailInstrs<'h, states, colors>
    for BlankInstrsSmall<'h, states, colors>
{
    type Table = BlankInstrTable;

    fn new(instr_table: &'h Self::Table) -> Self {
        let instrs = [
            &instr_table[0][states][colors][..],
            &instr_table[1][states][colors][..],
        ];

        let avail_blanks = AvailBlanks::init(states, colors);

        Self {
            instrs,
            avail_blanks,
        }
    }

    fn avail_instrs(&self, &(_, pr): &Slot) -> &'h [Instr] {
        let must_erase = self.avail_blanks.must_erase(pr);
        self.instrs[usize::from(must_erase)]
    }

    fn on_insert(&mut self, slot: &Slot, instr: &Instr) {
        self.avail_blanks.on_insert(slot, instr);
    }

    fn on_remove(&mut self) {
        self.avail_blanks.on_remove();
    }
}

/**************************************/

struct Tree<const states: usize, const colors: usize, AvIn, Harv> {
    prog: Prog<states, colors>,
    instrs: AvIn,
    sim_lim: Steps,
    remaining_slots: Slots,
    harvester: Harv,
}

impl<
    'i,
    const states: usize,
    const colors: usize,
    AvIn: AvailInstrs<'i, states, colors>,
    Harv: Harvester<states, colors>,
> Tree<states, colors, AvIn, Harv>
{
    const fn init(
        halt: Slots,
        sim_lim: Steps,
        harvester: Harv,
        instrs: AvIn,
    ) -> Self {
        let prog = Prog::<states, colors>::init_norm();

        let remaining_slots = (states * colors) - halt - 2;

        Self {
            prog,
            instrs,
            sim_lim,
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

        self.instrs.on_insert(slot, instr);
    }

    fn remove_and_update(&mut self, slot: &Slot) {
        self.prog.remove(slot);

        self.remaining_slots += 1;

        self.instrs.on_remove();
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

        let mut avail_instrs: Vec<_> =
            self.instrs.avail_instrs(&slot).into();

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
                    tree.prog.run_basic(states * colors, &mut config),
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

    fn init_branch(&mut self, instr: &Instr) {
        self.with_update(&(1, 0), instr, |tree: &mut Self| {
            tree.branch(Config::init_stepped());
        });
    }

    fn run_branch(
        init_instrs: &Instrs,
        halt: Slots,
        sim_lim: Steps,
        instr_table: &'i AvIn::Table,
        harvester: impl Send + Sync + Fn() -> Harv,
    ) -> TreeResult<Harv> {
        init_instrs
            .par_iter()
            .map(|instr| {
                let mut tree = Self::init(
                    halt,
                    sim_lim,
                    harvester(),
                    AvIn::new(instr_table),
                );

                tree.init_branch(instr);

                (*instr, tree.harvester)
            })
            .collect()
    }
}

type BasicTree<'i, const s: usize, const c: usize, H> =
    Tree<s, c, BasicInstrs<'i>, H>;

type BasicTreeSmall<'i, const s: usize, const c: usize, H> =
    Tree<s, c, BasicInstrsSmall<'i, s, c>, H>;

type BlankTree<'i, const s: usize, const c: usize, H> =
    Tree<s, c, BlankInstrs<'i>, H>;

type BlankTreeSmall<'i, const s: usize, const c: usize, H> =
    Tree<s, c, BlankInstrsSmall<'i, s, c>, H>;

type SpinoutTree<'i, const s: usize, const c: usize, H> =
    Tree<s, c, SpinoutInstrs<'i>, H>;

/**************************************/

pub trait Harvester<const states: usize, const colors: usize>:
    Send + Sized
{
    fn harvest(
        &mut self,
        prog: &Prog<states, colors>,
        config: PassConfig<'_>,
    );

    type Output;

    fn combine(results: &TreeResult<Self>) -> Self::Output;

    fn run_params(
        goal: Option<Goal>,
        sim_lim: Steps,
        harvester: &(impl Send + Sync + Fn() -> Self),
    ) -> Self::Output {
        let results = match goal {
            Some(Goal::Halt) | None => Self::run_all(
                Slots::from(goal.is_some()),
                sim_lim,
                harvester,
            ),
            Some(Goal::Blank) => Self::run_blank(sim_lim, harvester),
            Some(Goal::Spinout) => {
                Self::run_spinout(sim_lim, harvester)
            },
        };

        Self::combine(&results)
    }

    fn run_instrs<const instrs: usize>(
        sim_lim: Steps,
        harvester: &(impl Send + Sync + Fn() -> Self),
    ) -> Self::Output {
        assert!(states == instrs);
        assert!(colors == instrs);

        let results = Self::run_all(
            (instrs * instrs) - instrs,
            sim_lim,
            harvester,
        );

        Self::combine(&results)
    }

    fn run_all(
        halt: Slots,
        sim_lim: Steps,
        harvester: &(impl Send + Sync + Fn() -> Self),
    ) -> TreeResult<Self> {
        let (init_instrs, instr_table) =
            make_instr_table::<states, colors>();

        if states <= 3 && colors <= 3 {
            BasicTreeSmall::run_branch(
                &init_instrs,
                halt,
                sim_lim,
                &instr_table,
                harvester,
            )
        } else {
            BasicTree::run_branch(
                &init_instrs,
                halt,
                sim_lim,
                &instr_table,
                harvester,
            )
        }
    }

    fn run_blank(
        sim_lim: Steps,
        harvester: &(impl Send + Sync + Fn() -> Self),
    ) -> TreeResult<Self> {
        let (init_instrs, instr_table) =
            make_blank_table::<states, colors>();

        if states <= 3 && colors <= 3 {
            BlankTreeSmall::run_branch(
                &init_instrs,
                0,
                sim_lim,
                &instr_table,
                harvester,
            )
        } else {
            BlankTree::run_branch(
                &init_instrs,
                0,
                sim_lim,
                &instr_table,
                harvester,
            )
        }
    }

    fn run_spinout(
        sim_lim: Steps,
        harvester: &(impl Send + Sync + Fn() -> Self),
    ) -> TreeResult<Self> {
        let (init_instrs, instr_table) =
            make_spinout_table::<states, colors>();

        let (init_spins, init_other) =
            init_instrs.into_iter().partition(|&(_, _, tr)| tr == 1);

        let mut spins_result = BasicTree::run_branch(
            &init_spins,
            0,
            sim_lim,
            &instr_table[0],
            harvester,
        );

        if states == 2 {
            return spins_result;
        }

        let other_result = SpinoutTree::run_branch(
            &init_other,
            0,
            sim_lim,
            &instr_table,
            harvester,
        );

        spins_result.extend(other_result);

        spins_result
    }
}
