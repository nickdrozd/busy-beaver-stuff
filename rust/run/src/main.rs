#![allow(dead_code, clippy::wildcard_imports)]
#![expect(clippy::used_underscore_items, clippy::needless_for_each)]
use rayon::prelude::*;

use tm::{Goal, Instr, Prog, Steps, instrs::Parse as _};

pub mod check;
pub mod harvesters;
pub mod holdouts;
pub mod tree;

use check::{assert_holdouts_match, test_holdouts};
use harvesters::{Collector, HoldoutVisited, MultiCollector, Visited};
use holdouts::*;
use tree::{Harvester as _, PassConfig};

/**************************************/

const TREE_LIM: Steps = 876;

const LIN_MIN: Steps = 4_000;
const LIN_MOR: Steps = 10_000;
const LIN_MAX: Steps = 5_000_000;

const INF_MIN: Steps = 1_000;
const INF_MOR: Steps = 100_000;

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

macro_rules! assert_visited {
    ( $instrs:literal => [
        $steps:expr,
        $total:expr,
        [ $( $instr:literal => $visited:expr ),* $(,)? ],
        $(,)?
    ] ) => {{
        let (total, by_instr) =
            Visited::<$instrs, $instrs>::run_instrs::<$instrs>(
                $steps,
                &Visited::new,
            );

        assert_visited!(
            @check
            $instrs.to_string(),
            total,
            by_instr,
            $total,
            [ $( $instr => $visited ),* ]
        );
    }};

    ( $( ($states:literal, $colors:literal) => [
        $( $goal:literal => [
            $steps:expr,
            $total:expr,
            [ $( $instr:literal => $visited:expr ),* $(,)? ],
            $(,)?
        ] ),* $(,)?
    ] ),* $(,)? ) => {{
        rayon::scope(|scope| {
            $(
                $(
                    scope.spawn(move |_| {
                        let (total, by_instr) =
                            Visited::<$states, $colors>::run_params(
                                get_goal($goal),
                                $steps,
                                &Visited::new,
                            );

                        assert_visited!(
                            @check
                            format!("(({}, {}), {})", $states, $colors, $goal),
                            total,
                            by_instr,
                            $total,
                            [ $( $instr => $visited ),* ]
                        );
                    });
                )*
            )*
        });
    }};

    (@check
        $label:expr,
        $total:expr,
        $by_instr:expr,
        $expected_total:expr,
        [ $( $instr:literal => $visited:expr ),* $(,)? ]
    ) => {{
        let expected = std::collections::HashMap::<Instr, u64>::from([
            $( (Instr::read($instr), $visited), )*
        ]);

        if $total != $expected_total || $by_instr != expected {
            let mut actual = $by_instr.into_iter().collect::<Vec<_>>();
            actual.sort_by_key(|(_, visited)| *visited);

            let actual = actual
                .into_iter()
                .map(|(instr, visited)| {
                    format!("    \"{}\" => {},", instr.show(), show_num(visited))
                })
                .collect::<Vec<_>>()
                .join("\n");

            panic!(
                "{} visited mismatch; actual:\n{},\n[\n{}\n]",
                $label,
                show_num($total),
                actual,
            );
        }
    }};
}

