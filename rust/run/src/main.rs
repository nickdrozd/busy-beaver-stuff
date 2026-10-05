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
                &|| HoldoutVisited::new(|prog, config| {
                    prog.term_or_rec(LIN_MIN, config.to_mut()).is_settled()
                        || $pipeline(prog, config)
                }),
            );

            assert_eq!(
                result,
                ($leaves, $visited),
                "(({}, {}), {}, {result:?})",
                $states, $colors, $goal,
            );
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

            assert_eq!(visited, $visited, "({}, {visited:?})", $instrs);

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
        assert_eq!(
            $result.len(),
            $leaves,
            "{}:{}{}",
            $instrs,
            $goal,
            if $result.len() < 50 {
                format!(", {result:?}", result = $result)
            } else {
                String::new()
            },
        );
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
            1 => (_4_2_1, 99, (0, 508_606)),
            2 => (_4_2_2, 99, (0, 1_932_610)),
        ],
        (2, 4) => [
            1 => (_2_4_1, TREE_LIM, (_2_4_1_, 442_485)),
            2 => (_2_4_2, TREE_LIM, (0, 1_022_590)),
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

    assert_holdouts![
        (4, 2) => [
            3 => (_4_2_3, 99, (50, 2_134_923)),
        ],
        (2, 4) => [
            3 => (_2_4_3, TREE_LIM, (88, 1_698_850)),
        ],
    ];

    assert_bkw![
        4 => [
            4,
            4_909,
            [
                0 => 0,
                1 => 0,
                2 => 0,
                3 => 0,
            ],
        ],
        5 => [
            12,
            151_351,
            [
                0 => 5,
                1 => 0,
                2 => 0,
                3 => 0,
            ],
        ],
        6 => [
            22,
            5_568_167,
            [
                0 => 196,
                1 => 16,
                2 => 3,
                3 => 0,
            ],
        ],
        7 => [
            109,
            246_492_765,
            [
                0 => 9066,
                1 => 716,
                2 => 251,
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
            12_835_863_274,
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
            12_835_863_274,
            [
                "0LB" => 149_873_236,
                "0LA" => 152_344_902,
                "1LB" => 239_821_500,
                "1LA" => 292_240_522,
                "0RC" => 672_783_943,
                "0LC" => 704_776_101,
                "1RC" => 799_490_953,
                "2LB" => 864_219_877,
                "1LC" => 1_090_488_713,
                "2LA" => 1_180_096_348,
                "2RC" => 2_845_087_951,
                "2LC" => 3_844_639_228,
            ],
        ]
    ];
}

fn test_enum_9() {
    println!("enum 9 instrs");

    assert_visited![
        9 => [
            1000,
            777_451_944_058,
            [
                "0LB" => 7_866_447_610,
                "0LA" => 8_063_846_081,
                "1LB" => 12_423_565_371,
                "1LA" => 15_332_496_471,
                "0RC" => 39_098_051_860,
                "0LC" => 41_041_631_526,
                "1RC" => 46_319_757_671,
                "2LB" => 49_275_058_835,
                "1LC" => 62_674_393_651,
                "2LA" => 67_891_560_224,
                "2RC" => 182_986_656_496,
                "2LC" => 244_478_478_262,
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
                90_676_712,
                [
                    "0LB" => 1_875_871,
                    "0LA" => 2_278_046,
                    "1LB" => 4_335_648,
                    "1LA" => 7_799_522,
                    "0LC" => 9_937_507,
                    "0RC" => 15_120_269,
                    "1RC" => 22_976_562,
                    "1LC" => 26_353_287,
                ],
            ],
            1 => [
                TREE_LIM,
                128_538_992,
                [
                    "0LA" => 2_894_838,
                    "0LB" => 4_950_736,
                    "1LA" => 11_836_762,
                    "1LB" => 12_217_160,
                    "0LC" => 12_776_784,
                    "0RC" => 23_405_244,
                    "1RC" => 27_720_463,
                    "1LC" => 32_737_005,
                ],
            ],
            2 => [
                TREE_LIM,
                486_399_920,
                [
                    "0LB" => 8_818_482,
                    "0LA" => 11_858_029,
                    "1LB" => 21_513_613,
                    "1LA" => 43_586_149,
                    "0LC" => 50_093_900,
                    "0RC" => 79_933_668,
                    "1RC" => 127_398_399,
                    "1LC" => 143_197_680,
                ],
            ],
        ],
        (3, 3) => [
            1 => [
                3_000,
                36_360_641,
                [
                    "0LA" => 533_598,
                    "1RC" => 1_323_906,
                    "0LC" => 1_450_831,
                    "0RC" => 2_234_664,
                    "1LC" => 2_336_602,
                    "0LB" => 2_455_876,
                    "2RC" => 2_501_158,
                    "1LA" => 2_794_411,
                    "2LC" => 2_910_781,
                    "2LA" => 4_183_961,
                    "1LB" => 6_700_205,
                    "2LB" => 6_934_648,
                ],
            ],
            2 => [
                3_000,
                118_329_782,
                [
                    "0LB" => 2_820_418,
                    "0LA" => 3_128_601,
                    "0RC" => 3_895_383,
                    "0LC" => 5_573_748,
                    "1LB" => 7_877_729,
                    "2LB" => 8_053_502,
                    "1RC" => 8_681_607,
                    "1LA" => 13_430_241,
                    "2RC" => 13_622_985,
                    "1LC" => 14_962_828,
                    "2LC" => 17_059_423,
                    "2LA" => 19_223_317,
                ],
            ],
        ],
        (2, 5) => [
            0 => [
                TREE_LIM,
                69_763_571,
                [
                    "0LA" => 1_607_731,
                    "0LB" => 1_879_647,
                    "1LB" => 4_114_959,
                    "1LA" => 4_312_300,
                    "2LB" => 17_625_217,
                    "2LA" => 40_223_717,
                ],
            ],
            1 => [
                TREE_LIM,
                141_649_268,
                [
                    "0LB" => 10_713_155,
                    "1LB" => 25_643_461,
                    "2LB" => 105_292_652,
                ],
            ],
            2 => [
                TREE_LIM,
                320_747_800,
                [
                    "0LA" => 6_104_044,
                    "0LB" => 6_538_103,
                    "1LB" => 17_593_810,
                    "1LA" => 18_201_486,
                    "2LB" => 73_335_280,
                    "2LA" => 198_975_077,
                ],
            ],
        ],
        (6, 2) => [
            0 => [
                TREE_LIM,
                24_415_867_910,
                [
                    "0LB" => 379_620_882,
                    "0LA" => 475_137_678,
                    "1LB" => 888_336_991,
                    "1LA" => 1_578_696_463,
                    "0LC" => 2_717_127_448,
                    "0RC" => 4_591_613_661,
                    "1RC" => 6_606_298_097,
                    "1LC" => 7_179_036_690,
                ],
            ],
        ],
        (2, 6) => [
            0 => [
                TREE_LIM,
                22_923_400_494,
                [
                    "0LA" => 445_738_126,
                    "0LB" => 450_386_123,
                    "1LB" => 963_457_305,
                    "1LA" => 1_088_214_558,
                    "2LB" => 5_699_902_127,
                    "2LA" => 14_275_702_255,
                ],
            ],
        ],
    ];
}

/**************************************/

const FAST: &[fn()] = &[test_bkw, test_deciders];

const SLOW: &[fn()] = &[test_enum_p, test_enum_9];

fn main() {
    test_enum_8();

    if !std::env::args().any(|x| x == "--all") {
        return;
    }

    FAST.par_iter().for_each(|f| f());

    test_holdouts();

    test_pipeline_8();

    if !std::env::args().any(|x| x == "--extra") {
        return;
    }

    SLOW.iter().for_each(|f| f());
}
