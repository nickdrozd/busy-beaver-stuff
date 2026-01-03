use std::env;

use tm::{Goal, Prog, Steps};

pub mod harvesters;

use harvesters::{
    Collector, Harvester as _, HoldoutVisited, PassConfig,
    ReasonHarvester,
};

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

macro_rules! assert_params {
    ( $( ($states:literal, $colors:literal) => [ $( $goal:literal => ( $pipeline:ident, $steps:expr, $leaves:expr ) ),* $(,)? ] ),* $(,)? ) => {{
        rayon::scope(|s| { $( $( s.spawn(move |_| {
            let result = HoldoutVisited::<$states, $colors>::run_params(
                get_goal($goal),
                $steps,
                &|| HoldoutVisited::new($pipeline),
            );

            assert_eq!(
                result, $leaves,
                "(({}, {}), {}, {result:?})",
                $states, $colors, $goal,
            );
        }); )* )* });
    }};
}

fn params_2_2_0(prog: &Prog<2, 2>, _: PassConfig<'_>) -> bool {
    prog.cant_halt(3).is_refuted() || prog.ctl_cant_halt(11)
}

fn params_2_2_1(prog: &Prog<2, 2>, _: PassConfig<'_>) -> bool {
    prog.cant_spin_out(2).is_refuted()
}

fn params_2_2_2(prog: &Prog<2, 2>, _: PassConfig<'_>) -> bool {
    prog.cant_blank(2).is_refuted() || prog.ctl_cant_blank(14)
}

fn params_2_2_3(prog: &Prog<2, 2>, mut config: PassConfig<'_>) -> bool {
    prog.term_or_rec(16, config.to_mut()).is_settled()
}

//

fn params_3_2_0(prog: &Prog<3, 2>, _: PassConfig<'_>) -> bool {
    prog.cant_halt(13).is_refuted()
        || prog.ctl_cant_halt(20)
        || prog.cps_cant_halt(4)
}

fn params_3_2_1(prog: &Prog<3, 2>, _: PassConfig<'_>) -> bool {
    prog.cant_spin_out(7).is_refuted()
        || prog.ctl_cant_spin_out(40)
        || prog.cps_cant_spin_out(6)
}
fn params_3_2_2(prog: &Prog<3, 2>, _: PassConfig<'_>) -> bool {
    prog.cant_blank(14).is_refuted()
        || prog.ctl_cant_blank(42)
        || prog.cps_cant_blank(10)
}
fn params_3_2_3(prog: &Prog<3, 2>, mut config: PassConfig<'_>) -> bool {
    prog.term_or_rec(190, config.to_mut()).is_settled()
        || prog.check_inf(500, 50)
}

//

fn params_2_3_0(prog: &Prog<2, 3>, _: PassConfig<'_>) -> bool {
    prog.cant_halt(8).is_refuted()
        || prog.ctl_cant_halt(50)
        || prog.cps_cant_halt(4)
}

fn params_2_3_1(prog: &Prog<2, 3>, _: PassConfig<'_>) -> bool {
    prog.cant_spin_out(2).is_refuted()
        || prog.ctl_cant_spin_out(100)
        || prog.cps_cant_spin_out(5)
        || prog.seg_cant_spin_out(7).is_refuted()
}

fn params_2_3_2(prog: &Prog<2, 3>, _: PassConfig<'_>) -> bool {
    prog.cant_blank(16).is_settled()
        || prog.ctl_cant_blank(50)
        || prog.cps_cant_blank(9)
}

fn params_2_3_3(prog: &Prog<2, 3>, mut config: PassConfig<'_>) -> bool {
    prog.term_or_rec(290, config.to_mut()).is_settled()
        || prog.check_inf(1_000, 50)
}

//

fn params_4_2_0(prog: &Prog<4, 2>, _: PassConfig<'_>) -> bool {
    prog.cant_halt(46).is_refuted()
        || prog.ctl_cant_halt(130)
        || prog.cps_cant_halt(8)
}

fn params_4_2_1(prog: &Prog<4, 2>, _: PassConfig<'_>) -> bool {
    prog.cant_spin_out(15).is_refuted()
        || prog.ctl_cant_spin_out(190)
        || prog.cps_cant_spin_out(15)
        || prog.seg_cant_spin_out(8).is_refuted()
}

fn params_4_2_2(prog: &Prog<4, 2>, _: PassConfig<'_>) -> bool {
    prog.cant_blank(51).is_refuted()
        || prog.ctl_cant_blank(130)
        || prog.cps_cant_blank(20)
}

fn params_4_2_3(prog: &Prog<4, 2>, mut config: PassConfig<'_>) -> bool {
    if !prog.is_connected() {
        return true;
    }

    let config = config.to_mut();

    prog.term_or_rec(500, config).is_settled()
        || prog.check_inf(2_000, 200)
        || prog.term_or_rec(4_710, config).is_settled()
}

//

fn params_2_4_0(prog: &Prog<2, 4>, _: PassConfig<'_>) -> bool {
    prog.cant_halt(17).is_refuted()
        || prog.ctl_cant_halt(190)
        || prog.cps_cant_halt(12)
}

