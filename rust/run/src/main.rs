#![allow(dead_code, clippy::wildcard_imports)]
#![expect(clippy::used_underscore_items, clippy::needless_for_each)]
use rayon::prelude::*;

use tm::{Goal, Prog, Steps};

pub mod check;
pub mod harvesters;
pub mod holdouts;
pub mod tree;

use check::{assert_holdouts_match, test_holdouts};
use harvesters::{Collector, HoldoutVisited, Visited};
use holdouts::*;
use tree::{Harvester as _, PassConfig};

/**************************************/

const LIN_REC: Steps = 4_000;
const TREE_LIM: Steps = 876;

/**************************************/

use Goal::*;

fn get_goal(goal: u8) -> Option<Goal> {
    match goal {
        0 => Some(Halt),
        1 => Some(Spinout),
        2 => Some(Blank),
        3 | 4 => None,
        _ => unreachable!(),
    }
}

/**************************************/

macro_rules! assert_holdouts {
    ( $( ($states:literal, $colors:literal) => [ $( $goal:literal => ( $pipeline:ident, $steps:expr, ( $first:tt, $visited:expr ) ) ),* $(,)? ] ),* $(,)? ) => {{
        rayon::scope(|s| { $( $( assert_holdouts!(@goal s, $states, $colors, $goal, $pipeline, $steps, $first, $visited); )* )* });
    }};

    ( $( $instrs:literal => ( $pipeline:ident, $steps:expr, ( $first:tt, $visited:expr ) ) ),* $(,)? ) => {{
        rayon::scope(|s| { $( assert_holdouts!(@instrs s, $instrs, $pipeline, $steps, $first, $visited); )* });
    }};

    (@goal $scope:ident, $states:literal, $colors:literal, $goal:literal, $pipeline:ident, $steps:expr, $holdouts:ident, $visited:expr) => {{
        $scope.spawn(move |_| {
            let (champs, holdouts) = $holdouts;

            let (result, visited) = Collector::<$states, $colors>::run_params(
                get_goal($goal),
                $steps,
                &|| Collector::new(
                    |prog: &Prog<$states, $colors>, mut config: PassConfig<'_>| {
                        prog.term_or_rec(LIN_REC, config.to_mut()).is_settled()
                            || $pipeline(prog, config)
                    }
                ),
            );

            assert_holdouts_match(
                format!("(({}, {}), {})", $states, $colors, $goal),
                champs,
                holdouts,
                result,
            );

            assert_eq!(
                visited, $visited,
                "(({}, {}), {}, {visited:?})",
                $states, $colors, $goal,
            );
        });
    }};

    (@goal $scope:ident, $states:literal, $colors:literal, $goal:literal, $pipeline:ident, $steps:expr, $leaves:literal, $visited:expr) => {{
        $scope.spawn(move |_| {
            let result = HoldoutVisited::<$states, $colors>::run_params(
                get_goal($goal),
                $steps,
                &|| HoldoutVisited::new(
                    |prog: &Prog<$states, $colors>, mut config: PassConfig<'_>| {
                        prog.term_or_rec(LIN_REC, config.to_mut()).is_settled()
                            || $pipeline(prog, config)
                    }
                ),
            );

            assert_eq!(
                result,
                ($leaves, $visited),
                "(({}, {}), {}, {result:?})",
                $states, $colors, $goal,
            );
        });
    }};

    (@instrs $scope:ident, $instrs:literal, $pipeline:ident, $steps:expr, $leaves:literal, $visited:expr) => {{
        $scope.spawn(move |_| {
            let result = HoldoutVisited::<$instrs, $instrs>::run_instrs::<$instrs>(
                $steps,
                &|| HoldoutVisited::new(
                    |prog: &Prog<$instrs, $instrs>, mut config: PassConfig<'_>| {
                        prog.term_or_rec(LIN_REC, config.to_mut()).is_settled()
                            || $pipeline(prog, config)
                    }
                ),
            );

            assert_eq!(
                result,
                ($leaves, $visited),
                "({}, {result:?})",
                $instrs,
            );
        });
    }};

    (@instrs $scope:ident, $instrs:literal, $pipeline:ident, $steps:expr, $holdouts:ident, $visited:expr) => {{
        $scope.spawn(move |_| {
            let (champs, holdouts) = $holdouts;

            let (result, visited) = Collector::<$instrs, $instrs>::run_instrs::<$instrs>(
                $steps,
                &|| Collector::new(
                    |prog: &Prog<$instrs, $instrs>, mut config: PassConfig<'_>| {
                        prog.term_or_rec(LIN_REC, config.to_mut()).is_settled()
                            || $pipeline(prog, config)
                    }
                ),
            );

            assert_holdouts_match(
                format!("{}", $instrs),
                champs,
                holdouts,
                result,
            );

            assert_eq!(
                visited, $visited,
                "({}, {visited:?})",
                $instrs,
            );
        });
    }};
}

