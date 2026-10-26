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
            1 => (_4_2_1, 99, (0, 414_564)),
            2 => (_4_2_2, 99, (0, 1_701_253)),
            3 => (_4_2_3, 99, (50, 1_870_959)),
        ],
        (2, 4) => [
            1 => (_2_4_1, TREE_LIM, (_2_4_1_, 382_825)),
            2 => (_2_4_2, TREE_LIM, (0, 941_768)),
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
            1_436,
            [
                0 => 0,
                1 => 0,
                2 => 0,
                3 => 0,
            ],
        ],
        5 => [
            12,
            50_568,
            [
                0 => 5,
                1 => 0,
                2 => 0,
                3 => 0,
            ],
        ],
        6 => [
            22,
            1_801_717,
            [
                0 => 196,
                1 => 16,
                2 => 3,
                3 => 0,
            ],
        ],
        7 => [
            109,
            81_827_176,
            [
                0 => 9066,
                1 => 710,
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
            4_169_528_078,
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
            4_169_528_078,
            [
                "0LB" => 51_322_536,
                "0LA" => 58_347_066,
                "1LB" => 80_075_126,
                "1LA" => 110_019_855,
                "0RC" => 238_414_040,
                "0LC" => 250_926_047,
                "2LB" => 266_524_207,
                "1RC" => 273_904_117,
                "1LC" => 372_893_711,
                "2LA" => 406_111_912,
                "2RC" => 874_742_407,
                "2LC" => 1_186_247_054,
            ],
        ]
    ];
}

fn test_enum_9() {
    println!("enum 9 instrs");

    assert_visited![
        9 => [
            1000,
            252_836_060_637,
            [
                "0LB" => 2_689_178_782,
                "0LA" => 3_035_237_431,
                "1LB" => 4_091_809_961,
                "1LA" => 5_618_085_389,
                "0RC" => 14_083_992_863,
                "0LC" => 14_492_253_342,
                "2LB" => 15_009_584_968,
                "1RC" => 15_968_400_775,
                "1LC" => 21_095_785_724,
                "2LA" => 22_839_210_086,
                "2RC" => 58_759_856_808,
                "2LC" => 75_152_664_508,
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
                73_062_425,
                [
                    "0LB" => 1_413_134,
                    "0LA" => 1_825_675,
                    "1LB" => 3_291_125,
                    "1LA" => 6_235_661,
                    "0LC" => 7_928_141,
                    "0RC" => 12_294_930,
                    "1RC" => 18_825_508,
                    "1LC" => 21_248_251,
                ],
            ],
            1 => [
                TREE_LIM,
                110_585_512,
                [
                    "0LA" => 2_492_471,
                    "0LB" => 3_984_936,
                    "1LB" => 9_972_610,
                    "1LA" => 10_282_716,
                    "0LC" => 10_988_233,
                    "0RC" => 20_393_678,
                    "1RC" => 24_119_552,
                    "1LC" => 28_351_316,
                ],
            ],
            2 => [
                TREE_LIM,
                439_666_596,
                [
                    "0LB" => 7_561_738,
                    "0LA" => 10_865_405,
                    "1LB" => 18_441_001,
                    "1LA" => 39_592_875,
                    "0LC" => 44_925_359,
                    "0RC" => 72_676_688,
                    "1RC" => 116_430_967,
                    "1LC" => 129_172_563,
                ],
            ],
        ],
        (3, 3) => [
            1 => [
                3_000,
                31_799_660,
                [
                    "0LA" => 446_686,
                    "1RC" => 1_145_383,
                    "0LC" => 1_308_623,
                    "0RC" => 2_002_561,
                    "0LB" => 2_035_226,
                    "1LC" => 2_095_733,
                    "2RC" => 2_177_174,
                    "1LA" => 2_540_462,
                    "2LC" => 2_583_128,
                    "2LA" => 3_755_632,
                    "1LB" => 5_801_796,
                    "2LB" => 5_907_256,
                ],
            ],
            2 => [
                3_000,
                109_879_541,
                [
                    "0LB" => 2_457_964,
                    "0LA" => 2_876_843,
                    "0RC" => 3_539_342,
                    "0LC" => 5_105_626,
                    "1LB" => 7_072_536,
                    "2LB" => 7_123_849,
                    "1RC" => 8_116_865,
                    "1LA" => 12_751_243,
                    "2RC" => 12_905_027,
                    "1LC" => 14_066_245,
                    "2LC" => 15_807_162,
                    "2LA" => 18_056_839,
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
                131_250_448,
                [
                    "0LB" => 9_837_741,
                    "1LB" => 24_116_391,
                    "2LB" => 97_296_316,
                ],
            ],
            2 => [
                TREE_LIM,
                309_992_416,
                [
                    "0LA" => 5_900_435,
                    "0LB" => 6_080_221,
                    "1LB" => 16_756_454,
                    "1LA" => 17_469_294,
                    "2LB" => 68_770_847,
                    "2LA" => 195_015_165,
                ],
            ],
        ],
        (6, 2) => [
            0 => [
                TREE_LIM,
                20_474_755_176,
                [
                    "0LB" => 303_889_089,
                    "0LA" => 397_559_774,
                    "1LB" => 712_968_145,
                    "1LA" => 1_313_675_341,
                    "0LC" => 2_263_595_990,
                    "0RC" => 3_879_564_127,
                    "1RC" => 5_592_941_931,
                    "1LC" => 6_010_560_779,
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
