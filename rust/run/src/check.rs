use crate::holdouts::*;
use std::collections::BTreeSet;
use tm::{Prog, machine::RunProver as _};

/**************************************/

pub fn test_holdouts() {
    println!("subsets");
    test_subsets();

    println!("champs");
    test_champs();

    println!("lin rec");
    test_linrec();

    println!("prover");
    test_prover();

    println!("backward");
    test_backward();

    println!("cps");
    test_cps();

    println!("far");
    test_far();
}

/**************************************/

const LIN_CHECK: usize = 5_000_000;

fn check_linrec(
    failures: &mut Vec<String>,
    context: &str,
    progs: &[&str],
) {
    let mut settled = vec![];

    for &prog in progs {
        if Prog::<8, 8>::from(prog)
            .term_or_rec_fresh(LIN_CHECK)
            .is_settled()
        {
            settled.push(prog.to_owned());
        }
    }

    failures.extend(holdouts_match_errors(context, &[], &[], settled));
}

fn test_linrec() {
    let mut failures = vec![];

    check_linrec(&mut failures, "8-0 champs", _8_0_ch);
    check_linrec(&mut failures, "8-0 holdouts", _8_0_ho.as_slice());

    check_linrec(&mut failures, "8-1 champs", _8_1_ch);
    check_linrec(&mut failures, "8-1 holdouts", _8_1_ho.as_slice());

    check_linrec(&mut failures, "8-2 champs", _8_2_ch);
    check_linrec(&mut failures, "8-2 holdouts", _8_2_ho.as_slice());

    assert_no_holdout_failures("lin rec", &failures);
}

fn check_holdout_decider(
    failures: &mut Vec<String>,
    context: &str,
    progs: &[&str],
    decider: impl Fn(&Prog<8, 8>) -> bool,
) {
    println!("{context}");

    let mut refuted = vec![];

    for &prog_str in progs {
        let prog = Prog::<8, 8>::from(prog_str);

        if decider(&prog) {
            refuted.push(prog_str.to_owned());
        }
    }

    failures.extend(holdouts_match_errors(context, &[], &[], refuted));
}

fn test_champs() {
    assert!(
        Prog::<3, 3>::from("1RB 0LB 2LA  1LA 0RC 0LB  2RC 2RB 0LC")
            .run_prover(1000)
            .is_mult()
    );

    assert!(
        Prog::<5, 2>::from(
            "1RB 1LC  1RD 0RA  0LC 1LE  1LA 0RE  0LA 1RB",
        )
        .make_block_macro(2)
        .run_prover(337_356)
        .is_spinout()
    );

    assert!(
        Prog::<5, 2>::from(
            "1RB 1LC  0LD 0LB  0RD 0LA  0LE 1LD  1RE 1RA",
        )
        .make_block_macro(2)
        .run_prover(600_000)
        .is_spinout()
    );
}

const PROVER: usize = 10_000;

fn test_prover() {
    let mut failures = vec![];

    check_holdout_decider(
        &mut failures,
        "8-0 halt",
        _8_0_ho.as_slice(),
        |prog| prog.check_inf(PROVER),
    );
    check_holdout_decider(
        &mut failures,
        "8-1 spinout",
        _8_1_ho.as_slice(),
        |prog| prog.check_inf(PROVER),
    );
    check_holdout_decider(
        &mut failures,
        "8-2 blank",
        _8_2_ho.as_slice(),
        |prog| prog.check_inf(PROVER),
    );

    assert_no_holdout_failures("prover", &failures);
}

fn test_backward() {
    let mut failures = vec![];

    check_holdout_decider(
        &mut failures,
        "8-0 halt",
        _8_0_ho.as_slice(),
        |prog| prog.bkw_cant_halt(2000).is_refuted(),
    );
    check_holdout_decider(
        &mut failures,
        "8-1 spinout",
        _8_1_ho.as_slice(),
        |prog| prog.bkw_cant_spinout(2000).is_refuted(),
    );
    check_holdout_decider(
        &mut failures,
        "8-2 blank",
        _8_2_ho.as_slice(),
        |prog| prog.bkw_cant_blank(2000).is_refuted(),
    );

    assert_no_holdout_failures("backward", &failures);
}

fn test_cps() {
    let mut failures = vec![];

    check_holdout_decider(
        &mut failures,
        "8-0 halt",
        _8_0_ho.as_slice(),
        |prog| prog.cps_cant_halt(20),
    );
    check_holdout_decider(
        &mut failures,
        "8-1 spinout",
        _8_1_ho.as_slice(),
        |prog| prog.cps_cant_spinout(20),
    );
    check_holdout_decider(
        &mut failures,
        "8-2 blank",
        _8_2_ho.as_slice(),
        |prog| prog.cps_cant_blank(20),
    );
    check_holdout_decider(
        &mut failures,
        "2-4 quasihalt",
        _2_4_q_ho.as_slice(),
        |prog| prog.cps_cant_quasihalt(18),
    );

    assert_no_holdout_failures("cps", &failures);
}

fn test_far() {
    let mut failures = vec![];

    check_holdout_decider(
        &mut failures,
        "8-0 halt",
        _8_0_ho.as_slice(),
        |prog| prog.far_cant_halt(3),
    );
    check_holdout_decider(
        &mut failures,
        "8-1 spinout",
        _8_1_ho.as_slice(),
        |prog| prog.far_cant_spinout(3),
    );
    check_holdout_decider(
        &mut failures,
        "8-2 blank",
        _8_2_ho.as_slice(),
        |prog| prog.far_cant_blank(3),
    );

    assert_no_holdout_failures("far", &failures);
}

