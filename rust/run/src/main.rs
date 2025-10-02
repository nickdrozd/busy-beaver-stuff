#![expect(clippy::too_many_lines)]

use std::env;

use rayon::prelude::*;

use tm::{
    Goal, Params, Prog,
    tree::{PassConfig, Step, build_limited, build_tree},
};

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
    tree: Step,
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
            ((2, 2), 0, 2, (0, 31)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                //
                prog.term_or_rec(8).is_settled()
            }
        ),
        (
            ((2, 2), 1, 4, (0, 36)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.cant_spin_out(0).is_settled()
                    || prog.term_or_rec(16).is_settled()
            }
        ),
        (
            ((2, 2), 2, 4, (0, 61)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.cant_blank(3).is_settled()
                    || prog.term_or_rec(15).is_settled()
            }
        ),
        (
            ((2, 2), 3, 4, (4, 89)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.term_or_rec(19).is_settled()
            }
        ),
        (
            ((3, 2), 0, 12, (5, 2_930)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.term_or_rec(40).is_settled()
                    || prog.cant_halt(3).is_settled()
                    || prog.ctl_cant_halt(16)
            }
        ),
        (
            ((3, 2), 1, 13, (30, 12_353)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.term_or_rec(206).is_settled()
                    || prog.cant_spin_out(4).is_settled()
                    || prog.ctl_cant_spin_out(15)
            }
        ),
        (
            ((3, 2), 2, 13, (17, 10_046)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.cant_blank(14).is_settled()
                    || prog.term_or_rec(206).is_settled()
                    || prog.ctl_cant_blank(25)
                    || prog.cps_cant_blank(6)
            }
        ),
        (
            ((3, 2), 3, 13, (25, 12_353)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.term_or_rec(206).is_settled()
                    || prog.check_inf(500, 50)
            }
        ),
        (
            ((2, 3), 0, 7, (2, 2_387)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.term_or_rec(301).is_settled()
                    || prog.ctl_cant_halt(41)
                    || prog.cps_cant_halt(3)
            }
        ),
        (
            ((2, 3), 1, 20, (26, 3_580)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.term_or_rec(301).is_settled()
                    || prog.cant_spin_out(2).is_settled()
                    || prog.ctl_cant_spin_out(83)
                    || prog.seg_cant_spin_out(5).is_refuted()
            }
        ),
        (
            ((2, 3), 2, 20, (11, 6_038)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.cant_blank(16).is_settled()
                    || prog.term_or_rec(115).is_settled()
                    || prog.ctl_cant_blank(32)
                    || prog.cps_cant_blank(7)
            }
        ),
        (
            ((2, 3), 3, 20, (63, 8_844)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.term_or_rec(301).is_settled()
                    || prog.check_inf(1_000, 50)
            }
        ),
        (
            ((4, 2), 0, 25, (877, 448_563)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                !prog.is_connected()
                    || prog.term_or_rec(200).is_settled()
                    || prog.cant_halt(11).is_settled()
                    || prog.ctl_cant_halt(84)
                    || prog.cps_cant_halt(9)
            }
        ),
        (
            ((4, 2), 1, 99, (3_322, 2_208_026)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
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
            ((4, 2), 2, 99, (3_787, 1_999_945)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                !prog.is_connected()
                    || prog.cant_blank(55).is_settled()
                    || prog.term_or_rec(1_000).is_settled()
                    || prog.ctl_cant_blank(60)
                    || prog.cps_cant_blank(9)
            }
        ),
        (
            ((4, 2), 3, 99, (7_945, 2_208_026)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                !prog.is_connected()
                    || prog.term_or_rec(5_000).is_settled()
                    || prog.check_inf(2_000, 200)
            }
        ),
        (
            ((2, 4), 0, 109, (652, 308_993)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.cant_halt(0).is_settled()
                    || prog.term_or_rec(800).is_settled()
                    || prog.seg_cant_halt(2).is_refuted()
                    || prog.ctl_cant_halt(96)
                    || prog.cps_cant_halt(6)
            }
        ),
        (
            ((2, 4), 1, 876, (5_499, 614_144)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.cant_spin_out(2).is_settled()
                    || prog.term_or_rec(2_000).is_settled()
                    || prog.cant_spin_out(7).is_settled()
                    || prog.ctl_cant_spin_out(186)
                    || prog.cps_cant_spin_out(6)
                    || prog.seg_cant_spin_out(4).is_refuted()
            }
        ),
        (
            ((2, 4), 2, 876, (4_310, 1_189_458)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.cant_blank(58).is_settled()
                    || prog.term_or_rec(2_000).is_settled()
                    || prog.ctl_cant_blank(120)
                    || prog.cps_cant_blank(5)
            }
        ),
        (
            ((2, 4), 3, 876, (39_533, 1_698_504)),
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
            ((5, 2), 0, 700, (77_183_719, 92_727_997)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                !prog.is_connected() || prog.cant_halt(0).is_settled()
            }
        ),
        (
            ((5, 2), 1, TREE_LIM, (165_002_764, 521_354_298)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                !prog.is_connected()
                    || prog.cant_spin_out(0).is_settled()
            }
        ),
        (
            ((5, 2), 2, TREE_LIM, (474_746_985, 498_923_643)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                !prog.is_connected() || prog.cant_blank(0).is_settled()
            }
        ),
        (
            ((3, 3), 0, 2_700, (21_020_663, 24_785_306)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                !prog.is_connected() || prog.cant_halt(0).is_settled()
            }
        ),
        (
            ((3, 3), 1, 3_000, (51_375_220, 146_832_326)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                !prog.is_connected()
                    || prog.cant_spin_out(0).is_settled()
            }
        ),
        (
            ((3, 3), 2, 3_000, (121_258_803, 124_861_783)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                !prog.is_connected() || prog.cant_blank(0).is_settled()
            }
        ),
        (
            ((2, 5), 0, TREE_LIM, (66_076_634, 69_554_118)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.cant_halt(0).is_settled()
            }
        ),
        (
            ((2, 5), 1, TREE_LIM, (163_050_843, 163_050_843)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.cant_spin_out(0).is_settled()
            }
        ),
        (
            ((2, 5), 2, TREE_LIM, (349_647_329, 366_154_432)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                prog.cant_blank(0).is_settled()
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

        if result.is_settled() {
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
        ((2, 2), 0, (3, 6)),
        ((2, 2), 1, (2, 7)),
        ((2, 2), 2, (2, 10)),
        //
        ((3, 2), 0, (13, 1_387)),
        ((3, 2), 1, (7, 1_870)),
        ((3, 2), 2, (14, 2_298)),
        //
        ((2, 3), 0, (8, 1_336)),
        ((2, 3), 1, (2, 1_620)),
        ((2, 3), 2, (16, 1_640)),
        //
        ((4, 2), 0, (46, 251_961)),
        ((4, 2), 1, (15, 402_915)),
        ((4, 2), 2, (51, 490_815)),
        //
        ((2, 4), 0, (17, 255_402)),
        ((2, 4), 1, (8, 416_452)),
        ((2, 4), 2, (58, 358_177)),
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

    assert_eq!(progs.get().len(), 89);
}

/**************************************/

fn assert_limited(
    instrs: u8,
    tree: Step,
    expected: (u64, u64),
    pipeline: impl Fn(&Prog, PassConfig<'_>) -> bool + Sync,
) {
    let holdout = Basket::set(0);
    let visited = Basket::set(0);

    build_limited((instrs, instrs), instrs, tree, &|prog, config| {
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
            (4, 4, (0, 5_248)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                //
                prog.term_or_rec(24).is_settled()
                    || prog.cant_halt(0).is_settled()
                    || prog.ctl_cant_halt(13)
            }
        ),
        (
            (5, 12, (13, 156_627)),
            //
            |prog: &Prog, _config: PassConfig<'_>| {
                //
                prog.term_or_rec(301).is_settled()
                    || prog.ctl_cant_halt(25)
                    || prog.cps_cant_halt(3)
            }
        ),
        (
            (6, 22, (526, 5_726_886)),
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
        (7, 109, (22_996, 251_356_358)),
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