/**************************************/

fn _2_3_1(prog: &Prog<2, 3>, _: PassConfig<'_>) -> bool {
    prog.bkw_cant_spinout(7).is_refuted()
        || prog.ctl_cant_spinout(100)
        || prog.cps_cant_spinout(3)
        || prog.far_cant_spinout(3)
}

fn _4_2_1(prog: &Prog<4, 2>, _: PassConfig<'_>) -> bool {
    prog.bkw_cant_spinout(22).is_refuted()
        || prog.ctl_cant_spinout(500)
        || prog.cps_cant_spinout(12)
        || prog.seg_cant_spinout(10).is_refuted()
        || prog.far_cant_spinout(3)
}

fn _4_2_2(prog: &Prog<4, 2>, mut config: PassConfig<'_>) -> bool {
    prog.graph_cant_blank()
        || prog.bkw_cant_blank(51).is_refuted()
        || prog.ctl_cant_blank(130)
        || prog.cps_cant_blank(20)
        || prog.term_or_rec(10_000, config.to_mut()).is_settled()
        || prog.far_cant_blank(3)
}

fn _2_4_1(prog: &Prog<2, 4>, mut config: PassConfig<'_>) -> bool {
    prog.bkw_cant_spinout(50).is_refuted()
        || prog.ctl_cant_spinout(700)
        || prog.cps_cant_spinout(11)
        || prog.seg_cant_spinout(10).is_refuted()
        || prog.term_or_rec(10_000, config.to_mut()).is_settled()
        || prog.far_cant_spinout(3)
}

fn _2_4_2(prog: &Prog<2, 4>, mut config: PassConfig<'_>) -> bool {
    prog.graph_cant_blank()
        || prog.bkw_cant_blank(51).is_refuted()
        || prog.ctl_cant_blank(200)
        || prog.cps_cant_blank(20)
        || prog.term_or_rec(10_000, config.to_mut()).is_settled()
        || prog.far_cant_blank(3)
}

fn test_deciders() {
    println!("deciders");

    assert_holdouts![
        (2, 3) => [
            1 => (_2_3_1, 20, (_2_3_1_, 3_506)),
        ],
        (4, 2) => [
            1 => (_4_2_1, 99, (_4_2_1_, 753_582)),
            2 => (_4_2_2, 99, (_4_2_2_, 1_932_610)),
        ],
        (2, 4) => [
            1 => (_2_4_1, TREE_LIM, (_2_4_1_, 612_077)),
            2 => (_2_4_2, TREE_LIM, (_2_4_2_, 1_189_643)),
        ],
    ];
}

fn _3_2_3(prog: &Prog<3, 2>, _: PassConfig<'_>) -> bool {
    prog.graph_cant_twostep() || prog.bkw_cant_twostep(10).is_refuted()
}

fn _2_3_3(prog: &Prog<2, 3>, _: PassConfig<'_>) -> bool {
    prog.graph_cant_twostep() || prog.bkw_cant_twostep(10).is_refuted()
}

fn _4_2_3(prog: &Prog<4, 2>, _: PassConfig<'_>) -> bool {
    prog.graph_cant_twostep() || prog.bkw_cant_twostep(50).is_refuted()
}

fn _2_4_3(prog: &Prog<2, 4>, _: PassConfig<'_>) -> bool {
    prog.graph_cant_twostep() || prog.bkw_cant_twostep(30).is_refuted()
}

fn test_twostep() {
    println!("twostep");

    assert_holdouts![
        (3, 2) => [
            3 => (_3_2_3, 13, (_3_2_3_, 11_754)),
        ],
        (2, 3) => [
            3 => (_2_3_3, 20, (_2_3_3_, 8_766)),
        ],
    ];

    assert_holdouts![
        (4, 2) => [
            3 => (_4_2_3, 99, (507, 2_134_923)),
        ],
        (2, 4) => [
            3 => (_2_4_3, TREE_LIM, (3085, 1_698_850)),
        ],
    ];
}