macro_rules! assert_holdouts {
    ( $( ($states:literal, $colors:literal) => [ $( $goal:literal => ( $pipeline:ident, $steps:expr, ( $first:tt, $visited:expr ) ) ),* $(,)? ] ),* $(,)? ) => {{
        rayon::scope(|s| { $( $( assert_holdouts!(@goal s, $states, $colors, $goal, $pipeline, $steps, $first, $visited); )* )* });
    }};

    ( $( $instrs:literal => [
        $steps:expr,
        $visited:expr,
        [ $( $goal:tt => $case:tt ),* $(,)? ],
        $(,)?
    ] ),* $(,)? ) => {{
        rayon::scope(|s| {
            $(
                assert_holdouts!(
                    @instrs_run
                    s,
                    $instrs,
                    $steps,
                    $visited,
                    &|| MultiCollector::new(
                        |prog, config| {
                            prog.term_or_rec(LIN_MIN, config.to_mut()).is_settled()
                        },
                        [ $( |prog| match $goal {
                            0 => prog.bkw_cant_halt(BKW_8).is_refuted(),
                            1 => prog.bkw_cant_spinout(BKW_8).is_refuted(),
                            2 => prog.bkw_cant_blank(BKW_8).is_refuted(),
                            _ => unreachable!(),
                        } ),* ],
                        |prog, config| {
                            prog.term_or_rec(LIN_MOR, config.to_mut()).is_settled()
                                || prog.prover_settled(INF_MIN)
                        },
                        [ $( |prog| match $goal {
                            0 => prog.cps_cant_halt(CPS_8),
                            1 => prog.cps_cant_spinout(CPS_8),
                            2 => prog.cps_cant_blank(CPS_8),
                            _ => unreachable!(),
                        } ),* ],
                        |prog, config| {
                            prog.term_or_rec(LIN_MAX, config.to_mut()).is_settled()
                                || prog.prover_settled(INF_MOR)
                        },
                        [ $( |prog| match $goal {
                            0 => prog.far_cant_halt(FAR_8),
                            1 => prog.far_cant_spinout(FAR_8),
                            2 => prog.far_cant_blank(FAR_8),
                            _ => unreachable!(),
                        } ),* ],
                    ),
                    [ $( $goal => $case ),* ]
                );
            )*
        });
    }};

    (@goal $scope:ident, $states:literal, $colors:literal, $goal:literal, $pipeline:ident, $steps:expr, $holdouts:ident, $visited:expr) => {{
        $scope.spawn(move |_| {
            let (champs, holdouts) = $holdouts;

            let (result, visited) = Collector::<$states, $colors>::run_params(
                get_goal($goal),
                $steps,
                &|| Collector::new(|prog, config| {
                    prog.term_or_rec(LIN_MIN, config.to_mut()).is_settled()
                        || $pipeline(prog, config)
                }),
            );

            assert_holdouts_match(
                format!("(({}, {}), {})", $states, $colors, $goal),
                champs,
                holdouts,
                result,
            );

            if visited != $visited {
                panic!(
                    "(({}, {}), {}) visited mismatch:\n  old: {}\n  new: {}",
                    $states,
                    $colors,
                    $goal,
                    show_num($visited),
                    show_num(visited),
                );
            }
        });
    }};

    (@goal $scope:ident, $states:literal, $colors:literal, $goal:literal, $pipeline:ident, $steps:expr, $leaves:literal, $visited:expr) => {{
        $scope.spawn(move |_| {
            let result = HoldoutVisited::<$states, $colors>::run_params(
                get_goal($goal),
                $steps,
                &|| HoldoutVisited::new(|prog, config| {
                    prog.term_or_rec(LIN_MIN, config.to_mut()).is_settled()
                        || $pipeline(prog, config)
                }),
            );

            if result != ($leaves, $visited) {
                panic!(
                    "(({}, {}), {}) result mismatch:\n  old: ({}, {})\n  new: ({}, {})",
                    $states,
                    $colors,
                    $goal,
                    show_num($leaves),
                    show_num($visited),
                    show_num(result.0),
                    show_num(result.1),
                );
            }
        });
    }};

    (@instrs_run
        $scope:ident,
        $instrs:literal,
        $steps:expr,
        $visited:expr,
        $harvester:expr,
        [ $( $goal:tt => $case:tt ),* $(,)? ]
    ) => {{
        $scope.spawn(move |_| {
            let (result, visited) = MultiCollector::<
                $instrs,
                $instrs,
                { [$(stringify!($case)),*].len() },
            >::run_instrs::<$instrs>($steps, $harvester);

            if visited != $visited {
                panic!(
                    "{} visited mismatch:\n  old: {}\n  new: {}",
                    $instrs,
                    show_num($visited),
                    show_num(visited),
                );
            }

            let mut results = result.into_iter();
            let mut failed = false;

            $(
                let result = results.next().expect("missing multi-collector result");
                failed |= std::panic::catch_unwind(
                    core::panic::AssertUnwindSafe(|| {
                        assert_holdouts!(@expected $instrs, $goal, result, $case);
                    }),
                )
                .is_err();
            )*

            assert!(
                results.next().is_none(),
                "extra multi-collector results for {}",
                $instrs,
            );
            assert!(!failed, "multi-collector holdout mismatch for {}", $instrs);
        });
    }};

    (@expected $instrs:literal, $goal:tt, $result:ident, $leaves:literal) => {{
        if $result.len() != $leaves {
            panic!(
                "{}:{} count mismatch:\n  old: {}\n  new: {}{}",
                $instrs,
                $goal,
                show_num($leaves),
                show_num($result.len() as u64),
                if $result.len() < 50 {
                    format!(", {result:?}", result = $result)
                } else {
                    String::new()
                },
            );
        }
    }};

    (@expected $instrs:literal, $goal:tt, $result:ident, $holdouts:ident) => {{
        let (champs, holdouts) = $holdouts;
        assert_holdouts_match(
            format!("{}:{}", $instrs, $goal),
            champs,
            holdouts,
            $result,
        );
    }};
}

