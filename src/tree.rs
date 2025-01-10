use core::{
    cmp::{max, min},
    fmt::Debug,
};

use std::sync::{Arc, Mutex, MutexGuard};

use pyo3::pyfunction;
use rayon::prelude::*;

use crate::{
    instrs::{Color, CompProg, Instr, Params, Parse as _, Slot, State},
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

        tape.step(shift, color, same);

        if tape.blank() {
            return Err(());
        }

        state = next_state;
    }

    Ok(None)
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

    let Ok(Some(slot)) =
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

fn build_tree(
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
        access(&progs).push(comp.show(Some(params)));
    });

    get_val(progs)
}

/**************************************/

#[cfg(test)]
use {
    crate::{
        blocks::opt_block,
        graph::is_connected,
        machine::{quick_term_or_rec, run_for_infrul, run_prover},
        macros::{make_backsymbol_macro, make_block_macro},
        reason::{cant_blank, cant_halt, cant_spin_out},
        segment::{
            segment_cant_blank, segment_cant_halt,
            segment_cant_spin_out,
        },
    },
    std::collections::BTreeSet as Set,
};

#[cfg(test)]
fn incomplete(comp: &CompProg, params: Params, halt: bool) -> bool {
    let (states, colors) = params;

    let dimension = (states * colors) as usize;

    if comp.len() < (if halt { dimension - 1 } else { dimension }) {
        return true;
    }

    let (used_states, used_colors): (Set<State>, Set<Color>) =
        comp.values().map(|(pr, _, tr)| (tr, pr)).unzip();

    (colors == 2 && !used_colors.contains(&0))
        || (0..states).any(|state| !used_states.contains(&state))
        || (1..colors).any(|color| !used_colors.contains(&color))
}

#[cfg(test)]
fn check_inf(
    comp: &CompProg,
    params: Params,
    blocks: usize,
    steps: Step,
) -> bool {
    (if blocks == 1 {
        run_for_infrul(comp, steps)
    } else {
        run_for_infrul(&make_block_macro(comp, params, blocks), steps)
    }) || run_for_infrul(&make_backsymbol_macro(comp, params, 1), steps)
}

#[cfg(test)]
fn skip_all(comp: &CompProg, params: Params, halt: bool) -> bool {
    let (states, _) = params;

    let cant_reach = if halt { cant_halt } else { cant_spin_out };
    let segment_cant_reach = if halt {
        segment_cant_halt
    } else {
        segment_cant_spin_out
    };

    incomplete(comp, params, halt)
        || (states >= 4 && !is_connected(comp, states))
        || cant_reach(comp, 1).is_settled()
        || quick_term_or_rec(comp, 301, true)
        || cant_reach(comp, 256).is_settled()
        || check_inf(comp, params, opt_block(comp, 300), 306)
        || segment_cant_reach(comp, params, 3).is_refuted()
}

#[test]
fn test_skip() {
    let progs = [];

    let halt = 0;
    let params = (5, 2);

    let halt = halt != 0;

    for prog in progs {
        let comp = CompProg::from_str(prog);

        println!("{}", comp.show(Some(params)));

        assert!(skip_all(&comp, params, halt));
    }
}

/**************************************/

#[cfg(test)]
fn assert_tree(params: Params, halt: u8, expected: (u64, u64)) {
    let halt_flag = halt != 0;

    let holdout_count = set_val(0);
    let visited_count = set_val(0);

    build_tree(params, halt_flag, 876, &|prog| {
        *access(&visited_count) += 1;

        if skip_all(prog, params, halt_flag) {
            return;
        }

        *access(&holdout_count) += 1;

        // println!("{}", prog.show(Some(params)));
    });

    let result = (get_val(holdout_count), get_val(visited_count));

    assert_eq!(result, expected, "({params:?}, {halt}, {result:?})");
}

#[cfg(test)]
macro_rules! assert_trees {
    ( $( ( $params:expr, $halt:expr, $leaves:expr ) ),* $(,)? ) => {
        vec![$( ($params, $halt, $leaves) ),*]
            .par_iter().for_each(|&(params, halt, expected)| {
                assert_tree(params, halt, expected);
            });
    };
}

