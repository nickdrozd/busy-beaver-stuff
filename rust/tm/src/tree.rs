use core::cmp::{max, min};

use rayon::prelude::*;

use crate::{
    Color, Goal, Instr, Params, Prog, Shift, Slot, State, Steps,
    config::MedConfig as Config, machine::RunResult,
};

pub use crate::config::PassConfig;

type Slots = u8;

use RunResult::*;

/**************************************/

const SHIFTS: [Shift; 2] = [false, true];

type Instrs = Vec<Instr>;
type InstrTable = Vec<Vec<Instrs>>;

fn make_instr_table(
    max_states: State,
    max_colors: Color,
) -> (Instrs, InstrTable) {
    let mut table = vec![
        vec![vec![]; 1 + max_colors as usize];
        1 + max_states as usize
    ];

    for states in 2..=max_states {
        for colors in 2..=max_colors {
            let mut instrs =
                Vec::with_capacity((colors * 2 * states).into());

            for color in 0..colors {
                for shift in SHIFTS {
                    for state in 0..states {
                        instrs.push((color, shift, state));
                    }
                }
            }

            table[states as usize][colors as usize] = instrs;
        }
    }

    let init_states = min(3, max_states);
    let init_colors = min(3, max_colors);

    let mut init_instrs =
        table[init_states as usize][init_colors as usize].clone();

    init_instrs.retain(|instr| !matches!(instr, (_, true, 0 | 1)));

    (init_instrs, table)
}

type BlankInstrTable = [InstrTable; 2];

fn make_blank_table(
    max_states: State,
    max_colors: Color,
) -> (Instrs, BlankInstrTable) {
    let (init_instrs, table) = make_instr_table(max_states, max_colors);

    let mut partial = table.clone();

    for states in 2..=max_states {
        for colors in 2..=max_colors {
            partial[states as usize][colors as usize]
                .retain(|&(co, _, _)| co == 0);
        }
    }

    (init_instrs, [table, partial])
}

type SpinoutInstrTable = [InstrTable; 2];

fn make_spinout_table(
    max_states: State,
    max_colors: Color,
) -> (Instrs, SpinoutInstrTable) {
    let (init_instrs, plain) = make_instr_table(max_states, max_colors);

    let mut spins: InstrTable =
        vec![
            vec![Vec::new(); 1 + max_colors as usize];
            1 + max_states as usize
        ];

    for read_state in 0..max_states {
        for colors in 2..=max_colors {
            let mut instrs = Vec::with_capacity((colors * 2).into());

            for color in 0..colors {
                for shift in SHIFTS {
                    instrs.push((color, shift, read_state));
                }
            }

            spins[1 + read_state as usize][colors as usize] = instrs;
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

struct BasicInstrs<'h> {
    instr_table: &'h InstrTable,
}

impl<'h> AvailInstrs<'h> for BasicInstrs<'h> {
    fn avail_instrs(&self, _: &Slot, (st, co): Params) -> &'h [Instr] {
        &self.instr_table[st as usize][co as usize]
    }
}

struct BlankInstrs<'h> {
    instr_table: &'h BlankInstrTable,
    avail_blanks: Vec<Option<Slots>>,
}

impl BlankInstrs<'_> {
    #[expect(clippy::unwrap_in_result)]
    fn avail_blanks(&self) -> Option<Slots> {
        *self.avail_blanks.last().unwrap()
    }
}

impl<'h> AvailInstrs<'h> for BlankInstrs<'h> {
    fn avail_instrs(
        &self,
        &(_, pr): &Slot,
        (st, co): Params,
    ) -> &'h [Instr] {
        &self.instr_table
            [usize::from(pr != 0 && self.avail_blanks() == Some(1))]
            [st as usize][co as usize]
    }

    fn on_insert(&mut self, &(_, sc): &Slot, &(pr, _, _): &Instr) {
        let next = if pr == 0 && sc != 0 {
            None
        } else {
            self.avail_blanks()
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
    avail_spinouts: Vec<Option<Slots>>,
}

impl SpinoutInstrs<'_> {
    #[expect(clippy::unwrap_in_result)]
    fn avail_spinouts(&self) -> Option<Slots> {
        *self.avail_spinouts.last().unwrap()
    }
}

impl<'h> AvailInstrs<'h> for SpinoutInstrs<'h> {
    fn avail_instrs(
        &self,
        &(read_state, read_color): &Slot,
        (states, colors): Params,
    ) -> &'h [Instr] {
        let spinout =
            read_color == 0 && self.avail_spinouts() == Some(1);

        &(if spinout {
            &self.instr_table[1][1 + read_state as usize]
        } else {
            &self.instr_table[0][states as usize]
        })[colors as usize]
    }

    fn on_insert(&mut self, &(st, co): &Slot, &(_, _, tr): &Instr) {
        let next = if co != 0 {
            self.avail_spinouts()
        } else if st == tr {
            None
        } else {
            self.avail_spinouts().map(|rem| rem - 1)
        };

        self.avail_spinouts.push(next);
    }

    fn on_remove(&mut self) {
        self.avail_spinouts.pop();
    }
}

/**************************************/

struct Tree<'h, AvIn: AvailInstrs<'h>> {
    prog: Prog,
    instrs: AvIn,
    sim_lim: Steps,
    avail_params: Vec<Params>,
    remaining_slots: Slots,
    harvester: &'h dyn Fn(&Prog, PassConfig),
}

