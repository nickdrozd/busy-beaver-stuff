use rayon::prelude::*;

use crate::{
    blocks::opt_block,
    cps::Cps as _,
    ctl::Ctl as _,
    graph::is_connected,
    instrs::{CompProg, GetInstr as _, Params, Parse as _},
    machine::{quick_term_or_rec, run_for_infrul},
    macros::{make_backsymbol_macro, make_block_macro},
    reason::Backward as _,
    segment::Segment as _,
    tree::{access, build_tree, get_val, set_val, Step},
};

/**************************************/

const TREE_LIM: Step = 876;

/**************************************/

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
    ( $( ( ($params:expr, $halt:expr, $tree:expr, $leaves:expr), $pipeline:expr ) ),* $(,)? ) => {
        vec![
            $( (
                ($params, $halt, $tree, $leaves),
                Box::new($pipeline) as Box<dyn Fn(&CompProg, Params) -> bool + Sync>
            ) ),*]
            .par_iter()
            .for_each(|&((params, halt, tree, expected), ref pipeline)| {
                assert_tree(params, halt, tree, expected, pipeline);
            });
    };
}

fn assert_tree(
    params: Params,
    halt: u8,
    tree: Step,
    expected: (u64, u64),
    pipeline: impl Fn(&CompProg, Params) -> bool + Sync,
) {
    let halt_flag = halt != 0;

    let holdout_count = set_val(0);
    let visited_count = set_val(0);

    // if (params, halt) != ((5, 2), 0) {
    //     return;
    // }

    build_tree(params, halt_flag, tree, &|prog| {
        *access(&visited_count) += 1;

        if prog.incomplete(params, halt_flag) {
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
            ((2, 2), 1, 2, (0, 36)),
            //
            |prog: &CompProg, _: Params| {
                quick_term_or_rec(prog, 8).is_settled()
            }
        ),
        (
            ((2, 2), 0, 4, (0, 106)),
            //
            |prog: &CompProg, _: Params| {
                prog.cant_spin_out(0).is_settled()
                    || quick_term_or_rec(prog, 16).is_settled()
            }
        ),
        (
            ((3, 2), 1, 12, (0, 3_140)),
            //
            |prog: &CompProg, _: Params| {
                quick_term_or_rec(prog, 40).is_settled()
                    || prog.cant_halt(3).is_settled()
                    || prog.ctl_cant_halt(16)
                    || run_for_infrul(prog, 187)
            }
        ),
        (
            ((3, 2), 0, 13, (0, 13_128)),
            //
            |prog: &CompProg, params: Params| {
                quick_term_or_rec(prog, 206).is_settled()
                    || prog.cant_spin_out(4).is_settled()
                    || prog.ctl_cant_spin_out(15)
                    || run_for_infrul(prog, 236)
                    || check_inf(prog, params, 2, 40)
            }
        ),
        (
            ((2, 3), 1, 7, (0, 2_447)),
            //
            |prog: &CompProg, _: Params| {
                quick_term_or_rec(prog, 301).is_settled()
                    || prog.ctl_cant_halt(41)
                    || prog.cps_cant_halt(3)
                    || run_for_infrul(prog, 159)
            }
        ),
        (
            ((2, 3), 0, 20, (3, 9_168)),
            //
            |prog: &CompProg, params: Params| {
                quick_term_or_rec(prog, 301).is_settled()
                    || prog.cant_spin_out(2).is_settled()
                    || prog.ctl_cant_spin_out(83)
                    || prog.seg_cant_spin_out(params, 5).is_refuted()
                    || run_for_infrul(prog, 285)
            }
        ),
        (
            ((4, 2), 1, 25, (21, 467_142)),
            //
            |prog: &CompProg, _: Params| {
                !is_connected(prog, 4)
                    || quick_term_or_rec(prog, 200).is_settled()
                    || prog.cant_halt(11).is_settled()
                    || prog.ctl_cant_halt(84)
                    || prog.cps_cant_halt(9)
                    || run_for_infrul(prog, 895)
            }
        ),
        (
            ((4, 2), 0, 99, (251, 2_291_637)),
            //
            |prog: &CompProg, params: Params| {
                !is_connected(prog, 4)
                    || prog.cant_spin_out(1).is_settled()
                    || quick_term_or_rec(prog, 1_000).is_settled()
                    || prog.cant_spin_out(11).is_settled()
                    || prog.ctl_cant_spin_out(114)
                    || prog.cps_cant_spin_out(9)
                    || prog.seg_cant_spin_out(params, 7).is_refuted()
                    || run_for_infrul(prog, 1_000)
            }
        ),
        (
            ((2, 4), 1, 109, (71, 312_642)),
            //
            |prog: &CompProg, prms: Params| {
                prog.cant_halt(0).is_settled()
                    || quick_term_or_rec(prog, 800).is_settled()
                    || prog.seg_cant_halt(prms, 2).is_refuted()
                    || prog.ctl_cant_halt(96)
                    || prog.cps_cant_halt(6)
                    || check_inf(prog, prms, opt_block(prog, 300), 500)
            }
        ),
        (
            ((2, 4), 0, 876, (1_803, 1_719_357)),
            //
            |prog: &CompProg, prms: Params| {
                prog.cant_spin_out(2).is_settled()
                    || quick_term_or_rec(prog, 2_000).is_settled()
                    || prog.cant_spin_out(7).is_settled()
                    || prog.ctl_cant_spin_out(186)
                    || prog.cps_cant_spin_out(6)
                    || prog.seg_cant_spin_out(prms, 4).is_refuted()
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
            ((3, 3), 1, 2_700, (9_092, 25_306_375)),
            //
            |prog: &CompProg, prms: Params| {
                prog.cant_halt(1).is_settled()
                    || quick_term_or_rec(prog, 1_200).is_settled()
                    || prog.cant_halt(9).is_settled()
                    || prog.ctl_cant_halt(200)
                    || prog.cps_cant_halt(7)
                    || check_inf(prog, prms, opt_block(prog, 300), 500)
            }
        ),
        (
            ((3, 3), 0, 3_000, (94_945, 149_382_609)),
            //
            |prog: &CompProg, prms: Params| {
                prog.cant_spin_out(1).is_settled()
                    || quick_term_or_rec(prog, 2_000).is_settled()
                    || prog.cant_spin_out(73).is_settled()
                    || prog.ctl_cant_spin_out(200)
                    || prog.seg_cant_spin_out(prms, 5).is_refuted()
                    || check_inf(prog, prms, opt_block(prog, 300), 500)
                    || prog.cps_cant_spin_out(8)
            }
        ),
        (
            ((5, 2), 1, 700, (12_643, 95_310_282)),
            //
            |prog: &CompProg, prms: Params| {
                !is_connected(prog, 5)
                    || prog.cant_halt(1).is_settled()
                    || quick_term_or_rec(prog, 1_000).is_settled()
                    || prog.cant_halt(44).is_settled()
                    || prog.ctl_cant_halt(200)
                    || prog.cps_cant_halt(7)
                    || check_inf(prog, prms, opt_block(prog, 300), 500)
            }
        ),
        (
            ((5, 2), 0, TREE_LIM, (114_452, 534_813_722)),
            //
            |prog: &CompProg, prms: Params| {
                !is_connected(prog, 5)
                    || prog.cant_spin_out(2).is_settled()
                    || quick_term_or_rec(prog, 3_000).is_settled()
                    || prog.cant_spin_out(30).is_settled()
                    || prog.ctl_cant_spin_out(200)
                    || prog.cps_cant_spin_out(5)
                    || prog.seg_cant_spin_out(prms, 5).is_refuted()
                    || check_inf(prog, prms, opt_block(prog, 300), 500)
            }
        ),
        (
            ((2, 5), 1, TREE_LIM, (81_460, 70_032_629)),
            //
            |prog: &CompProg, prms: Params| {
                prog.cant_halt(1).is_settled()
                    || quick_term_or_rec(prog, 3_000).is_settled()
                    || prog.seg_cant_halt(prms, 5).is_refuted()
                    || prog.ctl_cant_halt(200)
                    || prog.cps_cant_halt(5)
                    || check_inf(prog, prms, opt_block(prog, 300), 500)
            }
        ),
        (
            ((2, 5), 0, TREE_LIM, (1_248_156, 515_255_468)),
            //
            |prog: &CompProg, prms: Params| {
                prog.cant_spin_out(1).is_settled()
                    || quick_term_or_rec(prog, 3_000).is_settled()
                    || prog.cant_spin_out(20).is_settled()
                    || prog.ctl_cant_spin_out(200)
                    || prog.seg_cant_spin_out(prms, 5).is_refuted()
                    || prog.cps_cant_spin_out(5)
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

    let cant_reach = if halt_flag {
        CompProg::cant_halt
    } else {
        CompProg::cant_spin_out
    };

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

fn assert_blank(params: Params, expected: (usize, (u64, u64))) {
    let (max_refuted, _) = expected;

    let holdout_count = set_val(0);
    let visited_count = set_val(0);
    let refuted_steps = set_val(0);

    build_tree(params, false, TREE_LIM, &|prog| {
        *access(&visited_count) += 1;

        let run = 700;

        let backward = prog.cant_blank(44);

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
            || check_inf(prog, params, opt_block(prog, 300), run)
            || prog.ctl_cant_blank(100)
            || prog.cps_cant_blank(5)
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
        ((4, 2), (43, (631, 2_291_637))),
        //
        ((2, 4), (40, (2_084, 1_719_357))),
    ];
}