#[test]
fn test_tree() {
    assert_trees![
        ((2, 2), 1, (0, 36)),
        ((2, 2), 0, (0, 106)),
        //
        ((3, 2), 1, (0, 3_140)),
        ((3, 2), 0, (0, 13_128)),
        //
        ((2, 3), 1, (0, 2_447)),
        ((2, 3), 0, (11, 9_168)),
        //
        ((4, 2), 1, (116, 467_142)),
        ((4, 2), 0, (851, 2_291_637)),
        //
        ((2, 4), 1, (549, 312_642)),
        ((2, 4), 0, (7_661, 1_719_357)),
    ];
}

#[test]
#[ignore]
fn test_tree_slow() {
    assert_trees![
        ((5, 2), 1, (105_297, 95_310_282)),
        // ((5, 2), 0, (529_204?, 534_813_722)),
        //
        ((2, 5), 1, (596_533, 70_032_629)),
        // ((2, 5), 0, (5_826_162?, 515_255_468)),
        //
        ((3, 3), 1, (70_104, 25_306_290)),
        ((3, 3), 0, (531_368, 149_378_138)),
    ];
}

/**************************************/

#[cfg(test)]
fn assert_reason(params: Params, halt: u8, expected: (u64, u64)) {
    let halt_flag = halt != 0;

    let holdout_count = set_val(0);
    let visited_count = set_val(0);

    let cant_reach = if halt_flag { cant_halt } else { cant_spin_out };

    build_tree(params, halt_flag, 300, &|prog| {
        *access(&visited_count) += 1;

        if cant_reach(prog, 256).is_settled() {
            return;
        }

        *access(&holdout_count) += 1;

        // println!("{}", prog.show(Some(params)));
    });

    let result = (get_val(holdout_count), get_val(visited_count));

    assert_eq!(result, expected, "({params:?}, {halt}, {result:?})");
}

#[cfg(test)]
macro_rules! assert_reason_results {
    ( $( ( $params:expr, $halt:expr, $leaves:expr ) ),* $(,)? ) => {
        vec![$( ($params, $halt, $leaves) ),*]
            .par_iter().for_each(|&(params, halt, expected)| {
                assert_reason(params, halt, expected);
            });
    };
}

#[test]
fn test_reason() {
    assert_reason_results![
        ((2, 2), 1, (18, 36)),
        ((2, 2), 0, (13, 106)),
        //
        ((3, 2), 1, (1_886, 3_140)),
        ((3, 2), 0, (1_502, 13_128)),
        //
        ((2, 3), 1, (2_307, 2_447)),
        ((2, 3), 0, (1_605, 9_168)),
        //
        ((4, 2), 1, (289_361, 467_142)),
        ((4, 2), 0, (274_315, 2_291_637)),
        //
        ((2, 4), 1, (310_597, 312_642)),
        ((2, 4), 0, (406_828, 1_719_237)),
    ];
}

#[test]
#[ignore]
fn test_reason_slow() {
    assert_reason_results![
        ((5, 2), 1, (59_262_312, 95_310_168)),
        ((5, 2), 0, (66_415_144, 534_798_275)),
        //
        ((2, 5), 1, (69_848_916, 70_028_531)),
        ((2, 5), 0, (137_507_422, 515_051_756)),
        //
        ((3, 3), 1, (24_341_612, 25_306_222)),
        ((3, 3), 0, (28_534_586, 149_365_898)),
    ];
}

/**************************************/

#[cfg(test)]
fn assert_linrec(params: Params, halt: u8, expected: (u64, u64)) {
    let halt_flag = halt != 0;

    let holdout_count = set_val(0);
    let visited_count = set_val(0);

    build_tree(params, halt_flag, 300, &|prog| {
        *access(&visited_count) += 1;

        if quick_term_or_rec(prog, 1_000, halt_flag) {
            return;
        }

        *access(&holdout_count) += 1;

        // println!("{}", prog.show(Some(params)));
    });

    let result = (get_val(holdout_count), get_val(visited_count));

    assert_eq!(result, expected, "({params:?}, {halt}, {result:?})");
}

#[cfg(test)]
macro_rules! assert_linrec_results {
    ( $( ( $params:expr, $halt:expr, $leaves:expr ) ),* $(,)? ) => {
        vec![$( ($params, $halt, $leaves) ),*]
            .par_iter().for_each(|&(params, halt, expected)| {
                assert_linrec(params, halt, expected);
            });
    };
}

