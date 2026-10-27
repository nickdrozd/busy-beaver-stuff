#![allow(
    dead_code,
    unused_imports,
    unused_macro_rules,
    clippy::wildcard_imports
)]
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
            1 => (_4_2_1, 99, (0, 258_229)),
            2 => (_4_2_2, 99, (0, 949_480)),
            3 => (_4_2_3, 99, (50, 1_828_723)),
        ],
        (2, 4) => [
            1 => (_2_4_1, TREE_LIM, (_2_4_1_, 358_596)),
            2 => (_2_4_2, TREE_LIM, (0, 622_367)),
            3 => (_2_4_3, TREE_LIM, (88, 1_563_752)),
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
            1_433,
            [
                0 => 0,
                1 => 0,
                2 => 0,
                3 => 0,
            ],
        ],
        5 => [
            12,
            50_520,
            [
                0 => 5,
                1 => 0,
                2 => 0,
                3 => 0,
            ],
        ],
        6 => [
            22,
            1_800_711,
            [
                0 => 196,
                1 => 16,
                2 => 3,
                3 => 0,
            ],
        ],
        7 => [
            109,
            81_810_021,
            [
                0 => 9066,
                1 => 709,
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
            4_169_210_444,
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
            4_169_210_444,
            [
                "0LB" => 51_318_890,
                "0LA" => 58_330_838,
                "1LB" => 80_068_614,
                "1LA" => 110_003_966,
                "0RC" => 238_384_367,
                "0LC" => 250_885_144,
                "2LB" => 266_519_598,
                "1RC" => 273_870_294,
                "1LC" => 372_854_693,
                "2LA" => 406_088_755,
                "2RC" => 874_688_702,
                "2LC" => 1_186_196_583,
            ],
        ]
    ];
}

fn test_enum_9() {
    println!("enum 9 instrs");

    assert_visited![
        9 => [
            1000,
            252_830_536_471,
            [
                "0LB" => 2_689_123_397,
                "0LA" => 3_034_979_171,
                "1LB" => 4_091_719_031,
                "1LA" => 5_617_876_346,
                "0RC" => 14_083_518_366,
                "0LC" => 14_491_563_127,
                "2LB" => 15_009_475_205,
                "1RC" => 15_967_873_960,
                "1LC" => 21_095_128_342,
                "2LA" => 22_838_768_925,
                "2RC" => 58_758_913_720,
                "2LC" => 75_151_596_881,
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
                72_682_622,
                [
                    "0LB" => 1_403_879,
                    "0LA" => 1_816_063,
                    "1LB" => 3_266_544,
                    "1LA" => 6_210_836,
                    "0LC" => 7_881_650,
                    "0RC" => 12_241_936,
                    "1RC" => 18_740_752,
                    "1LC" => 21_120_962,
                ],
            ],
            1 => [
                TREE_LIM,
                76_849_961,
                [
                    "0LA" => 1_518_825,
                    "0LB" => 2_567_344,
                    "1LB" => 6_920_114,
                    "1LA" => 7_471_927,
                    "0LC" => 7_660_683,
                    "0RC" => 14_699_854,
                    "1RC" => 16_573_710,
                    "1LC" => 19_437_504,
                ],
            ],
            2 => [
                TREE_LIM,
                278_918_904,
                [
                    "0LB" => 3_862_795,
                    "0LA" => 5_725_451,
                    "1LB" => 10_830_189,
                    "0LC" => 24_224_679,
                    "1LA" => 25_095_099,
                    "0RC" => 47_677_337,
                    "1RC" => 77_326_103,
                    "1LC" => 84_177_251,
                ],
            ],
        ],
        (3, 3) => [
            1 => [
                3_000,
                25_336_522,
                [
                    "0LA" => 364_721,
                    "0LC" => 825_234,
                    "1RC" => 902_112,
                    "0RC" => 1_238_840,
                    "0LB" => 1_650_853,
                    "1LC" => 1_701_959,
                    "2RC" => 1_825_934,
                    "1LA" => 2_043_823,
                    "2LC" => 2_178_761,
                    "2LA" => 3_044_808,
                    "1LB" => 4_731_997,
                    "2LB" => 4_827_480,
                ],
            ],
            2 => [
                3_000,
                65_138_633,
                [
                    "0LB" => 1_538_862,
                    "0LA" => 1_744_182,
                    "0RC" => 2_217_440,
                    "0LC" => 3_056_016,
                    "1LB" => 3_914_697,
                    "2LB" => 3_952_996,
                    "1RC" => 4_670_574,
                    "1LA" => 7_335_165,
                    "2RC" => 8_040_708,
                    "1LC" => 8_398_517,
                    "2LC" => 9_653_074,
                    "2LA" => 10_616_402,
                ],
            ],
        ],
        (2, 5) => [
            0 => [
                TREE_LIM,
                60_133_714,
                [
                    "0LA" => 1_399_305,
                    "0LB" => 1_549_988,
                    "1LB" => 3_435_112,
                    "1LA" => 3_737_843,
                    "2LB" => 14_555_195,
                    "2LA" => 35_456_271,
                ],
            ],
            1 => [
                TREE_LIM,
                128_014_114,
                [
                    "0LB" => 9_588_668,
                    "1LB" => 23_668_285,
                    "2LB" => 94_757_161,
                ],
            ],
            2 => [
                TREE_LIM,
                209_082_323,
                [
                    "0LA" => 4_519_600,
                    "0LB" => 4_767_975,
                    "1LB" => 10_621_624,
                    "1LA" => 11_364_740,
                    "2LB" => 44_768_935,
                    "2LA" => 133_039_449,
                ],
            ],
        ],
        (6, 2) => [
            0 => [
                TREE_LIM,
                20_368_321_792,
                [
                    "0LB" => 301_977_086,
                    "0LA" => 395_597_726,
                    "1LB" => 708_163_348,
                    "1LA" => 1_308_680_518,
                    "0LC" => 2_250_632_116,
                    "0RC" => 3_861_944_967,
                    "1RC" => 5_565_707_594,
                    "1LC" => 5_975_618_437,
                ],
            ],
        ],
        (2, 6) => [
            0 => [
                TREE_LIM,
                20_492_395_089,
                [
                    "0LB" => 389_285_810,
                    "0LA" => 401_585_628,
                    "1LB" => 843_714_024,
                    "1LA" => 974_355_615,
                    "2LB" => 4_944_290_194,
                    "2LA" => 12_939_163_818,
                ],
            ],
        ],
    ];
}

/**************************************/

const FAST: &[fn()] = &[test_bkw, test_deciders, test_enum_8];

const SLOW: &[fn()] = &[test_enum_p, test_enum_9, test_pipeline_8];

fn main() {
    FAST.par_iter().for_each(|f| f());

    if !std::env::args().any(|x| x == "--all") {
        return;
    }

    test_holdouts();

    SLOW.par_iter().for_each(|f| f());
}
