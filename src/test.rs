use rayon::prelude::*;

use crate::{
    instrs::{Color, CompProg, Params, Parse as _, State},
    tree::{access, build_tree, get_val, set_val, Step},
};

use crate::{
    blocks::opt_block,
    cps::{cps_cant_blank, cps_cant_halt, cps_cant_spin_out},
    graph::is_connected,
    machine::{quick_term_or_rec, run_for_infrul, run_prover},
    macros::{make_backsymbol_macro, make_block_macro},
    reason::{cant_blank, cant_halt, cant_spin_out},
    segment::{
        segment_cant_halt as seg_cant_halt,
        segment_cant_spin_out as seg_cant_spin_out,
    },
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

/**************************************/

macro_rules! assert_trees {
    ( $( ( ($params:expr, $halt:expr, $leaves:expr), $pipeline:expr ) ),* $(,)? ) => {
        vec![
            $( (
                ($params, $halt, $leaves),
                Box::new($pipeline) as Box<dyn Fn(&CompProg, Params) -> bool + Sync>
            ) ),*]
            .par_iter()
            .for_each(|&((params, halt, expected), ref pipeline)| {
                assert_tree(params, halt, expected, pipeline);
            });
    };
}

fn assert_tree(
    params: Params,
    halt: u8,
    expected: (u64, u64),
    pipeline: impl Fn(&CompProg, Params) -> bool + Sync,
) {
    let halt_flag = halt != 0;

    let holdout_count = set_val(0);
    let visited_count = set_val(0);

    // if (params, halt) != ((5, 2), 0) {
    //     return;
    // }

    build_tree(params, halt_flag, TREE_LIM, &|prog| {
        *access(&visited_count) += 1;

        if incomplete(prog, params, halt_flag) {
            return;
        }

        if pipeline(prog, params) {
            return;
        }

        *access(&holdout_count) += 1;

        // println!("{}", prog.show(Some(params)));
    });

    let result = (get_val(holdout_count), get_val(visited_count));

    assert_eq!(result, expected, "({params:?}, {halt}, {result:?})");
}

#[test]
fn test_tree() {
    assert_trees![
        (
            ((2, 2), 1, (0, 36)),
            //
            |prog: &CompProg, _: Params| {
                quick_term_or_rec(prog, 8).is_settled()
            }
        ),
        (
            ((2, 2), 0, (0, 106)),
            //
            |prog: &CompProg, _: Params| {
                cant_spin_out(prog, 0).is_settled()
                    || quick_term_or_rec(prog, 16).is_settled()
            }
        ),
        (
            ((3, 2), 1, (0, 3_140)),
            //
            |prog: &CompProg, _: Params| {
                quick_term_or_rec(prog, 40).is_settled()
                    || cant_halt(prog, 3).is_settled()
                    || cps_cant_halt(prog, 4)
                    || run_for_infrul(prog, 187)
            }
        ),
        (
            ((3, 2), 0, (0, 13_128)),
            //
            |prog: &CompProg, params: Params| {
                quick_term_or_rec(prog, 206).is_settled()
                    || cant_spin_out(prog, 4).is_settled()
                    || cps_cant_spin_out(prog, 4)
                    || run_for_infrul(prog, 236)
                    || check_inf(prog, params, 2, 40)
            }
        ),
        (
            ((2, 3), 1, (0, 2_447)),
            //
            |prog: &CompProg, _: Params| {
                quick_term_or_rec(prog, 301).is_settled()
                    || cant_halt(prog, 1).is_settled()
                    || cps_cant_halt(prog, 4)
                    || run_for_infrul(prog, 159)
            }
        ),
        (
            ((2, 3), 0, (6, 9_168)),
            //
            |prog: &CompProg, params: Params| {
                quick_term_or_rec(prog, 301).is_settled()
                    || cant_spin_out(prog, 2).is_settled()
                    || cps_cant_spin_out(prog, 5)
                    || run_for_infrul(prog, 474)
                    || seg_cant_spin_out(prog, params, 6).is_refuted()
            }
        ),
        (
            ((4, 2), 1, (26, 467_142)),
            //
            |prog: &CompProg, _: Params| {
                !is_connected(prog, 4)
                    || quick_term_or_rec(prog, 890).is_settled()
                    || cant_halt(prog, 11).is_settled()
                    || cps_cant_halt(prog, 9)
                    || run_for_infrul(prog, 895)
            }
        ),
        (
            ((4, 2), 0, (390, 2_291_637)),
            //
            |prog: &CompProg, params: Params| {
                !is_connected(prog, 4)
                    || cant_spin_out(prog, 1).is_settled()
                    || quick_term_or_rec(prog, 1_000).is_settled()
                    || cant_spin_out(prog, 11).is_settled()
                    || cps_cant_spin_out(prog, 9)
                    || seg_cant_spin_out(prog, params, 7).is_refuted()
                    || run_for_infrul(prog, 1_000)
            }
        ),
        (
            ((2, 4), 1, (72, 312_642)),
            //
            |prog: &CompProg, _: Params| {
                cant_halt(prog, 0).is_settled()
                    || quick_term_or_rec(prog, 1_000).is_settled()
                    || cps_cant_halt(prog, 8)
                    || run_for_infrul(prog, 1_000)
            }
        ),
        (
            ((2, 4), 0, (2_799, 1_719_357)),
            //
            |prog: &CompProg, prms: Params| {
                cant_spin_out(prog, 2).is_settled()
                    || quick_term_or_rec(prog, 2_000).is_settled()
                    || cant_spin_out(prog, 7).is_settled()
                    || seg_cant_spin_out(prog, prms, 4).is_refuted()
                    || cps_cant_spin_out(prog, 4)
                    || check_inf(prog, prms, opt_block(prog, 300), 500)
            }
        ),
    ];
}

#[test]
#[ignore]
fn test_tree_slow() {
    assert_trees![
        (
            ((3, 3), 1, (11_011, 25_306_290)),
            //
            |prog: &CompProg, prms: Params| {
                cant_halt(prog, 1).is_settled()
                    || quick_term_or_rec(prog, 1_200).is_settled()
                    || cant_halt(prog, 9).is_settled()
                    || cps_cant_halt(prog, 7)
                    || check_inf(prog, prms, opt_block(prog, 300), 500)
            }
        ),
        (
            ((3, 3), 0, (148_022, 149_378_138)),
            //
            |prog: &CompProg, prms: Params| {
                cant_spin_out(prog, 1).is_settled()
                    || quick_term_or_rec(prog, 2_000).is_settled()
                    || cant_spin_out(prog, 73).is_settled()
                    || seg_cant_spin_out(prog, prms, 5).is_refuted()
                    || check_inf(prog, prms, opt_block(prog, 300), 500)
                    || cps_cant_spin_out(prog, 8)
            }
        ),
        (
            ((5, 2), 1, (17_465, 95_310_282)),
            //
            |prog: &CompProg, prms: Params| {
                !is_connected(prog, 5)
                    || cant_halt(prog, 1).is_settled()
                    || quick_term_or_rec(prog, 1_000).is_settled()
                    || cant_halt(prog, 44).is_settled()
                    || cps_cant_halt(prog, 7)
                    || check_inf(prog, prms, opt_block(prog, 300), 500)
            }
        ),
        (
            ((5, 2), 0, (174_050, 534_813_722)),
            //
            |prog: &CompProg, prms: Params| {
                !is_connected(prog, 5)
                    || cant_spin_out(prog, 2).is_settled()
                    || quick_term_or_rec(prog, 3_000).is_settled()
                    || cant_spin_out(prog, 30).is_settled()
                    || cps_cant_spin_out(prog, 5)
                    || seg_cant_spin_out(prog, prms, 5).is_refuted()
                    || check_inf(prog, prms, opt_block(prog, 300), 500)
            }
        ),
        (
            ((2, 5), 1, (106_319, 70_032_629)),
            //
            |prog: &CompProg, prms: Params| {
                cant_halt(prog, 1).is_settled()
                    || quick_term_or_rec(prog, 3_000).is_settled()
                    || seg_cant_halt(prog, prms, 5).is_refuted()
                    || cps_cant_halt(prog, 5)
                    || check_inf(prog, prms, opt_block(prog, 300), 500)
            }
        ),
        (
            ((2, 5), 0, (1_855_227, 515_255_468)),
            //
            |prog: &CompProg, prms: Params| {
                cant_spin_out(prog, 1).is_settled()
                    || quick_term_or_rec(prog, 3_000).is_settled()
                    || cant_spin_out(prog, 20).is_settled()
                    || seg_cant_spin_out(prog, prms, 5).is_refuted()
                    || cps_cant_spin_out(prog, 5)
                    || check_inf(prog, prms, opt_block(prog, 300), 500)
            }
        ),
    ];
}

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
        seg_cant_halt
    } else {
        seg_cant_spin_out
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
            || cps_cant_blank(prog, 5)
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
        ((2, 3), (15, (8, 9_168))),
        //
        ((4, 2), (43, (628, 2_291_637))),
        //
        ((2, 4), (40, (2_088, 1_719_357))),
    ];
}
