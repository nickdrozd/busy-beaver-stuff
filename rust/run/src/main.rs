#![expect(clippy::too_many_lines)]

use std::env;

use rayon::prelude::*;

use tm::{
    Goal, Params, Prog, Steps,
    tree::{PassConfig, build_limited, build_tree},
};

pub mod basket;

use basket::Basket;

/**************************************/

const TREE_LIM: Steps = 876;

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
        #[expect(trivial_casts)]
        vec![
            $( (
                ($params, $goal, $tree, $leaves),
                Box::new($pipeline) as Box<dyn Fn(&Prog, PassConfig<'_>) -> bool + Sync>
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
    tree: Steps,
    expected: (u64, u64),
    pipeline: impl Fn(&Prog, PassConfig<'_>) -> bool + Sync,
) {
    let holdout = Basket::set(0);
    let visited = Basket::set(0);

    build_tree(params, get_goal(goal), tree, &|prog, config| {
        *visited.access() += 1;

        if pipeline(prog, config) {
            return;
        }

        *holdout.access() += 1;

        // prog.print();
    });

    let result = (holdout.get(), visited.get());

    assert_eq!(result, expected, "({params:?}, {goal}, {result:?})");
}

fn test_tree() {
    println!("tree fast");

    assert_trees![
        (
            ((2, 2), 0, 2, (9, 23)),
            //
            |prog: &Prog, _: PassConfig<'_>| {
                prog.cant_halt(3).is_refuted() || prog.ctl_cant_halt(11)
            }
        ),
        (
            ((2, 2), 1, 4, (5, 32)),
            //
            |prog: &Prog, _: PassConfig<'_>| {
                prog.cant_spin_out(2).is_refuted()
                    || prog.ctl_cant_spin_out(6)
                    || prog.cps_cant_spin_out(3)
            }
        ),
        (
            ((2, 2), 2, 4, (5, 53)),
            //
            |prog: &Prog, _: PassConfig<'_>| {
                prog.cant_blank(2).is_refuted()
                    || prog.ctl_cant_blank(14)
            }
        ),
        (
            ((2, 2), 3, 4, (4, 81)),
            //
            |prog: &Prog, mut config: PassConfig<'_>| {
                prog.term_or_rec(16, config.to_mut()).is_settled()
            }
        ),
        (
            ((3, 2), 0, 12, (850, 2_650)),
            //
            |prog: &Prog, _: PassConfig<'_>| {
                prog.cant_halt(13).is_refuted()
                    || prog.ctl_cant_halt(31)
                    || prog.cps_cant_halt(4)
            }
        ),
        (
            ((3, 2), 1, 13, (517, 3_979)),
            //
            |prog: &Prog, _: PassConfig<'_>| {
                prog.cant_spin_out(7).is_refuted()
                    || prog.ctl_cant_spin_out(54)
                    || prog.cps_cant_spin_out(7)
            }
        ),
        (
            ((3, 2), 2, 13, (669, 9_442)),
            //
            |prog: &Prog, _: PassConfig<'_>| {
                prog.cant_blank(14).is_refuted()
                    || prog.ctl_cant_blank(42)
                    || prog.cps_cant_blank(10)
            }
        ),
        (
            ((3, 2), 3, 13, (25, 11_687)),
            //
            |prog: &Prog, mut config: PassConfig<'_>| {
                prog.term_or_rec(190, config.to_mut()).is_settled()
                    || prog.check_inf(500, 50)
            }
        ),
        (
            ((2, 3), 0, 7, (548, 2_264)),
            //
            |prog: &Prog, _: PassConfig<'_>| {
                prog.cant_halt(8).is_refuted()
                    || prog.ctl_cant_halt(60)
                    || prog.cps_cant_halt(4)
            }
        ),
        (
            ((2, 3), 1, 20, (551, 3_486)),
            //
            |prog: &Prog, _: PassConfig<'_>| {
                prog.cant_spin_out(2).is_refuted()
                    || prog.ctl_cant_spin_out(100)
                    || prog.cps_cant_spin_out(5)
                    || prog.seg_cant_spin_out(7).is_refuted()
            }
        ),
        (
            ((2, 3), 2, 20, (177, 5_891)),
            //
            |prog: &Prog, _: PassConfig<'_>| {
                prog.cant_blank(16).is_settled()
                    || prog.ctl_cant_blank(60)
                    || prog.cps_cant_blank(9)
            }
        ),
        (
            ((2, 3), 3, 20, (63, 8_700)),
            //
            |prog: &Prog, mut config: PassConfig<'_>| {
                prog.term_or_rec(290, config.to_mut()).is_settled()
                    || prog.check_inf(1_000, 50)
            }
        ),
        (
            ((4, 2), 0, 25, (115_953, 421_591)),
            //
            |prog: &Prog, _: PassConfig<'_>| {
                prog.cant_halt(46).is_refuted()
                    || prog.ctl_cant_halt(130)
                    || prog.cps_cant_halt(9)
            }
        ),
        (
            ((4, 2), 1, 99, (89_168, 743_986)),
            //
            |prog: &Prog, _: PassConfig<'_>| {
                prog.cant_spin_out(15).is_refuted()
                    || prog.ctl_cant_spin_out(190)
                    || prog.cps_cant_spin_out(9)
                    || prog.seg_cant_spin_out(8).is_refuted()
            }
        ),
        (
            ((4, 2), 2, 99, (113_529, 1_923_161)),
            //
            |prog: &Prog, _: PassConfig<'_>| {
                prog.cant_blank(51).is_refuted()
                    || prog.ctl_cant_blank(130)
                    || prog.cps_cant_blank(9)
            }
        ),
        (
            ((4, 2), 3, 99, (7_944, 2_125_270)),
            //
            |prog: &Prog, mut config: PassConfig<'_>| {
                if !prog.is_connected() {
                    return true;
                }

                let config = config.to_mut();

                prog.term_or_rec(500, config).is_settled()
                    || prog.check_inf(2_000, 200)
                    || prog.term_or_rec(4_710, config).is_settled()
            }
        ),
        (
            ((2, 4), 0, 109, (88_142, 301_646)),
            //
            |prog: &Prog, _: PassConfig<'_>| {
                prog.cant_halt(17).is_refuted()
                    || prog.ctl_cant_halt(190)
                    || prog.cps_cant_halt(5)
            }
        ),
        (
            ((2, 4), 1, TREE_LIM, (95_642, 610_085)),
            //
            |prog: &Prog, _: PassConfig<'_>| {
                prog.cant_spin_out(8).is_refuted()
                    || prog.ctl_cant_spin_out(300)
                    || prog.cps_cant_spin_out(5)
                    || prog.seg_cant_spin_out(5).is_refuted()
            }
        ),
        (
            ((2, 4), 2, TREE_LIM, (34_630, 1_182_719)),
            //
            |prog: &Prog, _: PassConfig<'_>| {
                prog.cant_blank(51).is_refuted()
                    || prog.ctl_cant_blank(200)
                    || prog.cps_cant_blank(5)
            }
        ),
        (
            ((2, 4), 3, TREE_LIM, (39_526, 1_691_774)),
            //
            |prog: &Prog, mut config: PassConfig<'_>| {
                let config = config.to_mut();

                prog.term_or_rec(500, config).is_settled()
                    || prog.check_inf(1_000, 200)
                    || prog.term_or_rec(4_600, config).is_settled()
            }
        ),
    ];
}