/**************************************/

fn _2_2_0(prog: &Prog<2, 2>, _: PassConfig<'_>) -> bool {
    prog.graph_cant_halt()
}

fn _2_2_1(prog: &Prog<2, 2>, _: PassConfig<'_>) -> bool {
    prog.bkw_cant_spinout(2).is_refuted()
}

fn _2_2_2(prog: &Prog<2, 2>, _: PassConfig<'_>) -> bool {
    prog.bkw_cant_blank(2).is_refuted() || prog.ctl_cant_blank(14)
}

fn _2_2_3(prog: &Prog<2, 2>, _: PassConfig<'_>) -> bool {
    prog.bkw_cant_twostep(0).is_refuted()
}

fn _3_2_0(prog: &Prog<3, 2>, _: PassConfig<'_>) -> bool {
    prog.graph_cant_halt()
        || prog.bkw_cant_halt(13).is_refuted()
        || prog.ctl_cant_halt(20)
        || prog.cps_cant_halt(3)
}

fn _3_2_1(prog: &Prog<3, 2>, _: PassConfig<'_>) -> bool {
    prog.bkw_cant_spinout(7).is_refuted()
        || prog.ctl_cant_spinout(40)
        || prog.cps_cant_spinout(4)
        || prog.far_cant_spinout(3)
}

fn _3_2_2(prog: &Prog<3, 2>, _: PassConfig<'_>) -> bool {
    prog.graph_cant_blank()
        || prog.bkw_cant_blank(20).is_refuted()
        || prog.ctl_cant_blank(42)
        || prog.cps_cant_blank(10)
        || prog.far_cant_blank(3)
}

fn _2_3_0(prog: &Prog<2, 3>, _: PassConfig<'_>) -> bool {
    prog.graph_cant_halt()
        || prog.bkw_cant_halt(8).is_refuted()
        || prog.cps_cant_halt(3)
}

fn _2_3_2(prog: &Prog<2, 3>, _: PassConfig<'_>) -> bool {
    prog.graph_cant_blank()
        || prog.bkw_cant_blank(16).is_settled()
        || prog.ctl_cant_blank(50)
        || prog.cps_cant_blank(7)
        || prog.far_cant_blank(3)
}

fn _4_2_0(prog: &Prog<4, 2>, _: PassConfig<'_>) -> bool {
    prog.graph_cant_halt()
        || prog.bkw_cant_halt(46).is_refuted()
        || prog.ctl_cant_halt(130)
        || prog.cps_cant_halt(6)
        || prog.far_cant_halt(3)
}

fn _2_4_0(prog: &Prog<2, 4>, _: PassConfig<'_>) -> bool {
    prog.graph_cant_halt()
        || prog.ctl_cant_halt(190)
        || prog.cps_cant_halt(6)
        || prog.far_cant_halt(3)
        || prog.to_string() == "1RB 2LA 1RA 1RA  1LB 1LA 3RB ..."
}

fn instrs_4(prog: &Prog<4, 4>, _: PassConfig<'_>) -> bool {
    prog.bkw_cant_halt(0).is_refuted() || prog.ctl_cant_halt(11)
}

fn instrs_5(prog: &Prog<5, 5>, _: PassConfig<'_>) -> bool {
    prog.bkw_cant_halt(3).is_refuted() || prog.cps_cant_halt(3)
}

fn instrs_6(prog: &Prog<6, 6>, _: PassConfig<'_>) -> bool {
    prog.ctl_cant_halt(41) || prog.far_cant_halt(4)
}

fn test_solved() {
    println!("solved");

    assert_holdouts![
        (2, 2) => [
            0 => (_2_2_0, 2, (0, 23)),
            1 => (_2_2_1, 4, (0, 32)),
            2 => (_2_2_2, 4, (0, 53)),
        ],
        (3, 2) => [
            0 => (_3_2_0, 12, (0, 2_718)),
            1 => (_3_2_1, 13, (0, 4_046)),
            2 => (_3_2_2, 13, (0, 9_510)),
        ],
        (2, 3) => [
            0 => (_2_3_0, 7, (0, 2_335)),
            2 => (_2_3_2, 20, (0, 5_959)),
        ],
        (4, 2) => [
            0 => (_4_2_0, 25, (0, 431_888)),
        ],
        (2, 4) => [
            0 => (_2_4_0, 109, (0, 308_968)),
        ],
    ];

    assert_holdouts![
        4 => (instrs_4, 4, (0, 4_909)),
        5 => (instrs_5, 12, (0, 151_351)),
        6 => (instrs_6, 22, (0, 5_568_167)),
    ];
}

