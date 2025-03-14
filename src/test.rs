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

const TREE_LIM: u64 = 876;

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
        || quick_term_or_rec(comp, 600).is_settled()
        || cant_reach(comp, 256).is_settled()
        || check_inf(comp, params, opt_block(comp, 300), 306)
        || segment_cant_reach(comp, params, 3).is_refuted()
}

/**************************************/

fn assert_tree(params: Params, halt: u8, expected: (u64, u64)) {
    let halt_flag = halt != 0;

    let holdout_count = set_val(0);
    let visited_count = set_val(0);

    build_tree(params, halt_flag, TREE_LIM, &|prog| {
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
        ((2, 3), 0, (9, 9_168)),
        //
        ((4, 2), 1, (114, 467_142)),
        ((4, 2), 0, (637, 2_291_637)),
        //
        ((2, 4), 1, (540, 312_642)),
        ((2, 4), 0, (6_885, 1_719_357)),
    ];
}

// #[test]
// #[ignore]
// fn test_tree_slow() {
//     assert_trees![
//         ((3, 3), 1, (68_085, 25_306_290)),
//         ((3, 3), 0, (482_973, 149_378_138)),
//         //
//         ((5, 2), 1, (101_619, 95_310_282)),
//         // ((5, 2), 0, (477_315?, 534_813_722)),
//         //
//         ((2, 5), 1, (572_681, 70_032_629)),
//         // ((2, 5), 0, (5_198_482?, 515_255_468)),
//     ];
// }

/**************************************/

use crate::reason::BackwardResult;

fn assert_reason(
    params: Params,
    halt: u8,
    expected: (usize, (u64, u64)),
) {
    let halt_flag = halt != 0;

    let (max_refuted, _) = expected;

    let holdout_count = set_val(0);
    let visited_count = set_val(0);
    let refuted_steps = set_val(0);

    let cant_reach = if halt_flag { cant_halt } else { cant_spin_out };

    build_tree(params, halt_flag, 300, &|prog| {
        *access(&visited_count) += 1;

        let result = cant_reach(prog, 256);

        if let BackwardResult::Refuted(steps) = result {
            if steps > max_refuted {
                println!(
                    "        \"{}\": {},",
                    prog.show(Some(params)),
                    1 + steps,
                );
            }

            if steps > *access(&refuted_steps) {
                *access(&refuted_steps) = steps;
            }
        }

        if result.is_settled() {
            return;
        }

        *access(&holdout_count) += 1;

        // println!("{}", prog.show(Some(params)));
    });

    let result = (
        get_val(refuted_steps),
        (get_val(holdout_count), get_val(visited_count)),
    );

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
        ((2, 2), 1, (2, (10, 36))),
        ((2, 2), 0, (1, (5, 106))),
        //
        ((3, 2), 1, (12, (1_506, 3_140))),
        ((3, 2), 0, (6, (1_157, 13_128))),
        //
        ((2, 3), 1, (7, (2_132, 2_447))),
        ((2, 3), 0, (6, (1_437, 9_168))),
        //
        ((4, 2), 1, (45, (262_616, 467_142))),
        ((4, 2), 0, (35, (242_617, 2_291_637))),
        //
        ((2, 4), 1, (16, (306_455, 312_642))),
        ((2, 4), 0, (12, (387_520, 1_719_237))),
    ];
}

// #[test]
// #[ignore]
// fn test_reason_slow() {
//     assert_reason_results![
//         ((3, 3), 1, (53, (24_147_816, 25_306_222))),
//         ((3, 3), 0, (124, (28_300_714, 149_365_898))),
//         //
//         ((5, 2), 1, (114, (57_361_209, 95_310_168))),
//         ((5, 2), 0, (255, (64_386_064, 534_798_275))),
//         //
//         ((2, 5), 1, (28, (69_846_877, 70_028_531))),
//         ((2, 5), 0, (55, (137_504_028, 515_051_756))),
//     ];
// }

/**************************************/

fn assert_linrec(params: Params, halt: u8, expected: (u64, u64)) {
    let halt_flag = halt != 0;

    let holdout_count = set_val(0);
    let visited_count = set_val(0);

    build_tree(params, halt_flag, TREE_LIM, &|prog| {
        *access(&visited_count) += 1;

        let result = quick_term_or_rec(prog, LINREC);

        if if halt_flag {
            result.is_settled()
        } else {
            result.is_recur()
        } {
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

const LINREC: usize = 20_000;

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
        ((4, 2), 0, (145_978, 2_291_637)),
        //
        ((2, 4), 1, (25_134, 312_642)),
        ((2, 4), 0, (257_617, 1_719_357)),
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

    build_tree(params, halt_flag, TREE_LIM, &|prog| {
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
        ((2, 4), 0, (202_795, 1_719_357)),
    ];
}

/**************************************/

fn assert_blank(params: Params, expected: (usize, (u64, u64))) {
    let (max_refuted, _) = expected;

    let holdout_count = set_val(0);
    let visited_count = set_val(0);
    let refuted_steps = set_val(0);

    build_tree(params, false, TREE_LIM, &|prog| {
        *access(&visited_count) += 1;

        let run = 700;

        let backward = cant_blank(prog, 44);

        if let BackwardResult::Refuted(steps) = backward {
            if steps > max_refuted {
                println!(
                    "        \"{}\": {},",
                    prog.show(Some(params)),
                    1 + steps,
                );
            }

            if steps > *access(&refuted_steps) {
                *access(&refuted_steps) = steps;
            }
        }

        if backward.is_settled()
            || quick_term_or_rec(prog, run).is_settled()
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

    let result = (
        get_val(refuted_steps),
        (get_val(holdout_count), get_val(visited_count)),
    );

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
        ((2, 2), (1, (0, 106))),
        //
        ((3, 2), (13, (0, 13_128))),
        //
        ((2, 3), (15, (14, 9_168))),
        //
        ((4, 2), (43, (837, 2_291_637))),
        //
        ((2, 4), (40, (8_243, 1_719_357))),
    ];
}
