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
            ((2, 2), 0, 2, (0, 23)),
            //
            |prog: &Prog, mut config: PassConfig<'_>| {
                //
                prog.run_transcript(9, config.to_mut()).is_settled()
            }
        ),
        (
            ((2, 2), 1, 4, (0, 32)),
            //
            |prog: &Prog, mut config: PassConfig<'_>| {
                prog.cant_spin_out(0).is_refuted()
                    || prog
                        .run_transcript(18, config.to_mut())
                        .is_settled()
            }
        ),
        (
            ((2, 2), 2, 4, (0, 53)),
            //
            |prog: &Prog, mut config: PassConfig<'_>| {
                prog.cant_blank(3).is_refuted()
                    || prog
                        .run_transcript(10, config.to_mut())
                        .is_settled()
            }
        ),
        (
            ((2, 2), 3, 4, (4, 81)),
            //
            |prog: &Prog, mut config: PassConfig<'_>| {
                prog.run_transcript(18, config.to_mut()).is_settled()
            }
        ),
        (
            ((3, 2), 0, 12, (5, 2_686)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.term_or_rec(40).is_settled()
                    || prog.cant_halt(3).is_refuted()
                    || prog.ctl_cant_halt(16)
            }
        ),
        (
            ((3, 2), 1, 13, (30, 4_002)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.term_or_rec(206).is_settled()
                    || prog.cant_spin_out(4).is_refuted()
                    || prog.ctl_cant_spin_out(15)
            }
        ),
        (
            ((3, 2), 2, 13, (17, 9_473)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.cant_blank(14).is_refuted()
                    || prog.term_or_rec(206).is_settled()
                    || prog.ctl_cant_blank(25)
                    || prog.cps_cant_blank(6)
            }
        ),
        (
            ((3, 2), 3, 13, (25, 11_709)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.term_or_rec(206).is_settled()
                    || prog.check_inf(500, 50)
            }
        ),
        (
            ((2, 3), 0, 7, (2, 2_327)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.term_or_rec(301).is_settled()
                    || prog.ctl_cant_halt(41)
                    || prog.cps_cant_halt(3)
            }
        ),
        (
            ((2, 3), 1, 20, (26, 3_500)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.term_or_rec(301).is_settled()
                    || prog.cant_spin_out(2).is_refuted()
                    || prog.ctl_cant_spin_out(83)
                    || prog.seg_cant_spin_out(5).is_refuted()
            }
        ),
        (
            ((2, 3), 2, 20, (11, 5_910)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.cant_blank(16).is_settled()
                    || prog.term_or_rec(115).is_settled()
                    || prog.ctl_cant_blank(32)
                    || prog.cps_cant_blank(7)
            }
        ),
        (
            ((2, 3), 3, 20, (63, 8_716)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.term_or_rec(301).is_settled()
                    || prog.check_inf(1_000, 50)
            }
        ),
        (
            ((4, 2), 0, 25, (877, 425_283)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                !prog.is_connected()
                    || prog.term_or_rec(200).is_settled()
                    || prog.cant_halt(11).is_refuted()
                    || prog.ctl_cant_halt(84)
                    || prog.cps_cant_halt(9)
            }
        ),
        (
            ((4, 2), 1, 99, (3_322, 749_240)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                !prog.is_connected()
                    || prog.cant_spin_out(1).is_refuted()
                    || prog.term_or_rec(1_000).is_settled()
                    || prog.cant_spin_out(11).is_refuted()
                    || prog.ctl_cant_spin_out(114)
                    || prog.cps_cant_spin_out(9)
                    || prog.seg_cant_spin_out(7).is_refuted()
            }
        ),
        (
            ((4, 2), 2, 99, (3_787, 1_928_464)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                !prog.is_connected()
                    || prog.cant_blank(55).is_refuted()
                    || prog.term_or_rec(1_000).is_settled()
                    || prog.ctl_cant_blank(60)
                    || prog.cps_cant_blank(9)
            }
        ),
        (
            ((4, 2), 3, 99, (7_945, 2_130_068)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                !prog.is_connected()
                    || prog.term_or_rec(5_000).is_settled()
                    || prog.check_inf(2_000, 200)
            }
        ),
        (
            ((2, 4), 0, 109, (652, 307_277)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.cant_halt(0).is_refuted()
                    || prog.term_or_rec(800).is_settled()
                    || prog.seg_cant_halt(2).is_refuted()
                    || prog.ctl_cant_halt(96)
                    || prog.cps_cant_halt(6)
            }
        ),
        (
            ((2, 4), 1, TREE_LIM, (5_499, 611_752)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.cant_spin_out(2).is_refuted()
                    || prog.term_or_rec(2_000).is_settled()
                    || prog.cant_spin_out(7).is_refuted()
                    || prog.ctl_cant_spin_out(186)
                    || prog.cps_cant_spin_out(6)
                    || prog.seg_cant_spin_out(4).is_refuted()
            }
        ),
        (
            ((2, 4), 2, TREE_LIM, (4_310, 1_184_586)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.cant_blank(58).is_refuted()
                    || prog.term_or_rec(2_000).is_settled()
                    || prog.ctl_cant_blank(120)
                    || prog.cps_cant_blank(5)
            }
        ),
        (
            ((2, 4), 3, TREE_LIM, (39_533, 1_693_632)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.term_or_rec(5_000).is_settled()
                    || prog.check_inf(1_000, 200)
            }
        ),
    ];
}

