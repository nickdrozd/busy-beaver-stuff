#![allow(unused_imports)]
#![expect(clippy::too_many_lines)]

use std::env;

use rayon::prelude::*;

use tm::{
    Goal,
    instrs::Params,
    prog::Prog,
    tree::{Step, build_limited, build_tree},
};

use tm::instrs::Parse as _;

mod basket;

use basket::Basket;

/**************************************/

const TREE_LIM: Step = 876;

/**************************************/

use Goal::*;

fn get_goal(goal: u8) -> Option<Goal> {
    match goal {
        0 => Some(Halt),
        1 => Some(Spinout),
        2 => Some(Blank),
        3 => None,
        _ => unreachable!(),
    }
}

/**************************************/

macro_rules! assert_trees {
    ( $( ( ($params:expr, $goal:expr, $tree:expr, $leaves:expr), $pipeline:expr ) ),* $(,)? ) => {
        vec![
            $( (
                ($params, $goal, $tree, $leaves),
                Box::new($pipeline) as Box<dyn Fn(&Prog) -> bool + Sync>
            ) ),*]
            .par_iter()
            .for_each(|&((params, goal, tree, expected), ref pipeline)| {
                assert_tree(params, goal, tree, expected, pipeline);
            });
    };
}

fn assert_tree(
    params: Params,
    goal: u8,
    tree: Step,
    expected: (u64, u64),
    pipeline: impl Fn(&Prog) -> bool + Sync,
) {
    let holdout = Basket::set(0);
    let visited = Basket::set(0);

    build_tree(params, get_goal(goal), tree, &|prog| {
        *visited.access() += 1;

        if pipeline(prog) {
            return;
        }

        *holdout.access() += 1;

        // println!("{}", prog.show());
    });

    let result = (holdout.get(), visited.get());

    assert_eq!(result, expected, "({params:?}, {goal}, {result:?})");
}

fn test_tree() {
    println!("tree fast");

    assert_trees![
        (
            ((2, 2), 0, 2, (0, 32)),
            //
            |prog: &Prog| {
                //
                prog.term_or_rec(8).is_settled()
            }
        ),
        (
            ((2, 2), 1, 4, (0, 36)),
            //
            |prog: &Prog| {
                prog.cant_spin_out(0).is_settled()
                    || prog.term_or_rec(16).is_settled()
            }
        ),
        (
            ((2, 2), 2, 4, (0, 61)),
            //
            |prog: &Prog| {
                prog.cant_blank(3).is_settled()
                    || prog.term_or_rec(15).is_settled()
            }
        ),
        (
            ((2, 2), 3, 4, (4, 89)),
            //
            |prog: &Prog| { prog.term_or_rec(19).is_settled() }
        ),
        (
            ((3, 2), 0, 12, (5, 3_030)),
            //
            |prog: &Prog| {
                prog.term_or_rec(40).is_settled()
                    || prog.cant_halt(3).is_settled()
                    || prog.ctl_cant_halt(16)
            }
        ),
        (
            ((3, 2), 1, 13, (30, 12_470)),
            //
            |prog: &Prog| {
                prog.term_or_rec(206).is_settled()
                    || prog.cant_spin_out(4).is_settled()
                    || prog.ctl_cant_spin_out(15)
            }
        ),
        (
            ((3, 2), 2, 13, (17, 10_175)),
            //
            |prog: &Prog| {
                prog.cant_blank(14).is_settled()
                    || prog.term_or_rec(206).is_settled()
                    || prog.ctl_cant_blank(25)
                    || prog.cps_cant_blank(6)
            }
        ),
        (
            ((3, 2), 3, 13, (640, 12_470)),
            //
            |prog: &Prog| { prog.term_or_rec(206).is_settled() }
        ),
        (
            ((2, 3), 0, 7, (2, 2_394)),
            //
            |prog: &Prog| {
                prog.term_or_rec(301).is_settled()
                    || prog.ctl_cant_halt(41)
                    || prog.cps_cant_halt(3)
            }
        ),
        (
            ((2, 3), 1, 20, (26, 3_580)),
            //
            |prog: &Prog| {
                prog.term_or_rec(301).is_settled()
                    || prog.cant_spin_out(2).is_settled()
                    || prog.ctl_cant_spin_out(83)
                    || prog.seg_cant_spin_out(5).is_refuted()
            }
        ),
        (
            ((2, 3), 2, 20, (11, 6_047)),
            //
            |prog: &Prog| {
                prog.cant_blank(16).is_settled()
                    || prog.term_or_rec(115).is_settled()
                    || prog.ctl_cant_blank(32)
                    || prog.cps_cant_blank(7)
            }
        ),
        (
            ((2, 3), 3, 20, (1_031, 8_847)),
            //
            |prog: &Prog| { prog.term_or_rec(301).is_settled() }
        ),
        (
            ((4, 2), 0, 25, (877, 458_588)),
            //
            |prog: &Prog| {
                !prog.is_connected()
                    || prog.term_or_rec(200).is_settled()
                    || prog.cant_halt(11).is_settled()
                    || prog.ctl_cant_halt(84)
                    || prog.cps_cant_halt(9)
            }
        ),
        (
            ((4, 2), 1, 99, (3_322, 2_222_970)),
            //
            |prog: &Prog| {
                !prog.is_connected()
                    || prog.cant_spin_out(1).is_settled()
                    || prog.term_or_rec(1_000).is_settled()
                    || prog.cant_spin_out(11).is_settled()
                    || prog.ctl_cant_spin_out(114)
                    || prog.cps_cant_spin_out(9)
                    || prog.seg_cant_spin_out(7).is_refuted()
            }
        ),
        (
            ((4, 2), 2, 99, (3_787, 2_014_818)),
            //
            |prog: &Prog| {
                !prog.is_connected()
                    || prog.cant_blank(55).is_settled()
                    || prog.term_or_rec(1_000).is_settled()
                    || prog.ctl_cant_blank(60)
                    || prog.cps_cant_blank(9)
            }
        ),
        (
            ((4, 2), 3, 99, (134_743, 2_222_970)),
            //
            |prog: &Prog| {
                !prog.is_connected()
                    || prog.term_or_rec(5_000).is_settled()
            }
        ),
        (
            ((2, 4), 0, 109, (652, 310_211)),
            //
            |prog: &Prog| {
                prog.cant_halt(0).is_settled()
                    || prog.term_or_rec(800).is_settled()
                    || prog.seg_cant_halt(2).is_refuted()
                    || prog.ctl_cant_halt(96)
                    || prog.cps_cant_halt(6)
            }
        ),
        (
            ((2, 4), 1, 876, (5_499, 614_151)),
            //
            |prog: &Prog| {
                prog.cant_spin_out(2).is_settled()
                    || prog.term_or_rec(2_000).is_settled()
                    || prog.cant_spin_out(7).is_settled()
                    || prog.ctl_cant_spin_out(186)
                    || prog.cps_cant_spin_out(6)
                    || prog.seg_cant_spin_out(4).is_refuted()
            }
        ),
        (
            ((2, 4), 2, 876, (4_310, 1_189_715)),
            //
            |prog: &Prog| {
                prog.cant_blank(58).is_settled()
                    || prog.term_or_rec(2_000).is_settled()
                    || prog.ctl_cant_blank(120)
                    || prog.cps_cant_blank(5)
            }
        ),
        (
            ((2, 4), 3, 876, (257_465, 1_698_539)),
            //
            |prog: &Prog| { prog.term_or_rec(5_000).is_settled() }
        ),
    ];
}

