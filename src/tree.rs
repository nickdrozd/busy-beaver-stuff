use core::{
    cmp::{max, min},
    fmt::Debug,
};
use std::sync::{Arc, Mutex, MutexGuard};

use pyo3::pyfunction;
use rayon::prelude::*;

use crate::{
    instrs::{Color, CompProg, Instr, Slot, State},
    parse::show_comp,
    tape::BasicTape as Tape,
};

type Step = u64;

/**************************************/

fn make_instrs(states: State, colors: Color) -> Vec<Instr> {
    let mut instrs = vec![];

    let shifts = [false, true];

    for color in 0..colors {
        for shift in shifts {
            for state in 0..states {
                instrs.push((color, shift, state));
            }
        }
    }

    instrs
}

/**************************************/

fn run_for_undefined(
    comp: &CompProg,
    mut state: State,
    tape: &mut Tape,
    sim_lim: Step,
) -> Result<Option<Slot>, ()> {
    for _ in 0..sim_lim {
        let slot = (state, tape.scan);

        let Some(&(color, shift, next_state)) = comp.get(&slot) else {
            return Ok(Some(slot));
        };

        let same = state == next_state;

        if same && tape.at_edge(shift) {
            return Err(());
        }

        let _ = tape.step(shift, color, same);

        if tape.blank() {
            return Err(());
        }

        state = next_state;
    }

    Ok(None)
}

/**************************************/

type Slots = u64;
type Params = (State, Color);
type Config<'t> = (State, &'t mut Tape);

fn leaf<F>(
    prog: &CompProg,
    (max_state, max_color): Params,
    harvester: &F,
) where
    F: Fn(&CompProg),
{
    if prog.values().all(|(_, _, state)| 1 + state < max_state)
        || prog.values().all(|(color, _, _)| 1 + color < max_color)
    {
        return;
    }

    harvester(prog);
}

#[allow(clippy::too_many_arguments)]
fn branch<F>(
    instr: Instr,
    prog: &mut CompProg,
    (state, tape): Config,
    sim_lim: Step,
    (mut avail_states, mut avail_colors): Params,
    params: Params,
    remaining_slots: Slots,
    harvester: &F,
) where
    F: Fn(&CompProg),
{
    let (max_states, max_colors) = params;

    let Ok(Some(slot)) = run_for_undefined(prog, state, tape, sim_lim)
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

    let next_remaining_slots = remaining_slots - 1;

    for next_instr in make_instrs(avail_states, avail_colors) {
        prog.insert(slot, next_instr);

        if next_remaining_slots == 0 {
            leaf(prog, params, harvester);
        } else {
            branch(
                next_instr,
                prog,
                (slot_state, &mut tape.clone()),
                sim_lim,
                (avail_states, avail_colors),
                (max_states, max_colors),
                next_remaining_slots,
                harvester,
            );
        }

        prog.remove(&slot);
    }
}

/**************************************/

fn build_tree<F>(
    (states, colors): Params,
    halt: bool,
    sim_lim: Step,
    harvester: &F,
) where
    F: Fn(&CompProg) + Sync,
{
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
                (1, &mut Tape::init_stepped()),
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

fn set_val<T>(val: T) -> Basket<T> {
    Arc::new(Mutex::new(val))
}

fn access<T>(basket: &Basket<T>) -> MutexGuard<'_, T> {
    basket.lock().unwrap()
}

fn get_val<T: Debug>(basket: Basket<T>) -> T {
    Arc::try_unwrap(basket).unwrap().into_inner().unwrap()
}

/**************************************/

#[pyfunction]
pub fn tree_progs(
    params: Params,
    halt: bool,
    sim_lim: Step,
) -> Vec<String> {
    let progs = set_val(vec![]);

    build_tree(params, halt, sim_lim, &|comp| {
        access(&progs).push(show_comp(comp, Some(params)));
    });

    get_val(progs)
}

/**************************************/

#[cfg(test)]
use crate::{
    machine::quick_term_or_rec,
    reason::{cant_halt, cant_spin_out},
};

#[cfg(test)]
fn skip(comp: &CompProg, params: Params, halt: bool) -> bool {
    quick_term_or_rec(comp, 50, true) || {
        let (states, colors) = params;

        (colors == 2 || states * colors < 10)
            && (if halt { cant_halt } else { cant_spin_out })(
                &show_comp(comp, Some(params)),
            )
    }
}

#[cfg(test)]
fn assert_tree(params: Params, halt: u8, leaves: u64) {
    let halt = halt != 0;

    let leaf_count = set_val(0);

    build_tree(params, halt, 100, &|prog| {
        if skip(prog, params, halt) {
            return;
        }

        *access(&leaf_count) += 1;
    });

    assert_eq!(get_val(leaf_count), leaves);
}

#[cfg(test)]
macro_rules! assert_trees {
    ( $( ( $params:expr, $halt:expr, $leaves:expr ) ),* $(,)? ) => {
        {
            vec![$( ($params, $halt, $leaves) ),*]
                .par_iter().for_each(|&(params, halt, leaves)| {
                assert_tree(params, halt, leaves);
            });
        }
    };
}

#[test]
fn test_tree() {
    assert_trees![
        ((2, 2), 1, 0),
        ((2, 2), 0, 0),
        //
        ((3, 2), 1, 26),
        ((3, 2), 0, 51),
        //
        ((2, 3), 1, 135),
        ((2, 3), 0, 143),
        //
        ((4, 2), 1, 6_769),
        ((4, 2), 0, 16_591),
        //
        ((2, 4), 1, 30_522),
        ((2, 4), 0, 90_555),
    ];
}

#[test]
#[ignore]
fn test_tree_slow() {
    assert_trees![
        ((5, 2), 1, 3_241_903),
        //
        ((2, 5), 1, 13_073_485),
        //
        ((3, 3), 1, 2_254_003),
        ((3, 3), 0, 5_986_602),
    ];
}

#[test]
fn test_progs() {
    let params = (3, 2);

    build_tree(params, true, 100, &|comp| {
        println!("{}", show_comp(comp, Some(params)));
    });
}