/**************************************/

fn prover_2_2(prog: &Prog<2, 2>, _: PassConfig<'_>) -> bool {
    prog.check_inf(100, 50)
}

fn prover_3_2(prog: &Prog<3, 2>, _: PassConfig<'_>) -> bool {
    prog.check_inf(500, 50)
}

fn prover_2_3(prog: &Prog<2, 3>, _: PassConfig<'_>) -> bool {
    prog.check_inf(1_000, 50)
}

fn prover_4_2(prog: &Prog<4, 2>, _: PassConfig<'_>) -> bool {
    if !prog.is_connected() {
        return true;
    }

    prog.check_inf(3_000, 200)
}

fn prover_2_4(prog: &Prog<2, 4>, _: PassConfig<'_>) -> bool {
    prog.check_inf(3_000, 200)
}

fn test_prover() {
    println!("prover");

    assert_holdouts![
        (2, 2) => [
            3 => (prover_2_2, 4, (1, 81)),
        ],
        (3, 2) => [
            3 => (prover_3_2, 13, (25, 11_754)),
        ],
        (2, 3) => [
            3 => (prover_2_3, 20, (63, 8_766)),
        ],
        (4, 2) => [
            3 => (prover_4_2, 99, (7_740, 2_134_923)),
        ],
        (2, 4) => [
            3 => (prover_2_4, TREE_LIM, (34_346, 1_698_850)),
        ],
    ];
}

/**************************************/

fn qh_2_2(prog: &Prog<2, 2>, _: PassConfig<'_>) -> bool {
    prog.graph_cant_quasihalt()
}

fn qh_3_2(prog: &Prog<3, 2>, _: PassConfig<'_>) -> bool {
    prog.graph_cant_quasihalt() || prog.cps_cant_quasihalt(4)
}

fn qh_2_3(prog: &Prog<2, 3>, _: PassConfig<'_>) -> bool {
    prog.graph_cant_quasihalt() || prog.cps_cant_quasihalt(4)
}

fn qh_4_2(prog: &Prog<4, 2>, _: PassConfig<'_>) -> bool {
    prog.graph_cant_quasihalt() || prog.cps_cant_quasihalt(5)
}

fn qh_2_4(prog: &Prog<2, 4>, _: PassConfig<'_>) -> bool {
    prog.graph_cant_quasihalt() || prog.cps_cant_quasihalt(5)
}

fn test_quasihalt() {
    println!("quasihalt");

    assert_holdouts![
        (2, 2) => [
            3 => (qh_2_2, 4, (0, 81)),
        ],
        (3, 2) => [
            3 => (qh_3_2, 13, (390, 11_754)),
        ],
        (2, 3) => [
            3 => (qh_2_3, 20, (470, 8_766)),
        ],
        (4, 2) => [
            3 => (qh_4_2, 99, (126_019, 2_134_923)),
        ],
        (2, 4) => [
            3 => (qh_2_4, TREE_LIM, (148_279, 1_698_850)),
        ],
    ];
}

/**************************************/

fn _5_2_0(prog: &Prog<5, 2>, _: PassConfig<'_>) -> bool {
    !prog.is_connected() || prog.bkw_cant_halt(3).is_refuted()
}

fn _5_2_1(prog: &Prog<5, 2>, _: PassConfig<'_>) -> bool {
    !prog.is_connected() || prog.bkw_cant_spinout(3).is_refuted()
}

fn _5_2_2(prog: &Prog<5, 2>, _: PassConfig<'_>) -> bool {
    !prog.is_connected() || prog.bkw_cant_blank(3).is_refuted()
}

fn _3_3_0(prog: &Prog<3, 3>, _: PassConfig<'_>) -> bool {
    !prog.is_connected() || prog.bkw_cant_halt(3).is_refuted()
}

fn _3_3_1(prog: &Prog<3, 3>, _: PassConfig<'_>) -> bool {
    !prog.is_connected() || prog.bkw_cant_spinout(3).is_refuted()
}