macro_rules! assert_bkw {
    ( $( $instrs:literal => [
        $steps:expr,
        $visited:expr,
        [
            0 => $halt:tt,
            1 => $spinout:tt,
            2 => $blank:tt,
            3 => $twostep:tt,
        ],
    ] ),* $(,)? ) => {{
        rayon::scope(|s| {
            $(
                assert_holdouts!(
                    @instrs_run
                    s,
                    $instrs,
                    $steps,
                    $visited,
                    &|| MultiCollector::new(
                        |prog, config| {
                            prog.term_or_rec(LIN_MIN, config.to_mut()).is_settled()
                        },
                        [
                            |prog| prog.bkw_cant_halt(BKW).is_refuted(),
                            |prog| prog.bkw_cant_spinout(BKW).is_refuted(),
                            |prog| prog.bkw_cant_blank(BKW).is_refuted(),
                            |prog| prog.bkw_cant_twostep(BKW).is_refuted(),
                        ],
                        |_, _| false,
                        [|_| false, |_| false, |_| false, |_| false],
                        |_, _| false,
                        [|_| false, |_| false, |_| false, |_| false],
                    ),
                    [
                        0 => $halt,
                        1 => $spinout,
                        2 => $blank,
                        3 => $twostep,
                    ]
                );
            )*
        });
    }};
}

#[expect(clippy::string_slice, clippy::sliced_string_as_bytes)]
fn show_num(n: u64) -> String {
    let s = n.to_string();
    let first = s.len() % 3;
    let mut out = String::new();

    if first != 0 {
        out.push_str(&s[..first]);
    }

    for chunk in s[first..].as_bytes().chunks(3) {
        if !out.is_empty() {
            out.push('_');
        }

        out.push_str(core::str::from_utf8(chunk).unwrap());
    }

    out
}

/**************************************/

fn _4_2_1(prog: &Prog<4, 2>, config: &mut PassConfig<'_>) -> bool {
    prog.bkw_cant_spinout(22).is_refuted()
        || prog.term_or_rec(LIN_MOR, config.to_mut()).is_settled()
        || prog.prover_settled(INF_MIN)
        || prog.cps_cant_spinout(21)
        || prog.term_or_rec(LIN_MAX, config.to_mut()).is_settled()
        || prog.prover_settled(INF_MOR)
        || prog.far_cant_spinout(3)
}

fn _4_2_2(prog: &Prog<4, 2>, config: &mut PassConfig<'_>) -> bool {
    prog.bkw_cant_blank(51).is_refuted()
        || prog.term_or_rec(LIN_MOR, config.to_mut()).is_settled()
        || prog.prover_settled(INF_MIN)
        || prog.cps_cant_blank(20)
        || prog.term_or_rec(LIN_MAX, config.to_mut()).is_settled()
        || prog.prover_settled(INF_MOR)
}

fn _2_4_1(prog: &Prog<2, 4>, config: &mut PassConfig<'_>) -> bool {
    prog.bkw_cant_spinout(50).is_refuted()
        || prog.term_or_rec(LIN_MOR, config.to_mut()).is_settled()
        || prog.prover_settled(INF_MIN)
        || prog.cps_cant_spinout(11)
        || prog.term_or_rec(LIN_MAX, config.to_mut()).is_settled()
        || prog.prover_settled(INF_MOR)
        || prog.far_cant_spinout(4)
}