fn params_2_4_1(prog: &Prog<2, 4>, _: PassConfig<'_>) -> bool {
    prog.cant_spin_out(8).is_refuted()
        || prog.ctl_cant_spin_out(300)
        || prog.cps_cant_spin_out(15)
        || prog.seg_cant_spin_out(5).is_refuted()
}

fn params_2_4_2(prog: &Prog<2, 4>, _: PassConfig<'_>) -> bool {
    prog.cant_blank(51).is_refuted()
        || prog.ctl_cant_blank(200)
        || prog.cps_cant_blank(20)
}

fn params_2_4_3(prog: &Prog<2, 4>, mut config: PassConfig<'_>) -> bool {
    let config = config.to_mut();

    prog.term_or_rec(500, config).is_settled()
        || prog.check_inf(1_000, 200)
        || prog.term_or_rec(4_600, config).is_settled()
}

fn test_deciders() {
    println!("deciders");

    assert_params![
        (2, 2) => [
            0 => (params_2_2_0, 2, (9, 23)),
            1 => (params_2_2_1, 4, (5, 32)),
            2 => (params_2_2_2, 4, (5, 53)),
        ],
        (3, 2) => [
            0 => (params_3_2_0, 12, (842, 2_721)),
            1 => (params_3_2_1, 13, (476, 4_050)),
            2 => (params_3_2_2, 13, (632, 9_513)),
        ],
        (2, 3) => [
            0 => (params_2_3_0, 7, (546, 2_335)),
            1 => (params_2_3_1, 20, (532, 3_510)),
            2 => (params_2_3_2, 20, (145, 5_962)),
        ],
        (4, 2) => [
            0 => (params_4_2_0, 25, (114_653, 432_318)),
            1 => (params_4_2_1, 99, (84_773, 754_707)),
            2 => (params_4_2_2, 99, (98_249, 1_933_882)),
        ],
        (2, 4) => [
            0 => (params_2_4_0, 109, (87_281, 309_759)),
            1 => (params_2_4_1, TREE_LIM, (87_791, 613_031)),
            2 => (params_2_4_2, TREE_LIM, (15_736, 1_190_832)),
        ],
    ];
}

fn test_prover() {
    println!("prover");

    assert_params![
        (2, 2) => [
            3 => (params_2_2_3, 4, (4, 81)),
        ],
        (3, 2) => [
            3 => (params_3_2_3, 13, (25, 11_758)),
        ],
        (2, 3) => [
            3 => (params_2_3_3, 20, (63, 8_771)),
        ],
        (4, 2) => [
            3 => (params_4_2_3, 99, (7_944, 2_135_991)),
        ],
        (2, 4) => [
            3 => (params_2_4_3, TREE_LIM, (39_623, 1_699_887)),
        ],
    ];
}

fn params_5_2_0(prog: &Prog<5, 2>, _: PassConfig<'_>) -> bool {
    !prog.is_connected() || prog.cant_halt(2).is_refuted()
}

fn params_5_2_1(prog: &Prog<5, 2>, _: PassConfig<'_>) -> bool {
    !prog.is_connected() || prog.cant_spin_out(2).is_refuted()
}

fn params_5_2_2(prog: &Prog<5, 2>, _: PassConfig<'_>) -> bool {
    !prog.is_connected() || prog.cant_blank(2).is_refuted()
}

//

fn params_3_3_0(prog: &Prog<3, 3>, _: PassConfig<'_>) -> bool {
    !prog.is_connected() || prog.cant_halt(2).is_refuted()
}

fn params_3_3_1(prog: &Prog<3, 3>, _: PassConfig<'_>) -> bool {
    !prog.is_connected() || prog.cant_spin_out(2).is_refuted()
}

fn params_3_3_2(prog: &Prog<3, 3>, _: PassConfig<'_>) -> bool {
    !prog.is_connected() || prog.cant_blank(2).is_refuted()
}

//

fn params_2_5_0(prog: &Prog<2, 5>, _: PassConfig<'_>) -> bool {
    prog.cant_halt(2).is_refuted()
}

fn params_2_5_1(prog: &Prog<2, 5>, _: PassConfig<'_>) -> bool {
    prog.cant_spin_out(2).is_refuted()
}

fn params_2_5_2(prog: &Prog<2, 5>, _: PassConfig<'_>) -> bool {
    prog.cant_blank(2).is_refuted()
}

fn test_params_slow() {
    println!("params slow");

    assert_params![
        (5, 2) => [
            0 => (params_5_2_0, 700, (60_238_535, 90_773_891)),
            1 => (params_5_2_1, TREE_LIM, (69_543_346, 181_095_466)),
            2 => (params_5_2_2, TREE_LIM, (249_954_945, 486_712_056)),
        ],
        (3, 3) => [
            0 => (params_3_3_0, 2_700, (19_664_183, 24_057_699)),
            1 => (params_3_3_1, 3_000, (25_352_963, 51_028_928)),
            2 => (params_3_3_2, 3_000, (43_955_136, 123_294_779)),
        ],
        (2, 5) => [
            0 => (params_2_5_0, TREE_LIM, (62_781_237, 69_999_829)),
            1 => (params_2_5_1, TREE_LIM, (131_625_721, 163_068_753)),
            2 => (params_2_5_2, TREE_LIM, (111_406_075, 367_061_568)),
        ],
    ];
}

