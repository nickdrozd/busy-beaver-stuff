#![allow(dead_code, clippy::wildcard_imports)]
#![expect(clippy::used_underscore_items, clippy::needless_for_each)]
use tm::{Goal, Instr, Prog, Steps, instrs::Parse as _};

pub mod check;
pub mod harvesters;
pub mod holdouts;
pub mod tree;

use check::assert_holdouts_match;
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
            1 => (_4_2_1, 99, (0, 414_930)),
            2 => (_4_2_2, 99, (0, 1_703_180)),
            3 => (_4_2_3, 99, (50, 1_872_886)),
        ],
        (2, 4) => [
            1 => (_2_4_1, TREE_LIM, (_2_4_1_, 391_847)),
            2 => (_2_4_2, TREE_LIM, (0, 981_620)),
            3 => (_2_4_3, TREE_LIM, (88, 1_610_547)),
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
            1_438,
            [
                0 => 0,
                1 => 0,
                2 => 0,
                3 => 0,
            ],
        ],
        5 => [
            12,
            50_623,
            [
                0 => 5,
                1 => 0,
                2 => 0,
                3 => 0,
            ],
        ],
        6 => [
            22,
            1_803_307,
            [
                0 => 196,
                1 => 16,
                2 => 3,
                3 => 0,
            ],
        ],
        7 => [
            109,
            81_849_303,
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
            4_169_973_285,
            [
                0 => _8_0_,
                1 => _8_1_,
                2 => _8_2_,
            ],
        ],
    ];
}

// Run the same three-goal pipeline as test_pipeline_8, but only for one
// normalized B0 instruction. There are no precomputed 9-instruction
// holdout sets to compare against, so report each goal's remaining count.
fn test_pipeline_9(second: &str) {
    println!("pipeline 9 instrs, B0 = {second}");

    let (results, visited) =
        MultiCollector::<9, 9, 3>::run_instrs_second::<9>(
            second,
            1_000,
            &|| {
                MultiCollector::new(
                    |prog, config| {
                        prog.term_or_rec(LIN_MIN, config.to_mut())
                            .is_settled()
                    },
                    [
                        |prog| prog.bkw_cant_halt(BKW_8).is_refuted(),
                        |prog| {
                            prog.bkw_cant_spinout(BKW_8).is_refuted()
                        },
                        |prog| prog.bkw_cant_blank(BKW_8).is_refuted(),
                    ],
                    |prog, config| {
                        prog.term_or_rec(LIN_MOR, config.to_mut())
                            .is_settled()
                            || prog.prover_settled(INF_MIN)
                    },
                    [
                        |prog| prog.cps_cant_halt(CPS_8),
                        |prog| prog.cps_cant_spinout(CPS_8),
                        |prog| prog.cps_cant_blank(CPS_8),
                    ],
                    |prog, config| {
                        prog.term_or_rec(LIN_MAX, config.to_mut())
                            .is_settled()
                            || prog.prover_settled(INF_MOR)
                    },
                    [
                        |prog| prog.far_cant_halt(FAR_8),
                        |prog| prog.far_cant_spinout(FAR_8),
                        |prog| prog.far_cant_blank(FAR_8),
                    ],
                )
            },
        );

    println!("visited: {}", show_num(visited));
    for (goal, result) in
        ["halt", "spinout", "blank"].into_iter().zip(results)
    {
        // Sort text representations so output is deterministic even if the
        // collector's underlying set has no stable iteration order.
        let mut holdouts: Vec<_> = result
            .into_iter()
            .map(|prog| format!("{prog:?}"))
            .collect();
        holdouts.sort_unstable();

        println!(
            "\n{goal} holdouts: {}",
            show_num(holdouts.len() as u64)
        );
        for prog in holdouts {
            println!("  {prog}");
        }
    }
}

fn test_enum_8() {
    println!("enum 8 instrs");

    assert_visited![
        8 => [
            500,
            4_169_973_285,
            [
                "0LB" => 51_333_122,
                "0LA" => 58_367_611,
                "1LB" => 80_083_759,
                "1LA" => 110_043_563,
                "0RC" => 238_450_775,
                "0LC" => 250_960_637,
                "2LB" => 266_559_788,
                "1RC" => 273_943_684,
                "1LC" => 372_920_355,
                "2LA" => 406_184_953,
                "2RC" => 874_803_659,
                "2LC" => 1_186_321_379,
            ],
        ]
    ];
}