fn test_tree_slow() {
    println!("tree slow");

    assert_trees![
        (
            ((5, 2), 0, 700, (77_897_792, 94_160_306)),
            //
            |prog: &Prog| {
                !prog.is_connected() || prog.cant_halt(0).is_settled()
            }
        ),
        (
            ((5, 2), 1, TREE_LIM, (166_610_384, 523_722_375)),
            //
            |prog: &Prog| {
                !prog.is_connected()
                    || prog.cant_spin_out(0).is_settled()
            }
        ),
        (
            ((5, 2), 2, TREE_LIM, (476_395_037, 501_259_494)),
            //
            |prog: &Prog| {
                !prog.is_connected() || prog.cant_blank(0).is_settled()
            }
        ),
        (
            ((3, 3), 0, 2_700, (21_235_610, 25_028_837)),
            //
            |prog: &Prog| {
                !prog.is_connected() || prog.cant_halt(0).is_settled()
            }
        ),
        (
            ((3, 3), 1, 3_000, (51_748_364, 147_230_805)),
            //
            |prog: &Prog| {
                !prog.is_connected()
                    || prog.cant_spin_out(0).is_settled()
            }
        ),
        (
            ((3, 3), 2, 3_000, (121_596_738, 125_217_382)),
            //
            |prog: &Prog| {
                !prog.is_connected() || prog.cant_blank(0).is_settled()
            }
        ),
        (
            ((2, 5), 0, TREE_LIM, (66_279_667, 69_757_168)),
            //
            |prog: &Prog| { prog.cant_halt(0).is_settled() }
        ),
        (
            ((2, 5), 1, TREE_LIM, (163_051_841, 163_051_841)),
            //
            |prog: &Prog| { prog.cant_spin_out(0).is_settled() }
        ),
        (
            ((2, 5), 2, TREE_LIM, (349_660_714, 366_167_817)),
            //
            |prog: &Prog| { prog.cant_blank(0).is_settled() }
        ),
    ];
}

/**************************************/

use tm::reason::BackwardResult;

