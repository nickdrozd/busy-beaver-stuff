use core::{
    cmp::{max, min},
    fmt::Debug,
};

use std::sync::{Arc, Mutex, MutexGuard};

use rayon::prelude::*;

use crate::{
    instrs::{Color, CompProg, Instr, Params, Slot, State},
    tape::{BasicTape as Tape, MachineTape as _},
};

pub type Step = u64;

/**************************************/

const SHIFTS: [bool; 2] = [false, true];

fn make_instrs(states: State, colors: Color) -> Vec<Instr> {
    let mut instrs = vec![];

    for color in 0..colors {
        for shift in SHIFTS {
            for state in 0..states {
                instrs.push((color, shift, state));
            }
        }
    }

    instrs
}

/**************************************/

enum RunResult {
    Limit,
    Blank,
    Spinout,
    Undefined(Slot),
}

use RunResult::*;

fn run_for_undefined(
    comp: &CompProg,
    mut state: State,
    tape: &mut Tape,
    sim_lim: Step,
) -> RunResult {
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

type Slots = u64;
type Config = (State, Tape);

fn leaf(
    prog: &CompProg,
    (max_state, max_color): Params,
    harvester: &impl Fn(&CompProg),
) {
    if prog.values().all(|(_, _, state)| 1 + state < max_state)
        || prog.values().all(|(color, _, _)| 1 + color < max_color)
    {
        return;
    }

    harvester(prog);
}

fn branch(
    instr: Instr,
    prog: &mut CompProg,
    (state, mut tape): Config,
    sim_lim: Step,
    (mut avail_states, mut avail_colors): Params,
    params: Params,
    remaining_slots: Slots,
    harvester: &impl Fn(&CompProg),
) {
    let (max_states, max_colors) = params;

    let Undefined(slot) =
        run_for_undefined(prog, state, &mut tape, sim_lim)
    else {
        leaf(prog, params, harvester);
        return;
    };

    let (slot_state, slot_color) = slot;
    let (instr_color, _, instr_state) = instr;

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

    let instrs = make_instrs(avail_states, avail_colors);

    let next_remaining_slots = remaining_slots - 1;

    if next_remaining_slots == 0 {
        for next_instr in instrs {
            prog.insert(slot, next_instr);
            leaf(prog, params, harvester);
            prog.remove(&slot);
        }

        return;
    }

    let avail_params = (avail_states, avail_colors);

    let (last_instr, instrs) = instrs.split_last().unwrap();

    for &next_instr in instrs {
        prog.insert(slot, next_instr);

        branch(
            next_instr,
            prog,
            (slot_state, tape.clone()),
            sim_lim,
            avail_params,
            params,
            next_remaining_slots,
            harvester,
        );

        prog.remove(&slot);
    }

    {
        prog.insert(slot, *last_instr);

        branch(
            *last_instr,
            prog,
            (slot_state, tape),
            sim_lim,
            avail_params,
            params,
            next_remaining_slots,
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

    make_instrs(init_states, init_colors).par_iter().for_each(
        |&next_instr| {
            branch(
                next_instr,
                &mut CompProg::from([
                    ((0, 0), (1, true, 1)),
                    ((1, 0), next_instr),
                ]),
                (1, Tape::init_stepped()),
                sim_lim,
                (init_states, init_colors),
                (states, colors),
                (states * colors) - 1 - (1 + Slots::from(halt)),
                harvester,
            );
        },
    );
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