/**************************************/

macro_rules! assert_reason {
    ( $( ($states:literal, $colors:literal) => [ $( $goal:literal => $leaves:expr ),* $(,)? ] ),* $(,)? ) => {{
        rayon::scope(|s| { $( $( s.spawn(move |_| {
            let cant_reach = match $goal {
                0 => Prog::cant_halt,
                1 => Prog::cant_spin_out,
                2 => Prog::cant_blank,
                _ => unreachable!(),
            };

            let result = ReasonHarvester::<$states, $colors>::run_params(
                get_goal($goal),
                TREE_LIM,
                &|| ReasonHarvester::new(cant_reach),
            );

            assert_eq!(
                result, $leaves,
                "(({}, {}), {}, {result:?})",
                $states, $colors, $goal,
            );
        }); )* )*});
    }};
}

fn test_reason() {
    println!("reason");

    assert_reason![
        (2, 2) => [
            0 => (3, 12),
            1 => (2, 5),
            2 => (2, 10),
        ],
        (3, 2) => [
            0 => (13, 1_364),
            1 => (13, 672),
            2 => (20, 1_687),
        ],
        (2, 3) => [
            0 => (8, 1_159),
            1 => (7, 1_015),
            2 => (15, 1_024),
        ],
        (4, 2) => [
            0 => (46, 233_972),
            1 => (36, 145_975),
            2 => (79, 376_066),
        ],
        (2, 4) => [
            0 => (20, 223_044),
            1 => (14, 287_062),
            2 => (100, 220_762),
        ],
    ];
}

/**************************************/

fn test_collect() {
    println!("collect");

    let result =
        Collector::<2, 2>::run_params(None, 4, &Collector::new);

    assert_eq!(result.len(), 81);
}

/**************************************/

macro_rules! assert_instrs {
    ( $( $instrs:literal => ($pipeline:ident, $steps:literal, $leaves:expr) ),* $(,)? ) => {{
        rayon::scope(|s| { $( s.spawn(move |_| {
            let result = HoldoutVisited::<$instrs, $instrs>::run_instrs::<$instrs>(
                $steps,
                &|| HoldoutVisited::new($pipeline),
            );

            assert_eq!(
                result, $leaves,
                "({}, {result:?})",
                $instrs,
            );
        }); )* });
    }};
}

fn instrs_4(prog: &Prog<4, 4>, mut config: PassConfig<'_>) -> bool {
    prog.term_or_rec(16, config.to_mut()).is_settled()
        || prog.cant_halt(0).is_refuted()
        || prog.ctl_cant_halt(11)
}

fn instrs_5(prog: &Prog<5, 5>, mut config: PassConfig<'_>) -> bool {
    prog.term_or_rec(301, config.to_mut()).is_settled()
        || prog.cant_halt(2).is_refuted()
        || prog.cps_cant_halt(3)
}

fn instrs_6(prog: &Prog<6, 6>, mut config: PassConfig<'_>) -> bool {
    prog.term_or_rec(304, config.to_mut()).is_settled()
        || prog.cant_halt(2).is_refuted()
        || prog.ctl_cant_halt(41)
        || prog.cps_cant_halt(3)
}

fn instrs_7(prog: &Prog<7, 7>, mut config: PassConfig<'_>) -> bool {
    let config = config.to_mut();

    prog.term_or_rec(100, config).is_settled()
        || prog.cant_halt(2).is_refuted()
        || prog.ctl_cant_halt(51)
        || prog.cps_cant_halt(7)
        || prog.term_or_rec(1_000, config).is_settled()
}

fn test_instrs() {
    println!("instrs");

    assert_instrs![
        4 => (instrs_4, 4, (0, 4_909)),
        5 => (instrs_5, 12, (0, 151_351)),
        6 => (instrs_6, 22, (3, 5_568_167)),
        7 => (instrs_7, 109, (216, 246_492_765)),
    ];
}

fn instrs_8(prog: &Prog<8, 8>, mut config: PassConfig<'_>) -> bool {
    let config = config.to_mut();

    prog.term_or_rec(100, config).is_settled()
        || prog.cant_halt(3).is_settled()
        || prog.ctl_cant_halt(300)
        || prog.cps_cant_halt(20)
        || prog.term_or_rec(1_000, config).is_settled()
}

fn test_8_instr() {
    println!("8 instrs");

    assert_instrs![
        8 => (instrs_8, 500, (12_597, 12_835_863_274)),
    ];
}

/**************************************/

const TESTS: [fn(); 4] =
    [test_collect, test_reason, test_instrs, test_deciders];

use rayon::prelude::*;

fn main() {
    test_prover();

    if !env::args().any(|x| x == "--all") {
        return;
    }

    TESTS.par_iter().for_each(|f| f());

    test_8_instr();
    test_params_slow();
}
