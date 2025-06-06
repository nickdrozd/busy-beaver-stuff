use std::{
    env,
    fmt::Debug,
    sync::{Arc, Mutex, MutexGuard},
};

use rayon::prelude::*;

use tm::{
    blocks::opt_block,
    cps::Cps as _,
    ctl::Ctl as _,
    graph::is_connected,
    instrs::{GetInstr as _, Params, Prog},
    machine::{quick_term_or_rec, run_for_infrul},
    macros::{make_backsymbol_macro, make_block_macro},
    reason::Backward as _,
    segment::Segment as _,
    tree::{build_tree, Step},
};

#[expect(unused_imports)]
use tm::instrs::Parse as _;

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

/**************************************/

const TREE_LIM: Step = 876;

/**************************************/

fn check_inf(
    comp: &Prog,
    params: Params,
    block_steps: usize,
    steps: Step,
) -> bool {
    let blocks = opt_block(comp, block_steps);

    (if blocks == 1 {
        run_for_infrul(comp, steps)
    } else {
        run_for_infrul(&make_block_macro(comp, params, blocks), steps)
    })
    .is_infinite()
        || run_for_infrul(
            &make_backsymbol_macro(comp, params, 1),
            steps,
        )
        .is_infinite()
}

/**************************************/

macro_rules! assert_trees {
    ( $( ( ($params:expr, $halt:expr, $tree:expr, $leaves:expr), $pipeline:expr ) ),* $(,)? ) => {
        vec![
            $( (
                ($params, $halt, $tree, $leaves),
                Box::new($pipeline) as Box<dyn Fn(&Prog, Params) -> bool + Sync>
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
    pipeline: impl Fn(&Prog, Params) -> bool + Sync,
) {
    let halt_flag = halt != 0;

    let holdout_count = set_val(0);
    let visited_count = set_val(0);

    // if (params, halt) != ((5, 2), 0) {
    //     return;
    // }

    build_tree(params, halt_flag, tree, &|prog| {
        *access(&visited_count) += 1;

        if pipeline(prog, params) {
            return;
        }

        *access(&holdout_count) += 1;

        // println!("{}", prog.show());
    });

    let result = (get_val(holdout_count), get_val(visited_count));

    assert_eq!(result, expected, "({params:?}, {halt}, {result:?})");
}

#[expect(clippy::too_many_lines)]
fn test_tree() {
    assert_trees![
        (
            ((2, 2), 1, 2, (0, 34)),
            //
            |prog: &Prog, _: Params| {
                quick_term_or_rec(prog, 8).is_settled()
            }
        ),
        (
            ((2, 2), 0, 4, (0, 91)),
            //
            |prog: &Prog, _: Params| {
                prog.cant_spin_out(0).is_settled()
                    || quick_term_or_rec(prog, 16).is_settled()
            }
        ),
        (
            ((3, 2), 1, 12, (0, 3_030)),
            //
            |prog: &Prog, _: Params| {
                quick_term_or_rec(prog, 40).is_settled()
                    || prog.cant_halt(3).is_settled()
                    || prog.ctl_cant_halt(16)
                    || run_for_infrul(prog, 187).is_infinite()
            }
        ),
        (
            ((3, 2), 0, 13, (0, 12_470)),
            //
            |prog: &Prog, params: Params| {
                quick_term_or_rec(prog, 206).is_settled()
                    || prog.cant_spin_out(4).is_settled()
                    || prog.ctl_cant_spin_out(15)
                    || run_for_infrul(prog, 236).is_infinite()
                    || check_inf(prog, params, 100, 40)
            }
        ),
        (
            ((2, 3), 1, 7, (0, 2_395)),
            //
            |prog: &Prog, _: Params| {
                quick_term_or_rec(prog, 301).is_settled()
                    || prog.ctl_cant_halt(41)
                    || prog.cps_cant_halt(3)
                    || run_for_infrul(prog, 159).is_infinite()
            }
        ),
        (
            ((2, 3), 0, 20, (2, 8_848)),
            //
            |prog: &Prog, params: Params| {
                quick_term_or_rec(prog, 301).is_settled()
                    || prog.cant_spin_out(2).is_settled()
                    || prog.ctl_cant_spin_out(83)
                    || prog.seg_cant_spin_out(params, 5).is_refuted()
                    || check_inf(prog, params, 300, 1000)
            }
        ),
        (
            ((4, 2), 1, 25, (13, 458_588)),
            //
            |prog: &Prog, params: Params| {
                !is_connected(prog, 4)
                    || quick_term_or_rec(prog, 200).is_settled()
                    || prog.cant_halt(11).is_settled()
                    || prog.ctl_cant_halt(84)
                    || prog.cps_cant_halt(9)
                    || check_inf(prog, params, 300, 1364)
            }
        ),
        (
            ((4, 2), 0, 99, (104, 2_222_970)),
            //
            |prog: &Prog, params: Params| {
                !is_connected(prog, 4)
                    || prog.cant_spin_out(1).is_settled()
                    || quick_term_or_rec(prog, 1_000).is_settled()
                    || prog.cant_spin_out(11).is_settled()
                    || prog.ctl_cant_spin_out(114)
                    || prog.cps_cant_spin_out(9)
                    || prog.seg_cant_spin_out(params, 7).is_refuted()
                    || check_inf(prog, params, 300, 7000)
            }
        ),
        (
            ((2, 4), 1, 109, (43, 310_211)),
            //
            |prog: &Prog, params: Params| {
                prog.cant_halt(0).is_settled()
                    || quick_term_or_rec(prog, 800).is_settled()
                    || prog.seg_cant_halt(params, 2).is_refuted()
                    || prog.ctl_cant_halt(96)
                    || prog.cps_cant_halt(6)
                    || check_inf(prog, params, 300, 7000)
            }
        ),
        (
            ((2, 4), 0, 876, (1_412, 1_698_539)),
            //
            |prog: &Prog, params: Params| {
                prog.cant_spin_out(2).is_settled()
                    || quick_term_or_rec(prog, 2_000).is_settled()
                    || prog.cant_spin_out(7).is_settled()
                    || prog.ctl_cant_spin_out(186)
                    || prog.cps_cant_spin_out(6)
                    || prog.seg_cant_spin_out(params, 4).is_refuted()
                    || check_inf(prog, params, 300, 1000)
                    || (prog.incomplete(params, false) && {
                        prog.seg_cant_halt(params, 5).is_refuted()
                            || prog.cps_cant_halt(6)
                    })
            }
        ),
    ];
}

fn test_tree_slow() {
    assert_trees![
        (
            ((3, 3), 1, 2_700, (9_449, 25_028_837)),
            //
            |prog: &Prog, params: Params| {
                prog.cant_halt(1).is_settled()
                    || quick_term_or_rec(prog, 1_200).is_settled()
                    || prog.cant_halt(9).is_settled()
                    || prog.ctl_cant_halt(200)
                    || prog.cps_cant_halt(7)
                    || check_inf(prog, params, 300, 500)
            }
        ),
        (
            ((3, 3), 0, 3_000, (98_497, 147_230_805)),
            //
            |prog: &Prog, params: Params| {
                prog.cant_spin_out(1).is_settled()
                    || quick_term_or_rec(prog, 2_000).is_settled()
                    || prog.cant_spin_out(73).is_settled()
                    || prog.ctl_cant_spin_out(200)
                    || prog.seg_cant_spin_out(params, 5).is_refuted()
                    || check_inf(prog, params, 300, 500)
                    || prog.cps_cant_spin_out(8)
            }
        ),
        (
            ((5, 2), 1, 700, (12_900, 94_160_306)),
            //
            |prog: &Prog, params: Params| {
                !is_connected(prog, 5)
                    || prog.cant_halt(1).is_settled()
                    || quick_term_or_rec(prog, 1_000).is_settled()
                    || prog.cant_halt(44).is_settled()
                    || prog.ctl_cant_halt(200)
                    || prog.cps_cant_halt(7)
                    || check_inf(prog, params, 300, 500)
            }
        ),
        (
            ((5, 2), 0, TREE_LIM, (117_874, 523_722_375)),
            //
            |prog: &Prog, params: Params| {
                !is_connected(prog, 5)
                    || prog.cant_spin_out(2).is_settled()
                    || quick_term_or_rec(prog, 3_000).is_settled()
                    || prog.cant_spin_out(30).is_settled()
                    || prog.ctl_cant_spin_out(200)
                    || prog.cps_cant_spin_out(5)
                    || prog.seg_cant_spin_out(params, 5).is_refuted()
                    || check_inf(prog, params, 300, 500)
            }
        ),
        (
            ((2, 5), 1, TREE_LIM, (84_384, 69_757_168)),
            //
            |prog: &Prog, params: Params| {
                prog.cant_halt(1).is_settled()
                    || quick_term_or_rec(prog, 3_000).is_settled()
                    || prog.seg_cant_halt(params, 5).is_refuted()
                    || prog.ctl_cant_halt(200)
                    || prog.cps_cant_halt(5)
                    || check_inf(prog, params, 300, 500)
            }
        ),
        // (
        //     ((2, 5), 0, TREE_LIM, (1_296_168, 515_255_468)),
        //     //
        //     |prog: &Prog, params: Params| {
        //         prog.cant_spin_out(1).is_settled()
        //             || quick_term_or_rec(prog, 3_000).is_settled()
        //             || prog.cant_spin_out(20).is_settled()
        //             || prog.ctl_cant_spin_out(200)
        //             || prog.seg_cant_spin_out(params, 5).is_refuted()
        //             || prog.cps_cant_spin_out(5)
        //             || check_inf(prog, params, 300, 500)
        //     }
        // ),
    ];
}

/**************************************/

use tm::reason::BackwardResult;

fn assert_reason(params: Params, halt: u8, expected: (usize, u64)) {
    let halt_flag = halt != 0;

    let holdout_count = set_val(0);
    let refuted_steps = set_val(0);

    let cant_reach = if halt_flag {
        Prog::cant_halt
    } else {
        Prog::cant_spin_out
    };

    build_tree(params, halt_flag, 300, &|prog| {
        let result = cant_reach(prog, 256);

        if let BackwardResult::Refuted(steps) = result
            && steps > *access(&refuted_steps)
        {
            *access(&refuted_steps) = steps;
        }

        if result.is_settled() {
            return;
        }

        *access(&holdout_count) += 1;

        // println!("{}", prog.show());
    });

    let result = (get_val(refuted_steps), get_val(holdout_count));

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

fn test_reason() {
    assert_reason_results![
        ((2, 2), 1, (2, 10)),
        ((2, 2), 0, (1, 4)),
        //
        ((3, 2), 1, (12, 1_474)),
        ((3, 2), 0, (6, 996)),
        //
        ((2, 3), 1, (7, 2_113)),
        ((2, 3), 0, (6, 1_271)),
        //
        ((4, 2), 1, (45, 258_343)),
        ((4, 2), 0, (35, 211_555)),
        //
        ((2, 4), 1, (16, 304_330)),
        ((2, 4), 0, (12, 370_706)),
    ];
}

/**************************************/

fn assert_linrec(params: Params, expected: u64) {
    let holdout_count = set_val(0);

    build_tree(params, false, TREE_LIM, &|prog| {
        let result = quick_term_or_rec(prog, LINREC);

        if result.is_settled() {
            return;
        }

        *access(&holdout_count) += 1;

        // println!("{}", prog.show());
    });

    let result = get_val(holdout_count);

    assert_eq!(result, expected, "({params:?}, {result:?})");
}

macro_rules! assert_linrec_results {
    ( $( ( $params:expr, $leaves:expr ) ),* $(,)? ) => {
        vec![$( ($params, $leaves) ),*]
            .par_iter().for_each(|&(params, expected)| {
                assert_linrec(params, expected);
            });
    };
}

const LINREC: usize = 20_000;

fn test_linrec() {
    assert_linrec_results![
        ((2, 2), 4),
        ((3, 2), 643),
        ((2, 3), 1_031),
        ((4, 2), 138_585),
        ((2, 4), 257_311),
    ];
}

/**************************************/

fn assert_blank(params: Params, expected: (usize, u64)) {
    let holdout_count = set_val(0);
    let refuted_steps = set_val(0);

    build_tree(params, false, TREE_LIM, &|prog| {
        let run = 700;

        let backward = prog.cant_blank(44);

        if let BackwardResult::Refuted(steps) = backward
            && steps > *access(&refuted_steps)
        {
            *access(&refuted_steps) = steps;
        }

        if backward.is_settled()
            || quick_term_or_rec(prog, run).is_settled()
            || check_inf(prog, params, 300, run)
            || prog.ctl_cant_blank(100)
            || prog.cps_cant_blank(5)
        {
            return;
        }

        *access(&holdout_count) += 1;

        // println!("{}", prog.show());
    });

    let result = (get_val(refuted_steps), get_val(holdout_count));

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

fn test_blank() {
    assert_blank_results![
        ((2, 2), (1, 0)),
        //
        ((3, 2), (13, 0)),
        //
        ((2, 3), (15, 8)),
        //
        ((4, 2), (43, 630)),
        //
        ((2, 4), (40, 2_084)),
    ];
}

/**************************************/

fn main() {
    println!("tree fast");
    test_tree();

    let args: Vec<String> = env::args().collect();

    if !args.contains(&"--all".to_string()) {
        return;
    }

    println!("reason");
    test_reason();

    println!("blank");
    test_blank();

    println!("linrec");
    test_linrec();

    println!("tree slow");
    test_tree_slow();
}