fn assert_reason(params: Params, goal: u8, expected: (usize, u64)) {
    let holdout = Basket::set(0);
    let refuted = Basket::set(0);

    let cant_reach = match goal {
        0 => Prog::cant_halt,
        1 => Prog::cant_spin_out,
        2 => Prog::cant_blank,
        _ => unreachable!(),
    };

    build_tree(params, get_goal(goal), TREE_LIM, &|prog| {
        let result = cant_reach(prog, 256);

        if let BackwardResult::Refuted(steps) = result {
            let mut curr_max = refuted.access();

            if steps > *curr_max {
                *curr_max = steps;
            }
        }

        if result.is_settled() {
            return;
        }

        *holdout.access() += 1;

        // println!("{}", prog.show());
    });

    let result = (refuted.get(), holdout.get());

    assert_eq!(result, expected, "({params:?}, {goal}, {result:?})");
}

macro_rules! assert_reason_results {
    ( $( ( $params:expr, $goal:expr, $leaves:expr ) ),* $(,)? ) => {
        vec![$( ($params, $goal, $leaves) ),*]
            .par_iter().for_each(|&(params, goal, expected)| {
                assert_reason(params, goal, expected);
            });
    };
}

fn test_reason() {
    println!("reason");

    assert_reason_results![
        ((2, 2), 0, (3, 6)),
        ((2, 2), 1, (2, 7)),
        ((2, 2), 2, (2, 10)),
        //
        ((3, 2), 0, (13, 1_425)),
        ((3, 2), 1, (7, 1_973)),
        ((3, 2), 2, (14, 2_348)),
        //
        ((2, 3), 0, (8, 1_359)),
        ((2, 3), 1, (2, 1_620)),
        ((2, 3), 2, (16, 1_649)),
        //
        ((4, 2), 0, (46, 258_161)),
        ((4, 2), 1, (15, 416_871)),
        ((4, 2), 2, (51, 494_958)),
        //
        ((2, 4), 0, (17, 256_803)),
        ((2, 4), 1, (8, 416_459)),
        ((2, 4), 2, (58, 358_434)),
    ];
}

/**************************************/

fn test_collect() {
    println!("collect");

    let progs = Basket::set(vec![]);

    build_tree((2, 2), None, 4, &|prog| {
        progs.access().push(prog.show());
    });

    assert_eq!(progs.get().len(), 89);
}

/**************************************/

fn assert_limited(
    instrs: u8,
    tree: Step,
    expected: (u64, u64),
    pipeline: impl Fn(&Prog) -> bool + Sync,
) {
    let holdout = Basket::set(0);
    let visited = Basket::set(0);

    build_limited((instrs, instrs), instrs, tree, &|prog| {
        *visited.access() += 1;

        if pipeline(prog) {
            return;
        }

        *holdout.access() += 1;

        // println!("{}", prog.show());
    });

    let result = (holdout.get(), visited.get());

    assert_eq!(result, expected);
}

macro_rules! assert_limited_results {
    ( $( ( ($instrs:expr, $tree:expr, $leaves:expr), $pipeline:expr ) ),* $(,)? ) => {
        vec![
            $( (
                ($instrs,$tree, $leaves),
                Box::new($pipeline) as Box<dyn Fn(&Prog) -> bool + Sync>
            ) ),*]
            .par_iter()
            .for_each(|&((instrs, tree, expected), ref pipeline)| {
                assert_limited(instrs, tree, expected, pipeline);
            });
    };
}

fn test_limited() {
    println!("limited");

    assert_limited_results![
        (
            (4, 4, (0, 5_444)),
            //
            |prog: &Prog| {
                //
                prog.term_or_rec(24).is_settled()
                    || prog.ctl_cant_halt(13)
                    || prog.cps_cant_halt(3)
            }
        ),
        (
            (5, 12, (13, 161_024)),
            //
            |prog: &Prog| {
                //
                prog.term_or_rec(301).is_settled()
                    || prog.ctl_cant_halt(25)
                    || prog.cps_cant_halt(3)
            }
        ),
        (
            (6, 22, (526, 5_857_888)),
            //
            |prog: &Prog| {
                prog.term_or_rec(304).is_settled()
                    || prog.ctl_cant_halt(76)
                    || prog.cps_cant_halt(5)
            }
        ),
    ];
}

fn test_limited_slow() {
    println!("limited slow");

    assert_limited_results![(
        (7, 109, (22_996, 256_221_202)),
        //
        |prog: &Prog| {
            prog.term_or_rec(1000).is_settled()
                || prog.ctl_cant_halt(100)
                || prog.cps_cant_halt(4)
        }
    ),];
}

/**************************************/

fn main() {
    test_tree();

    #[expect(clippy::needless_collect)]
    let args: Vec<String> = env::args().collect();

    if !args.contains(&"--all".to_owned()) {
        return;
    }

    test_limited();

    test_collect();

    test_reason();

    test_limited_slow();

    test_tree_slow();
}
