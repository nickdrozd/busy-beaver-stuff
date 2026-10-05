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
            1 => (_4_2_1, 99, (0, 469_872)),
            2 => (_4_2_2, 99, (0, 1_841_212)),
            3 => (_4_2_3, 99, (50, 2_038_787)),
        ],
        (2, 4) => [
            1 => (_2_4_1, TREE_LIM, (_2_4_1_, 440_004)),
            2 => (_2_4_2, TREE_LIM, (0, 1_019_095)),
            3 => (_2_4_3, TREE_LIM, (88, 1_692_202)),
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
            4_670,
            [
                0 => 0,
                1 => 0,
                2 => 0,
                3 => 0,
            ],
        ],
        5 => [
            12,
            148_091,
            [
                0 => 5,
                1 => 0,
                2 => 0,
                3 => 0,
            ],
        ],
        6 => [
            22,
            5_476_853,
            [
                0 => 196,
                1 => 16,
                2 => 3,
                3 => 0,
            ],
        ],
        7 => [
            109,
            243_376_535,
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
            12_697_165_128,
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
            12_697_165_128,
            [
                "0LB" => 147_258_085,
                "0LA" => 151_125_613,
                "1LB" => 235_669_914,
                "1LA" => 290_359_141,
                "0RC" => 668_658_531,
                "0LC" => 694_007_993,
                "1RC" => 794_246_999,
                "2LB" => 850_003_181,
                "1LC" => 1_075_026_814,
                "2LA" => 1_173_276_248,
                "2RC" => 2_826_556_145,
                "2LC" => 3_790_976_464,
            ],
        ]
    ];
}

fn test_enum_9() {
    println!("enum 9 instrs");

    assert_visited![
        9 => [
            1000,
            770_358_617_963,
            [
                "0LB" => 7_754_428_193,
                "0LA" => 8_010_420_786,
                "1LB" => 12_241_691_840,
                "1LA" => 15_241_923_361,
                "0RC" => 38_892_762_033,
                "0LC" => 40_546_004_245,
                "1RC" => 46_047_627_805,
                "2LB" => 48_585_741_839,
                "1LC" => 61_938_139_226,
                "2LA" => 67_518_575_177,
                "2RC" => 181_918_696_242,
                "2LC" => 241_662_607_216,
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
                86_811_271,
                [
                    "0LB" => 1_768_674,
                    "0LA" => 2_175_756,
                    "1LB" => 4_071_088,
                    "1LA" => 7_405_531,
                    "0LC" => 9_467_379,
                    "0RC" => 14_704_408,
                    "1RC" => 22_165_354,
                    "1LC" => 25_053_081,
                ],
            ],
            1 => [
                TREE_LIM,
                120_847_197,
                [
                    "0LA" => 2_743_925,
                    "0LB" => 4_508_965,
                    "1LB" => 11_128_034,
                    "1LA" => 11_243_580,
                    "0LC" => 12_021_607,
                    "0RC" => 22_404_751,
                    "1RC" => 26_132_387,
                    "1LC" => 30_663_948,
                ],
            ],
            2 => [
                TREE_LIM,
                467_174_647,
                [
                    "0LB" => 8_320_742,
                    "0LA" => 11_534_989,
                    "1LB" => 20_229_748,
                    "1LA" => 42_205_799,
                    "0LC" => 47_887_446,
                    "0RC" => 77_639_697,
                    "1RC" => 122_817_766,
                    "1LC" => 136_538_460,
                ],
            ],
        ],
        (3, 3) => [
            1 => [
                3_000,
                36_128_707,
                [
                    "0LA" => 530_938,
                    "1RC" => 1_320_345,
                    "0LC" => 1_449_391,
                    "0RC" => 2_232_872,
                    "1LC" => 2_332_358,
                    "0LB" => 2_435_278,
                    "2RC" => 2_496_454,
                    "1LA" => 2_784_533,
                    "2LC" => 2_904_629,
                    "2LA" => 4_165_038,
                    "1LB" => 6_620_090,
                    "2LB" => 6_856_781,
                ],
            ],
            2 => [
                3_000,
                117_824_260,
                [
                    "0LB" => 2_806_007,
                    "0LA" => 3_125_857,
                    "0RC" => 3_890_932,
                    "0LC" => 5_540_171,
                    "1LB" => 7_819_622,
                    "2LB" => 7_996_651,
                    "1RC" => 8_671_280,
                    "1LA" => 13_419_169,
                    "2RC" => 13_608_526,
                    "1LC" => 14_836_818,
                    "2LC" => 16_903_574,
                    "2LA" => 19_205_653,
                ],
            ],
        ],
        (2, 5) => [
            0 => [
                TREE_LIM,
                68_357_368,
                [
                    "0LA" => 1_565_010,
                    "0LB" => 1_830_669,
                    "1LB" => 3_997_099,
                    "1LA" => 4_191_005,
                    "2LB" => 17_329_244,
                    "2LA" => 39_444_341,
                ],
            ],
            1 => [
                TREE_LIM,
                141_186_457,
                [
                    "0LB" => 10_664_177,
                    "1LB" => 25_525_601,
                    "2LB" => 104_996_679,
                ],
            ],
            2 => [
                TREE_LIM,
                319_949_192,
                [
                    "0LA" => 6_085_431,
                    "0LB" => 6_516_220,
                    "1LB" => 17_528_677,
                    "1LA" => 18_136_599,
                    "2LB" => 73_175_384,
                    "2LA" => 198_506_881,
                ],
            ],
        ],
        (6, 2) => [
            0 => [
                TREE_LIM,
                23_484_758_947,
                [
                    "0LB" => 360_564_527,
                    "0LA" => 456_391_389,
                    "1LB" => 841_143_275,
                    "1LA" => 1_507_756_438,
                    "0LC" => 2_605_601_903,
                    "0RC" => 4_463_438_472,
                    "1RC" => 6_381_475_381,
                    "1LC" => 6_868_387_562,
                ],
            ],
        ],
        (2, 6) => [
            0 => [
                TREE_LIM,
                22_476_757_779,
                [
                    "0LA" => 435_022_993,
                    "0LB" => 440_536_551,
                    "1LB" => 940_227_064,
                    "1LA" => 1_060_887_471,
                    "2LB" => 5_605_216_062,
                    "2LA" => 13_994_867_638,
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