fn _2_4_2(prog: &Prog<2, 4>, config: &mut PassConfig<'_>) -> bool {
    prog.bkw_cant_blank(51).is_refuted()
        || prog.term_or_rec(LIN_MOR, config.to_mut()).is_settled()
        || prog.prover_settled(INF_MIN)
        || prog.cps_cant_blank(20)
        || prog.term_or_rec(LIN_MAX, config.to_mut()).is_settled()
        || prog.prover_settled(INF_MOR)
        || prog.far_cant_blank(6)
}

fn test_deciders() {
    println!("deciders");

    assert_holdouts![
        (4, 2) => [
            1 => (_4_2_1, 99, (0, 443_662)),
            2 => (_4_2_2, 99, (0, 1_794_878)),
            3 => (_4_2_3, 99, (50, 1_989_122)),
        ],
        (2, 4) => [
            1 => (_2_4_1, TREE_LIM, (_2_4_1_, 407_228)),
            2 => (_2_4_2, TREE_LIM, (0, 996_048)),
            3 => (_2_4_3, TREE_LIM, (88, 1_654_050)),
        ],
    ];
}

/**************************************/

const BKW: usize = 256;

fn _4_2_3(prog: &Prog<4, 2>, _: &mut PassConfig<'_>) -> bool {
    prog.bkw_cant_twostep(BKW).is_refuted()
}

fn _2_4_3(prog: &Prog<2, 4>, _: &mut PassConfig<'_>) -> bool {
    prog.bkw_cant_twostep(BKW).is_refuted()
}

fn test_bkw() {
    println!("bkw");

    assert_bkw![
        4 => [
            4,
            1_580,
            [
                0 => 0,
                1 => 0,
                2 => 0,
                3 => 0,
            ],
        ],
        5 => [
            12,
            53_062,
            [
                0 => 5,
                1 => 0,
                2 => 0,
                3 => 0,
            ],
        ],
        6 => [
            22,
            1_864_702,
            [
                0 => 196,
                1 => 16,
                2 => 3,
                3 => 0,
            ],
        ],
        7 => [
            109,
            84_034_227,
            [
                0 => 9066,
                1 => 711,
                2 => 248,
                3 => 15,
            ],
        ],
    ];
}

/**************************************/

const BKW_8: usize = 1000;
const CPS_8: usize = 21;
const FAR_8: usize = 6;

fn test_pipeline_8() {
    println!("pipeline 8 instrs");

    assert_holdouts![
        8 => [
            500,
            4_263_586_820,
            [
                0 => _8_0_,
                1 => _8_1_,
                2 => _8_2_,
            ],
        ],
    ];
}

fn test_enum_8() {
    println!("enum 8 instrs");

    assert_visited![
        8 => [
            500,
            4_263_586_820,
            [
                "0LB" => 53_138_732,
                "0LA" => 59_654_105,
                "1LB" => 82_371_626,
                "1LA" => 112_395_671,
                "0RC" => 246_836_683,
                "0LC" => 259_094_743,
                "2LB" => 273_470_145,
                "1RC" => 279_252_653,
                "1LC" => 381_762_214,
                "2LA" => 413_112_544,
                "2RC" => 891_088_361,
                "2LC" => 1_211_409_343,
            ],
        ]
    ];
}

fn test_enum_9() {
    println!("enum 9 instrs");

    assert_visited![
        9 => [
            1000,
            257_739_622_145,
            [
                "0LB" => 2_773_091_745,
                "0LA" => 3_092_617_635,
                "1LB" => 4_199_032_613,
                "1LA" => 5_721_672_912,
                "0RC" => 14_502_308_067,
                "0LC" => 14_889_491_931,
                "2LB" => 15_377_493_650,
                "1RC" => 16_233_241_150,
                "1LC" => 21_522_151_088,
                "2LA" => 23_183_987_765,
                "2RC" => 59_678_002_107,
                "2LC" => 76_566_531_482,
            ],
        ]
    ];
}

/**************************************/