fn test_tree_slow() {
    println!("tree slow");

    assert_trees![
        (
            ((5, 2), 0, 700, (74_494_706, 88_801_216)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                !prog.is_connected() || prog.cant_halt(0).is_refuted()
            }
        ),
        (
            ((5, 2), 1, TREE_LIM, (154_212_276, 179_122_791)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                !prog.is_connected()
                    || prog.cant_spin_out(0).is_refuted()
            }
        ),
        (
            ((5, 2), 2, TREE_LIM, (462_449_446, 484_739_381)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                !prog.is_connected() || prog.cant_blank(0).is_refuted()
            }
        ),
        (
            ((3, 3), 0, 2_700, (20_404_758, 24_038_936)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                !prog.is_connected() || prog.cant_halt(0).is_refuted()
            }
        ),
        (
            ((3, 3), 1, 3_000, (49_824_481, 51_017_475)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                !prog.is_connected()
                    || prog.cant_spin_out(0).is_refuted()
            }
        ),
        (
            ((3, 3), 2, 3_000, (119_728_901, 123_276_016)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                !prog.is_connected() || prog.cant_blank(0).is_refuted()
            }
        ),
        (
            ((2, 5), 0, TREE_LIM, (64_921_591, 68_394_214)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.cant_halt(0).is_refuted()
            }
        ),
        (
            ((2, 5), 1, TREE_LIM, (162_541_326, 162_541_326)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.cant_spin_out(0).is_refuted()
            }
        ),
        (
            ((2, 5), 2, TREE_LIM, (348_948_850, 365_455_953)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.cant_blank(0).is_refuted()
            }
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

    build_tree(params, get_goal(goal), TREE_LIM, &|prog, _| {
        let result = cant_reach(prog, 256);

        if let BackwardResult::Refuted(steps) = result {
            let mut curr_max = refuted.access();

            if steps > *curr_max {
                *curr_max = steps;
            }
        }

        if result.is_refuted() {
            return;
        }

        *holdout.access() += 1;

        // prog.print();
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
        ((2, 2), 0, (3, 13)),
        ((2, 2), 1, (2, 8)),
        ((2, 2), 2, (2, 10)),
        //
        ((3, 2), 0, (13, 1_656)),
        ((3, 2), 1, (7, 1_610)),
        ((3, 2), 2, (14, 2_130)),
        //
        ((2, 3), 0, (8, 1_360)),
        ((2, 3), 1, (2, 1_567)),
        ((2, 3), 2, (16, 1_557)),
        //
        ((4, 2), 0, (46, 260_381)),
        ((4, 2), 1, (15, 347_209)),
        ((4, 2), 2, (51, 464_990)),
        //
        ((2, 4), 0, (17, 250_283)),
        ((2, 4), 1, (8, 413_173)),
        ((2, 4), 2, (58, 353_532)),
    ];
}

/**************************************/

fn test_collect() {
    use tm::Parse as _;

    println!("collect");

    let progs = Basket::set(vec![]);

    build_tree((2, 2), None, 4, &|prog, _| {
        progs.access().push(prog.show());
    });

    assert_eq!(progs.get().len(), 81);
}

/**************************************/

fn assert_limited(
    instrs: u8,
    tree: Steps,
    expected: (u64, u64),
    pipeline: impl Fn(&Prog, PassConfig<'_>) -> bool + Sync,
) {
    let holdout = Basket::set(0);
    let visited = Basket::set(0);

    build_limited(instrs, tree, &|prog, config| {
        *visited.access() += 1;

        if pipeline(prog, config) {
            return;
        }

        *holdout.access() += 1;

        // prog.print();
    });

    let result = (holdout.get(), visited.get());

    assert_eq!(result, expected);
}

macro_rules! assert_limited_results {
    ( $( ( ($instrs:expr, $tree:expr, $leaves:expr), $pipeline:expr ) ),* $(,)? ) => {
        #[expect(trivial_casts)]
        vec![
            $( (
                ($instrs,$tree, $leaves),
                Box::new($pipeline) as Box<dyn Fn(&Prog, PassConfig<'_>) -> bool + Sync>
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
            (4, 4, (0, 4_867)),
            //
            |prog: &Prog, mut config: PassConfig<'_>| {
                //
                prog.term_or_rec(16, config.to_mut()).is_settled()
                    || prog.cant_halt(0).is_refuted()
                    || prog.ctl_cant_halt(13)
            }
        ),
        (
            (5, 12, (13, 150_322)),
            //
            |prog: &Prog, mut config: PassConfig<'_>| {
                //
                prog.term_or_rec(301, config.to_mut()).is_settled()
                    || prog.ctl_cant_halt(25)
                    || prog.cps_cant_halt(3)
            }
        ),
        (
            (6, 22, (526, 5_543_646)),
            //
            |prog: &Prog, mut config: PassConfig<'_>| {
                prog.term_or_rec(304, config.to_mut()).is_settled()
                    || prog.ctl_cant_halt(76)
                    || prog.cps_cant_halt(5)
            }
        ),
    ];
}

fn test_limited_slow() {
    println!("limited slow");

    assert_limited_results![(
        (7, 109, (22_996, 245_724_778)),
        //
        |prog: &Prog, mut config: PassConfig<'_>| {
            prog.term_or_rec(100, config.to_mut()).is_settled()
                || prog.ctl_cant_halt(100)
                || prog.cps_cant_halt(4)
                || prog.term_or_rec(1_000, config.to_mut()).is_settled()
        }
    ),];
}

/**************************************/

fn main() {
    test_tree();

    if !env::args().any(|x| x == "--all") {
        return;
    }

    test_limited();

    test_collect();

    test_reason();

    test_limited_slow();

    test_tree_slow();
}
