use core::{
    cmp::{max, min},
    fmt::Debug,
};

use std::sync::{Arc, Mutex, MutexGuard};

use rayon::prelude::*;

use crate::{
    instrs::{
        Color, CompProg, GetInstr as _, Instr, Params, Slot, State,
    },
    tape::{MachineTape as _, MedTape as Tape},
};

pub type Step = usize;

/**************************************/

const SHIFTS: [bool; 2] = [false, true];

type InstrTable = Vec<Vec<Vec<Instr>>>;

fn make_instr_table(
    max_states: State,
    max_colors: Color,
) -> InstrTable {
    let max_states = max_states as usize;
    let max_colors = max_colors as usize;

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

fn run(
    comp: &CompProg,
    config: &mut Config,
    sim_lim: Step,
) -> RunResult {
    let &mut (mut state, ref mut tape) = config;

    for _ in 0..sim_lim {
        let slot = (state, tape.scan);

        let Some(&(color, shift, next_state)) = comp.get(&slot) else {
            return Undefined(slot);
        };

        let same = state == next_state;

        if same && tape.at_edge(shift) {
            return Spinout;
        }

        tape.step(shift, color, same);

        if tape.blank() {
            return Blank;
        }

        state = next_state;
    }

    Limit
}

/**************************************/

type Slots = u8;
type Config = (State, Tape);

fn leaf(
    prog: &CompProg,
    params: Params,
    harvester: &impl Fn(&CompProg),
) {
    if prog.params_unreached(params) {
        return;
    }

    harvester(prog);
}

#[expect(clippy::too_many_arguments)]
fn branch(
    prog: &mut CompProg,
    mut config: Config,
    sim_lim: Step,
    &(instr_color, _, instr_state): &Instr,
    (mut avail_states, mut avail_colors): Params,
    params: Params,
    remaining_slots: Slots,
    instr_table: &InstrTable,
    harvester: &impl Fn(&CompProg),
) {
    let slot = match run(prog, &mut config, sim_lim) {
        Undefined(slot) => slot,
        Blank | Spinout => return,
        Limit => {
            leaf(prog, params, harvester);
            return;
        },
    };

    let (slot_state, slot_color) = slot;
    let (max_states, max_colors) = params;

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

    let avail_params = (avail_states, avail_colors);

    let (last_instr, instrs) = instrs.split_last().unwrap();

    let (_, tape) = config;

    for &next_instr in instrs {
        prog.insert(slot, next_instr);

        branch(
            prog,
            (slot_state, tape.clone()),
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
            (slot_state, tape),
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
    (states, colors): Params,
    halt: bool,
    sim_lim: Step,
    harvester: &(impl Fn(&CompProg) + Sync),
) {
    let init_states = min(3, states);
    let init_colors = min(3, colors);

    let instr_table = make_instr_table(states, colors);

    let init_instrs =
        &instr_table[init_states as usize][init_colors as usize];

    init_instrs.par_iter().for_each(|&next_instr| {
        branch(
            &mut CompProg::init_stepped(next_instr),
            (1, Tape::init_stepped()),
            sim_lim,
            &next_instr,
            (init_states, init_colors),
            (states, colors),
            ((states * colors) as Slots) - 1 - (1 + Slots::from(halt)),
            &instr_table,
            harvester,
        );
    });
}

/**************************************/

type Basket<T> = Arc<Mutex<T>>;

pub fn set_val<T>(val: T) -> Basket<T> {
    Arc::new(Mutex::new(val))
}

pub fn access<T>(basket: &Basket<T>) -> MutexGuard<'_, T> {
    basket.lock().unwrap()
}

pub fn get_val<T: Debug>(basket: Basket<T>) -> T {
    Arc::try_unwrap(basket).unwrap().into_inner().unwrap()
}