/**************************************/

fn holdout_subset_errors(
    context: impl core::fmt::Display,
    subset: &[&str],
    superset: &[&str],
) -> Vec<String> {
    let subset = subset.iter().copied().collect::<BTreeSet<_>>();
    let superset = superset.iter().copied().collect::<BTreeSet<_>>();
    let missing = subset.difference(&superset).collect::<Vec<_>>();

    if missing.is_empty() {
        vec![]
    } else {
        vec![format!(
            "{context}\nnot in general 8-instruction holdouts: {missing:#?}",
        )]
    }
}

fn test_subsets() {
    let mut failures = vec![];

    failures.extend(holdout_subset_errors(
        "7 halt holdouts",
        _7_0_ho,
        _8_0_ho.as_slice(),
    ));
    failures.extend(holdout_subset_errors(
        "4-2-1 spinout holdouts",
        _4_2_1_ho,
        _8_1_ho.as_slice(),
    ));
    failures.extend(holdout_subset_errors(
        "2-4-1 spinout holdouts",
        _2_4_1_ho,
        _8_1_ho.as_slice(),
    ));
    failures.extend(holdout_subset_errors(
        "2-4-1 spinout champs",
        _2_4_1_ch,
        _8_1_ch,
    ));
    failures.extend(holdout_subset_errors(
        "4-2-2 blank holdouts",
        _4_2_2_ho,
        _8_2_ho.as_slice(),
    ));
    failures.extend(holdout_subset_errors(
        "2-4-2 blank holdouts",
        _2_4_2_ho,
        _8_2_ho.as_slice(),
    ));

    assert_no_holdout_failures("subsets", &failures);
}

/**************************************/

fn assert_no_holdout_failures(context: &str, failures: &[String]) {
    assert!(
        failures.is_empty(),
        "{context}: {} failure(s)\n\n{}",
        failures.len(),
        failures.join("\n\n"),
    );
}

pub fn holdouts_match_errors<T>(
    context: impl core::fmt::Display,
    champs: &[&str],
    holdouts: &[&str],
    result: impl IntoIterator<Item = T>,
) -> Vec<String>
where
    T: ToString,
{
    let context = context.to_string();
    let mut failures = vec![];

    let mut seen_champs = BTreeSet::new();
    let mut duplicate_champs = BTreeSet::new();

    for &champ in champs {
        if !seen_champs.insert(champ) {
            duplicate_champs.insert(champ);
        }
    }

    let mut seen_holdouts = BTreeSet::new();
    let mut duplicate_holdouts = BTreeSet::new();

    for &holdout in holdouts {
        if !seen_holdouts.insert(holdout) {
            duplicate_holdouts.insert(holdout);
        }
    }

    let intersection = seen_champs
        .intersection(&seen_holdouts)
        .copied()
        .collect::<Vec<_>>();

    if !duplicate_champs.is_empty()
        || !duplicate_holdouts.is_empty()
        || !intersection.is_empty()
    {
        let mut message = context.clone();
        use core::fmt::Write as _;

        if !duplicate_champs.is_empty() {
            write!(
                &mut message,
                "\nduplicate champs: {duplicate_champs:#?}"
            )
            .unwrap();
        }

        if !duplicate_holdouts.is_empty() {
            write!(
                &mut message,
                "\nduplicate holdouts: {duplicate_holdouts:#?}",
            )
            .unwrap();
        }

        if !intersection.is_empty() {
            write!(
                &mut message,
                "\nchamp/holdout intersection: {intersection:#?}",
            )
            .unwrap();
        }

        failures.push(message);
    }

    let expected_holdouts = holdouts
        .iter()
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>();
    let expected_champs = champs
        .iter()
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>();
    let expected = expected_holdouts
        .union(&expected_champs)
        .cloned()
        .collect::<BTreeSet<_>>();
    let result = result
        .into_iter()
        .map(|x| x.to_string())
        .collect::<BTreeSet<_>>();

    let missing_champs =
        expected_champs.difference(&result).collect::<Vec<_>>();

    if !missing_champs.is_empty() {
        let mut message = context.clone();
        use core::fmt::Write as _;

        write!(&mut message, "\nmissing champs: {missing_champs:#?}")
            .unwrap();

        failures.push(message);
    }

    if result != expected {
        let collected_not_expected =
            result.difference(&expected).collect::<Vec<_>>();
        let expected_not_collected =
            expected.difference(&result).collect::<Vec<_>>();

        let mut message = context;
        use core::fmt::Write as _;

        if !collected_not_expected.is_empty() {
            write!(
                &mut message,
                "\ncollected but not expected: {collected_not_expected:#?}",
            )
            .unwrap();
        }

        if !expected_not_collected.is_empty() {
            write!(
                &mut message,
                "\nexpected but not collected: {expected_not_collected:#?}",
            )
            .unwrap();
        }

        failures.push(message);
    }

    failures
}

pub fn assert_holdouts_match<T>(
    context: impl core::fmt::Display,
    champs: &[&str],
    holdouts: &[&str],
    result: impl IntoIterator<Item = T>,
) where
    T: ToString,
{
    assert_no_holdout_failures(
        "holdout match",
        &holdouts_match_errors(context, champs, holdouts, result),
    );
}