fn test_enum_p() {
    println!("enum params");

    assert_visited![
        (5, 2) => [
            0 => [
                700,
                77_265_183,
                [
                    "0LB" => 1_510_578,
                    "0LA" => 1_930_690,
                    "1LB" => 3_509_088,
                    "1LA" => 6_610_622,
                    "0LC" => 8_446_283,
                    "0RC" => 13_090_031,
                    "1RC" => 19_752_737,
                    "1LC" => 22_415_154,
                ],
            ],
            1 => [
                TREE_LIM,
                116_571_356,
                [
                    "0LA" => 2_633_280,
                    "0LB" => 4_221_408,
                    "1LB" => 10_523_352,
                    "1LA" => 10_881_909,
                    "0LC" => 11_625_792,
                    "0RC" => 21_576_013,
                    "1RC" => 25_274_418,
                    "1LC" => 29_835_184,
                ],
            ],
            2 => [
                TREE_LIM,
                460_212_126,
                [
                    "0LB" => 7_941_845,
                    "0LA" => 11_332_068,
                    "1LB" => 19_438_894,
                    "1LA" => 41_647_747,
                    "0LC" => 47_179_602,
                    "0RC" => 76_364_689,
                    "1RC" => 121_252_824,
                    "1LC" => 135_054_457,
                ],
            ],
        ],
        (3, 3) => [
            1 => [
                3_000,
                34_207_385,
                [
                    "0LA" => 497_053,
                    "1RC" => 1_249_699,
                    "0LC" => 1_405_973,
                    "0RC" => 2_137_520,
                    "0LB" => 2_237_345,
                    "1LC" => 2_250_731,
                    "2RC" => 2_319_190,
                    "1LA" => 2_697_783,
                    "2LC" => 2_799_561,
                    "2LA" => 4_006_616,
                    "1LB" => 6_212_559,
                    "2LB" => 6_393_355,
                ],
            ],
            2 => [
                3_000,
                115_839_881,
                [
                    "0LB" => 2_630_722,
                    "0LA" => 3_085_688,
                    "0RC" => 3_807_946,
                    "0LC" => 5_489_145,
                    "1LB" => 7_437_396,
                    "2LB" => 7_558_784,
                    "1RC" => 8_580_957,
                    "1LA" => 13_317_517,
                    "2RC" => 13_404_670,
                    "1LC" => 14_738_836,
                    "2LC" => 16_769_788,
                    "2LA" => 19_018_432,
                ],
            ],
        ],
        (2, 5) => [
            0 => [
                TREE_LIM,
                61_844_347,
                [
                    "0LA" => 1_454_181,
                    "0LB" => 1_612_811,
                    "1LB" => 3_548_856,
                    "1LA" => 3_885_533,
                    "2LB" => 15_036_989,
                    "2LA" => 36_305_977,
                ],
            ],
            1 => [
                TREE_LIM,
                135_231_533,
                [
                    "0LB" => 10_183_546,
                    "1LB" => 24_821_461,
                    "2LB" => 100_226_526,
                ],
            ],
            2 => [
                TREE_LIM,
                316_252_529,
                [
                    "0LA" => 6_079_231,
                    "0LB" => 6_259_311,
                    "1LB" => 17_103_110,
                    "1LA" => 18_132_579,
                    "2LB" => 70_257_307,
                    "2LA" => 198_420_991,
                ],
            ],
        ],
        (6, 2) => [
            0 => [
                TREE_LIM,
                21_407_743_808,
                [
                    "0LB" => 319_860_410,
                    "0LA" => 415_502_728,
                    "1LB" => 749_784_432,
                    "1LA" => 1_376_193_227,
                    "0LC" => 2_377_496_859,
                    "0RC" => 4_071_068_386,
                    "1RC" => 5_822_264_863,
                    "1LC" => 6_275_572_903,
                ],
            ],
        ],
        (2, 6) => [
            0 => [
                TREE_LIM,
                20_835_769_791,
                [
                    "0LB" => 400_448_758,
                    "0LA" => 409_691_775,
                    "1LB" => 862_883_989,
                    "1LA" => 995_108_491,
                    "2LB" => 5_055_258_492,
                    "2LA" => 13_112_378_286,
                ],
            ],
        ],
    ];
}

/**************************************/

const FAST: &[fn()] = &[test_bkw, test_deciders];

const SLOW: &[fn()] = &[test_enum_p, test_enum_9, test_pipeline_8];

fn main() {
    test_enum_8();

    if !std::env::args().any(|x| x == "--all") {
        return;
    }

    FAST.par_iter().for_each(|f| f());

    test_holdouts();

    SLOW.par_iter().for_each(|f| f());
}