#[test]
fn test_linrec() {
    assert_linrec_results![
        ((2, 2), 1, (0, 36)),
        ((2, 2), 0, (4, 106)),
        //
        ((3, 2), 1, (58, 3_140)),
        ((3, 2), 0, (724, 13_128)),
        //
        ((2, 3), 1, (134, 2_447)),
        ((2, 3), 0, (1_042, 9_168)),
        //
        ((4, 2), 1, (12_383, 467_142)),
        ((4, 2), 0, (146_198, 2_291_637)),
        //
        ((2, 4), 1, (25_152, 312_642)),
        ((2, 4), 0, (258_834, 1_719_237)),
    ];
}

/**************************************/

#[cfg(test)]
fn assert_segment(params: Params, halt: u8, expected: (u64, u64)) {
    let halt_flag = halt != 0;

    let holdout_count = set_val(0);
    let visited_count = set_val(0);

    let cant_reach = if halt_flag {
        segment_cant_halt
    } else {
        segment_cant_spin_out
    };

    let segs = if halt_flag { 22 } else { 8 };

    build_tree(params, halt_flag, 300, &|prog| {
        *access(&visited_count) += 1;

        if cant_reach(prog, params, segs).is_refuted() {
            return;
        }

        *access(&holdout_count) += 1;

        // println!("{}", prog.show(Some(params)));
    });

    let result = (get_val(holdout_count), get_val(visited_count));

    assert_eq!(result, expected, "({params:?}, {halt}, {result:?})");
}

#[cfg(test)]
macro_rules! assert_segment_results {
    ( $( ( $params:expr, $halt:expr, $leaves:expr ) ),* $(,)? ) => {
        vec![$( ($params, $halt, $leaves) ),*]
            .par_iter().for_each(|&(params, halt, expected)| {
                assert_segment(params, halt, expected);
            });
    };
}

#[test]
fn test_segment() {
    assert_segment_results![
        ((2, 2), 1, (21, 36)),
        ((2, 2), 0, (28, 106)),
        //
        ((3, 2), 1, (1_245, 3_140)),
        ((3, 2), 0, (2_045, 13_128)),
        //
        ((2, 3), 1, (892, 2_447)),
        ((2, 3), 0, (1_396, 9_168)),
    ];
}

#[test]
#[ignore]
fn test_segment_slow() {
    assert_segment_results![
        ((4, 2), 1, (161_731, 467_142)),
        ((4, 2), 0, (280_689, 2_291_637)),
        //
        ((2, 4), 1, (130_046, 312_642)),
        ((2, 4), 0, (202_779, 1_719_237)),
    ];
}

/**************************************/

#[cfg(test)]
fn assert_blank(params: Params, expected: (u64, u64)) {
    let holdout_count = set_val(0);
    let visited_count = set_val(0);

    build_tree(params, false, 300, &|prog| {
        *access(&visited_count) += 1;

        let run = 700;

        if cant_blank(prog, 44).is_settled()
            || quick_term_or_rec(prog, run, true)
            || check_inf(prog, params, opt_block(prog, 300), run as u64)
            || !run_prover(&prog.show(Some(params)), run as u64)
                .blanks
                .is_empty()
            || segment_cant_blank(prog, params, 11).is_refuted()
        {
            return;
        }

        *access(&holdout_count) += 1;

        // println!("{}", prog.show(Some(params)));
    });

    let result = (get_val(holdout_count), get_val(visited_count));

    assert_eq!(result, expected, "({params:?}, {result:?})");
}

#[cfg(test)]
macro_rules! assert_blank_results {
    ( $( ( $params:expr, $leaves:expr ) ),* $(,)? ) => {
        vec![$( ($params, $leaves) ),*]
            .par_iter().for_each(|&(params, expected)| {
                assert_blank(params, expected);
            });
    };
}

#[test]
fn test_blank() {
    assert_blank_results![
        ((2, 2), (0, 106)),
        //
        ((3, 2), (3, 13_128)),
        //
        ((2, 3), (35, 9_168)),
        //
        ((4, 2), (2_208, 2_291_637)),
        //
        ((2, 4), (14_815, 1_719_237)),
    ];
}
