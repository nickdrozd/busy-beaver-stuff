use core::cmp::{max, min};

use rayon::prelude::*;

use crate::{
    instrs::{Color, GetInstr as _, Instr, Params, Prog, Slot, State},
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

            let Some(&(color, shift, state)) = comp.get(&slot) else {
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

fn leaf(
    prog: &Prog,
    (max_state, max_color): Params,
    harvester: &impl Fn(&Prog),
) {
    if prog.states_unreached(max_state)
        || prog.colors_unreached(max_color)
    {
        return;
    }

    harvester(prog);
}

#[expect(clippy::too_many_arguments)]
fn branch(
    prog: &mut Prog,
    mut config: Config,
    sim_lim: Step,
    &(instr_color, _, instr_state): &Instr,
    (mut avail_states, mut avail_colors): Params,
    params @ (max_states, max_colors): Params,
    remaining_slots: Slots,
    instr_table: &InstrTable,
    harvester: &impl Fn(&Prog),
) {
    let slot @ (slot_state, slot_color) =
        match config.run(prog, sim_lim) {
            Undefined(slot) => slot,
            Blank | Spinout => return,
            Limit => {
                leaf(prog, params, harvester);
                return;
            },
        };

    if avail_states < max_states
        && 1 + max(slot_state, instr_state) == avail_states
    {
        avail_states += 1;
    }

    if avail_colors < max_colors
        && 1 + max(slot_color, instr_color) == avail_colors
    {
        avail_colors += 1;
    }

    let instrs =
        &instr_table[avail_states as usize][avail_colors as usize];

    let next_remaining_slots = remaining_slots - 1;

    if next_remaining_slots == 0 {
        for next_instr in instrs {
            prog.insert(slot, *next_instr);
            leaf(prog, params, harvester);
            prog.remove(&slot);
        }

        return;
    }

    config.state = slot_state;

    let avail_params = (avail_states, avail_colors);

    let (last_instr, instrs) = instrs.split_last().unwrap();

    for &next_instr in instrs {
        prog.insert(slot, next_instr);

        branch(
            prog,
            config.clone(),
            sim_lim,
            &next_instr,
            avail_params,
            params,
            next_remaining_slots,
            instr_table,
            harvester,
        );

        prog.remove(&slot);
    }

    {
        prog.insert(slot, *last_instr);

        branch(
            prog,
            config,
            sim_lim,
            last_instr,
            avail_params,
            params,
            next_remaining_slots,
            instr_table,
            harvester,
        );

        prog.remove(&slot);
    }
}

/**************************************/

pub fn build_tree(
    params @ (states, colors): Params,
    goal: Option<u8>,
    sim_lim: Step,
    harvester: &(impl Fn(&Prog) + Sync),
) {
    let slots = open_slots(
        states,
        colors,
        goal.is_some_and(|goal| Goal::from(goal).is_halt()),
    );

    let init_states = min(3, states);
    let init_colors = min(3, colors);

    let instr_table =
        make_instr_table(states as usize, colors as usize);

    let mut init_instrs =
        instr_table[init_states as usize][init_colors as usize].clone();

    init_instrs.retain(|instr| !matches!(instr, (_, true, 0 | 1)));

    if states == 2
        && goal.is_some_and(|goal| Goal::from(goal).is_spinout())
    {
        init_instrs.retain(|instr| matches!(instr, (_, _, 1)));
    }

    init_instrs.par_iter().for_each(|&next_instr| {
        branch(
            &mut Prog::init_stepped(next_instr),
            Config::init_stepped(),
            sim_lim,
            &next_instr,
            (init_states, init_colors),
            params,
            slots,
            &instr_table,
            harvester,
        );
    });
}

/**************************************/

fn open_slots(states: State, colors: Color, halt: bool) -> Slots {
    ((states * colors) as Slots) - Slots::from(halt) - 2
}