fn _3_3_2(prog: &Prog<3, 3>, _: PassConfig<'_>) -> bool {
    !prog.is_connected() || prog.bkw_cant_blank(3).is_refuted()
}

fn _2_5_0(prog: &Prog<2, 5>, _: PassConfig<'_>) -> bool {
    prog.bkw_cant_halt(3).is_refuted()
}

fn _2_5_1(prog: &Prog<2, 5>, _: PassConfig<'_>) -> bool {
    prog.bkw_cant_spinout(3).is_refuted()
}

fn _2_5_2(prog: &Prog<2, 5>, _: PassConfig<'_>) -> bool {
    prog.bkw_cant_blank(3).is_refuted()
}

fn test_deciders_slow() {
    println!("deciders slow");

    assert_holdouts![
        (5, 2) => [
            0 => (_5_2_0, 700, (1_396_770, 90_676_712)),
            1 => (_5_2_1, TREE_LIM, (2_887_548, 180_764_612)),
            2 => (_5_2_2, TREE_LIM, (9_549_864, 486_399_920)),
        ],
        (3, 3) => [
            0 => (_3_3_0, 2_700, (769_835, 24_003_381)),
            1 => (_3_3_1, 3_000, (1_744_815, 50_932_166)),
            2 => (_3_3_2, 3_000, (2_634_833, 123_182_486)),
        ],
        (2, 5) => [
            0 => (_2_5_0, TREE_LIM, (4_289_417, 69_763_571)),
            1 => (_2_5_1, TREE_LIM, (14_470_412, 162_767_964)),
            2 => (_2_5_2, TREE_LIM, (9_698_645, 366_717_085)),
        ],
    ];
}

/**************************************/

use std::{
    fs::File,
    io::{self, BufRead as _, BufReader},
};

use tm::parse;

fn run_from_file(path: &str, steps: Steps) -> io::Result<()> {
    println!("running {path}");

    let file = File::open(path)?;
    let reader = BufReader::new(file);

    reader.lines().par_bridge().try_for_each(|line| {
        let line = line?;
        let line = line.as_str();

        // println!(
        //     "thread {:?}: {line}",
        //     rayon::current_thread_index().unwrap(),
        // );

        parse!(line, |prog: Prog<_, _>| prog.check_inf(steps, 4_000));

        Ok(())
    })
}

fn test_from_file() {
    let files = ["halt", "blank", "spinout"];

    for file in files {
        let filename = format!("test/data/holdouts/{file}.prog");

        run_from_file(&filename, 10_000).unwrap();
    }
}

/**************************************/

fn instrs_7(prog: &Prog<7, 7>, _: PassConfig<'_>) -> bool {
    prog.bkw_cant_halt(20).is_refuted()
        || prog.ctl_cant_halt(200)
        || prog.cps_cant_halt(6)
        || prog.far_cant_halt(4)
}

fn test_instrs() {
    println!("instrs");

    assert_holdouts![
        7 => (instrs_7, 109, (_7_0_, 246_492_765)),
    ];
}

fn instrs_8(prog: &Prog<8, 8>, mut config: PassConfig<'_>) -> bool {
    prog.graph_cant_halt()
        || prog.bkw_cant_halt(30).is_settled()
        || prog.ctl_cant_halt(300)
        || prog.term_or_rec(LIN_REC, config.to_mut()).is_settled()
        || prog.cps_cant_halt(20)
        || prog.far_cant_halt(4)
}

fn test_8_instr() {
    println!("8 instrs");

    assert_holdouts![
        8 => (instrs_8, 500, (586, 12_835_863_274)),
    ];
}

fn test_9_instr() {
    println!("9 instrs");

    assert_eq!(
        Visited::<9, 9>::run_instrs::<9>(1000, &Visited::new),
        777_451_944_058,
    );
}

/**************************************/

const CURR: &[fn()] = &[test_instrs];

const FAST: &[fn()] = &[
    test_deciders,
    test_from_file,
    test_holdouts,
    test_prover,
    test_quasihalt,
    test_solved,
    test_twostep,
];

const SLOW: &[fn()] = &[
    test_8_instr,
    test_deciders_slow,
    // test_9_instr,
];

fn main() {
    CURR.par_iter().for_each(|f| f());

    if !std::env::args().any(|x| x == "--all") {
        return;
    }

    FAST.par_iter().for_each(|f| f());

    SLOW.iter().for_each(|f| f());
}
