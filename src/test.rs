use rayon::prelude::*;

use crate::{
    instrs::{Color, CompProg, Params, Parse as _, State},
    tree::{access, build_tree, get_val, set_val, Step},
};

use crate::{
    blocks::opt_block,
    graph::is_connected,
    machine::{quick_term_or_rec, run_for_infrul, run_prover},
    macros::{make_backsymbol_macro, make_block_macro},
    reason::{cant_blank, cant_halt, cant_spin_out},
    segment::{segment_cant_halt, segment_cant_spin_out},
};
use std::collections::BTreeSet as Set;

/**************************************/

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
        {
            return;
        }

        *access(&holdout_count) += 1;

        // println!("{}", prog.show(Some(params)));
    });

    let result = (get_val(holdout_count), get_val(visited_count));

    assert_eq!(result, expected, "({params:?}, {result:?})");
}

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