fn test_tree_slow() {
    println!("tree slow");

    assert_trees![
        (
            ((5, 2), 0, 700, (75_024_753, 89_436_309)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                !prog.is_connected() || prog.cant_halt(0).is_refuted()
            }
        ),
        (
            ((5, 2), 1, TREE_LIM, (154_925_731, 180_262_912)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                !prog.is_connected()
                    || prog.cant_spin_out(0).is_refuted()
            }
        ),
        (
            ((5, 2), 2, TREE_LIM, (463_139_768, 485_795_395)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                !prog.is_connected() || prog.cant_blank(0).is_refuted()
            }
        ),
        (
            ((3, 3), 0, 2_700, (20_705_323, 24_351_293)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                !prog.is_connected() || prog.cant_halt(0).is_refuted()
            }
        ),
        (
            ((3, 3), 1, 3_000, (50_002_453, 51_212_296)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                !prog.is_connected()
                    || prog.cant_spin_out(0).is_refuted()
            }
        ),
        (
            ((3, 3), 2, 3_000, (119_932_883, 123_495_673)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                !prog.is_connected() || prog.cant_blank(0).is_refuted()
            }
        ),
        (
            ((2, 5), 0, TREE_LIM, (65_993_597, 69_468_390)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.cant_halt(0).is_refuted()
            }
        ),
        (
            ((2, 5), 1, TREE_LIM, (162_921_855, 162_921_855)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.cant_spin_out(0).is_refuted()
            }
        ),
        (
            ((2, 5), 2, TREE_LIM, (349_348_169, 365_855_272)),
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
        ((3, 2), 0, (13, 1_685)),
        ((3, 2), 1, (7, 1_632)),
        ((3, 2), 2, (14, 2_149)),
        //
        ((2, 3), 0, (8, 1_423)),
        ((2, 3), 1, (2, 1_578)),
        ((2, 3), 2, (16, 1_569)),
        //
        ((4, 2), 0, (46, 263_565)),
        ((4, 2), 1, (15, 352_161)),
        ((4, 2), 2, (51, 467_675)),
        //
        ((2, 4), 0, (17, 255_877)),
        ((2, 4), 1, (8, 414_757)),
        ((2, 4), 2, (58, 354_345)),
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
            (4, 4, (0, 4_868)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                //
                prog.term_or_rec(24).is_settled()
                    || prog.cant_halt(0).is_refuted()
                    || prog.ctl_cant_halt(13)
            }
        ),
        (
            (5, 12, (13, 150_325)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                //
                prog.term_or_rec(301).is_settled()
                    || prog.ctl_cant_halt(25)
                    || prog.cps_cant_halt(3)
            }
        ),
        (
            (6, 22, (526, 5_543_667)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
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
        (7, 109, (22_996, 245_724_902)),
        //
        |prog: &Prog, _config: PassConfig<'_>| {
            prog.term_or_rec(1000).is_settled()
                || prog.ctl_cant_halt(100)
                || prog.cps_cant_halt(4)
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