fn test_enum_9() {
    println!("enum 9 instrs");

    assert_visited![
        9 => [
            1000,
            252_848_015_936,
            [
                "0LB" => 2_689_455_347,
                "0LA" => 3_035_647_371,
                "1LB" => 4_092_124_657,
                "1LA" => 5_618_555_208,
                "0RC" => 14_084_824_199,
                "0LC" => 14_493_075_105,
                "2LB" => 15_010_621_688,
                "1RC" => 15_969_404_154,
                "1LC" => 21_096_591_633,
                "2LA" => 22_840_767_836,
                "2RC" => 58_761_765_607,
                "2LC" => 75_155_183_131,
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
                73_086_586,
                [
                    "0LB" => 1_414_133,
                    "0LA" => 1_827_414,
                    "1LB" => 3_291_125,
                    "1LA" => 6_237_645,
                    "0LC" => 7_931_651,
                    "0RC" => 12_298_827,
                    "1RC" => 18_835_394,
                    "1LC" => 21_250_397,
                ],
            ],
            1 => [
                TREE_LIM,
                110_593_543,
                [
                    "0LA" => 2_493_091,
                    "0LB" => 3_986_267,
                    "1LB" => 9_972_610,
                    "1LA" => 10_283_111,
                    "0LC" => 10_989_362,
                    "0RC" => 20_394_447,
                    "1RC" => 24_123_151,
                    "1LC" => 28_351_504,
                ],
            ],
            2 => [
                TREE_LIM,
                439_727_291,
                [
                    "0LB" => 7_563_069,
                    "0LA" => 10_869_155,
                    "1LB" => 18_441_001,
                    "1LA" => 39_604_318,
                    "0LC" => 44_931_697,
                    "0RC" => 72_685_114,
                    "1RC" => 116_451_739,
                    "1LC" => 129_181_198,
                ],
            ],
        ],
        (3, 3) => [
            1 => [
                3_000,
                32_740_635,
                [
                    "0LA" => 475_637,
                    "1RC" => 1_206_450,
                    "0LC" => 1_342_539,
                    "0RC" => 2_061_904,
                    "0LB" => 2_120_928,
                    "1LC" => 2_137_077,
                    "2RC" => 2_228_871,
                    "1LA" => 2_585_262,
                    "2LC" => 2_668_681,
                    "2LA" => 3_862_293,
                    "1LB" => 5_937_736,
                    "2LB" => 6_113_257,
                ],
            ],
            2 => [
                3_000,
                112_806_224,
                [
                    "0LB" => 2_539_560,
                    "0LA" => 3_019_076,
                    "0RC" => 3_678_917,
                    "0LC" => 5_293_732,
                    "1LB" => 7_206_521,
                    "2LB" => 7_319_247,
                    "1RC" => 8_424_699,
                    "1LA" => 12_988_774,
                    "2RC" => 13_135_169,
                    "1LC" => 14_344_189,
                    "2LC" => 16_278_849,
                    "2LA" => 18_577_491,
                ],
            ],
        ],
        (2, 5) => [
            0 => [
                TREE_LIM,
                60_590_302,
                [
                    "0LA" => 1_418_940,
                    "0LB" => 1_554_862,
                    "1LB" => 3_448_062,
                    "1LA" => 3_781_940,
                    "2LB" => 14_627_527,
                    "2LA" => 35_758_971,
                ],
            ],
            1 => [
                TREE_LIM,
                131_784_209,
                [
                    "0LB" => 9_871_732,
                    "1LB" => 24_204_430,
                    "2LB" => 97_708_047,
                ],
            ],
            2 => [
                TREE_LIM,
                312_872_424,
                [
                    "0LA" => 5_998_100,
                    "0LB" => 6_111_921,
                    "1LB" => 16_836_519,
                    "1LA" => 17_869_473,
                    "2LB" => 69_117_813,
                    "2LA" => 196_938_598,
                ],
            ],
        ],
        (6, 2) => [
            0 => [
                TREE_LIM,
                20_475_610_229,
                [
                    "0LB" => 303_910_704,
                    "0LA" => 397_603_850,
                    "1LB" => 712_968_145,
                    "1LA" => 1_313_749_103,
                    "0LC" => 2_263_716_271,
                    "0RC" => 3_879_725_308,
                    "1RC" => 5_593_269_716,
                    "1LC" => 6_010_667_132,
                ],
            ],
        ],
        (2, 6) => [
            0 => [
                TREE_LIM,
                20_538_873_222,
                [
                    "0LB" => 389_651_768,
                    "0LA" => 402_848_503,
                    "1LB" => 844_580_993,
                    "1LA" => 978_650_038,
                    "2LB" => 4_950_349_002,
                    "2LA" => 12_972_792_918,
                ],
            ],
        ],
    ];
}

/**************************************/

fn main() {
    test_pipeline_9("0LB");
}