impl<'h, AvIn: AvailInstrs<'h>> Tree<'h, AvIn> {
    fn init(
        params @ (states, colors): Params,
        halt: Slots,
        instrs: AvIn,
        sim_lim: Steps,
        harvester: &'h dyn Fn(&Prog, PassConfig),
    ) -> Self {
        let prog = Prog::init_stepped(params);

        let init_avail = (min(3, states), min(3, colors));

        let avail_params = vec![init_avail];

        let remaining_slots = (states * colors) - halt - 2;

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

    fn harvest(&self, config: PassConfig<'_>) {
        (self.harvester)(&self.prog, config);
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

    fn avail_params(&self) -> Params {
        *self.avail_params.last().unwrap()
    }

    fn avail_instrs(&self, slot: &Slot) -> &'h [Instr] {
        self.instrs.avail_instrs(slot, self.avail_params())
    }

    fn update_avail(
        &mut self,
        (slot_st, slot_co): &Slot,
        (instr_co, _, instr_st): &Instr,
    ) {
        let (mut av_st, mut av_co) = self.avail_params();

        let (max_st, max_co) = self.prog.params();

        if av_st < max_st && 1 + max(slot_st, instr_st) == av_st {
            av_st += 1;
        }

        if av_co < max_co && 1 + max(slot_co, instr_co) == av_co {
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
                if !self.prog.incomplete() {
                    self.harvest(PassConfig::Owned(config));
                }

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
                    tree.prog.run_basic(
                        tree.prog.dimension.into(),
                        &mut config,
                    ),
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

type BasicTree<'h> = Tree<'h, BasicInstrs<'h>>;

impl<'h> BasicTree<'h> {
    fn make(
        params: Params,
        halt: Slots,
        sim_lim: Steps,
        harvester: &'h dyn Fn(&Prog, PassConfig<'_>),
        instr_table: &'h InstrTable,
    ) -> Self {
        let instrs = BasicInstrs { instr_table };

        Self::init(params, halt, instrs, sim_lim, harvester)
    }
}
/**************************************/

type BlankTree<'h> = Tree<'h, BlankInstrs<'h>>;

impl<'h> BlankTree<'h> {
    fn make(
        params @ (states, colors): Params,
        sim_lim: Steps,
        harvester: &'h dyn Fn(&Prog, PassConfig<'_>),
        instr_table: &'h BlankInstrTable,
    ) -> Self {
        let instrs = BlankInstrs {
            instr_table,
            avail_blanks: vec![Some(states * (colors - 1))],
        };

        Self::init(params, 0, instrs, sim_lim, harvester)
    }
}

/**************************************/

type SpinoutTree<'h> = Tree<'h, SpinoutInstrs<'h>>;

impl<'h> SpinoutTree<'h> {
    fn make(
        params @ (states, _): Params,
        sim_lim: Steps,
        harvester: &'h dyn Fn(&Prog, PassConfig<'_>),
        instr_table: &'h SpinoutInstrTable,
    ) -> Self {
        let instrs = SpinoutInstrs {
            instr_table,
            avail_spinouts: vec![Some(states - 1)],
        };

        Self::init(params, 0, instrs, sim_lim, harvester)
    }
}

/**************************************/

fn kick_off_branch<'h, AvIn: AvailInstrs<'h>>(
    init_instrs: &Instrs,
    make_tree: impl Sync + Fn() -> Tree<'h, AvIn>,
) {
    init_instrs.par_iter().for_each(|&next_instr| {
        make_tree().with_update(
            &(1, 0),
            &next_instr,
            |tree: &mut Tree<_>| {
                tree.branch(Config::init_stepped());
            },
        );
    });
}

fn build_all(
    params @ (states, colors): Params,
    halt: Slots,
    sim_lim: Steps,
    harvester: &(impl Fn(&Prog, PassConfig<'_>) + Sync),
) {
    let (init_instrs, instr_table) = make_instr_table(states, colors);

    kick_off_branch(&init_instrs, || {
        BasicTree::make(params, halt, sim_lim, harvester, &instr_table)
    });
}

fn build_blank(
    params @ (states, colors): Params,
    sim_lim: Steps,
    harvester: &(impl Fn(&Prog, PassConfig<'_>) + Sync),
) {
    let (init_instrs, instr_table) = make_blank_table(states, colors);

    kick_off_branch(&init_instrs, || {
        BlankTree::make(params, sim_lim, harvester, &instr_table)
    });
}

fn build_spinout(
    params @ (states, colors): Params,
    sim_lim: Steps,
    harvester: &(impl Fn(&Prog, PassConfig) + Sync),
) {
    let (init_instrs, instr_table) = make_spinout_table(states, colors);

    let (init_spins, init_other) =
        init_instrs.into_iter().partition(|&(_, _, tr)| tr == 1);

    kick_off_branch(&init_spins, || {
        BasicTree::make(params, 0, sim_lim, harvester, &instr_table[0])
    });

    if states == 2 {
        return;
    }

    kick_off_branch(&init_other, || {
        SpinoutTree::make(params, sim_lim, harvester, &instr_table)
    });
}

/**************************************/

pub fn build_tree(
    params: Params,
    goal: Option<Goal>,
    sim_lim: Steps,
    harvester: &(impl Fn(&Prog, PassConfig<'_>) + Sync),
) {
    match goal {
        Some(Goal::Halt) | None => {
            build_all(
                params,
                Slots::from(goal.is_some()),
                sim_lim,
                harvester,
            );
        },
        Some(Goal::Blank) => {
            build_blank(params, sim_lim, harvester);
        },
        Some(Goal::Spinout) => {
            build_spinout(params, sim_lim, harvester);
        },
    }
}

pub fn build_limited(
    instrs: Slots,
    sim_lim: Steps,
    harvester: &(impl Fn(&Prog, PassConfig<'_>) + Sync),
) {
    build_all(
        (instrs, instrs),
        (instrs * instrs) - instrs,
        sim_lim,
        harvester,
    );
}
