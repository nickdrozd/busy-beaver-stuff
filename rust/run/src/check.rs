use crate::holdouts::*;
use std::collections::BTreeSet;
use tm::Prog;

/**************************************/

pub fn test_holdouts() {
    println!("lin rec");
    test_linrec();

    println!("backward");
    test_backward();

    println!("ctl");
    test_ctl();

    println!("cps");
    test_cps();

    println!("far");
    test_far();
}

/**************************************/

const LIN_CHECK: usize = 11_000;

fn check_linrec<const STATES: usize, const COLORS: usize>(
    failures: &mut Vec<String>,
    context: &str,
    progs: &[&str],
) {
    let mut settled = vec![];

    for &prog in progs {
        if Prog::<STATES, COLORS>::from(prog)
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

    check_linrec::<4, 2>(&mut failures, "4-2-1 champs", _4_2_1_ch);
    check_linrec::<4, 2>(&mut failures, "4-2-1 holdouts", _4_2_1_ho);
    check_linrec::<4, 2>(&mut failures, "4-2-2 champs", _4_2_2_ch);
    check_linrec::<4, 2>(&mut failures, "4-2-2 holdouts", _4_2_2_ho);

    check_linrec::<2, 4>(&mut failures, "2-4-1 champs", _2_4_1_ch);
    check_linrec::<2, 4>(&mut failures, "2-4-1 holdouts", _2_4_1_ho);
    check_linrec::<2, 4>(&mut failures, "2-4-2 champs", _2_4_2_ch);
    check_linrec::<2, 4>(&mut failures, "2-4-2 holdouts", _2_4_2_ho);

    assert_no_holdout_failures("lin rec", &failures);
}

fn check_holdout_decider<const STATES: usize, const COLORS: usize>(
    failures: &mut Vec<String>,
    context: &str,
    progs: &[&str],
    decider: impl Fn(&Prog<STATES, COLORS>) -> bool,
) {
    println!("{context}");

    let mut refuted = vec![];

    for &prog_str in progs {
        let prog = Prog::<STATES, COLORS>::from(prog_str);

        if decider(&prog) {
            refuted.push(prog_str.to_owned());
        }
    }

    failures.extend(holdouts_match_errors(context, &[], &[], refuted));
}

fn test_backward() {
    let mut failures = vec![];

    check_holdout_decider::<4, 2>(
        &mut failures,
        "4-2-2",
        _4_2_2_ho,
        |prog| prog.bkw_cant_blank(2000).is_refuted(),
    );
    check_holdout_decider::<2, 4>(
        &mut failures,
        "2-4-2",
        _2_4_2_ho,
        |prog| prog.bkw_cant_blank(2000).is_refuted(),
    );
    check_holdout_decider::<4, 2>(
        &mut failures,
        "4-2-1",
        _4_2_1_ho,
        |prog| prog.bkw_cant_spinout(2000).is_refuted(),
    );
    check_holdout_decider::<2, 4>(
        &mut failures,
        "2-4-1",
        _2_4_1_ho,
        |prog| prog.bkw_cant_spinout(2000).is_refuted(),
    );

    assert_no_holdout_failures("backward", &failures);
}

fn test_ctl() {
    let mut failures = vec![];

    check_holdout_decider::<4, 2>(
        &mut failures,
        "4-2-2",
        _4_2_2_ho,
        |prog| prog.ctl_cant_blank(130),
    );
    check_holdout_decider::<2, 4>(
        &mut failures,
        "2-4-2",
        _2_4_2_ho,
        |prog| prog.ctl_cant_blank(200),
    );
    check_holdout_decider::<4, 2>(
        &mut failures,
        "4-2-1",
        _4_2_1_ho,
        |prog| prog.ctl_cant_spinout(500),
    );
    check_holdout_decider::<2, 4>(
        &mut failures,
        "2-4-1",
        _2_4_1_ho,
        |prog| prog.ctl_cant_spinout(700),
    );

    assert_no_holdout_failures("ctl", &failures);
}

fn test_cps() {
    let mut failures = vec![];

    check_holdout_decider::<4, 2>(
        &mut failures,
        "4-2-2",
        _4_2_2_ho,
        |prog| prog.cps_cant_blank(20),
    );
    check_holdout_decider::<2, 4>(
        &mut failures,
        "2-4-2",
        _2_4_2_ho,
        |prog| prog.cps_cant_blank(20),
    );
    check_holdout_decider::<4, 2>(
        &mut failures,
        "4-2-1",
        _4_2_1_ho,
        |prog| prog.cps_cant_spinout(12),
    );
    check_holdout_decider::<2, 4>(
        &mut failures,
        "2-4-1",
        _2_4_1_ho,
        |prog| prog.cps_cant_spinout(11),
    );

    assert_no_holdout_failures("cps", &failures);
}

fn test_far() {
    let mut failures = vec![];

    check_holdout_decider::<4, 2>(
        &mut failures,
        "4-2-2",
        _4_2_2_ho,
        |prog| prog.far_cant_blank(3),
    );
    check_holdout_decider::<2, 4>(
        &mut failures,
        "2-4-2",
        _2_4_2_ho,
        |prog| prog.far_cant_blank(3),
    );
    check_holdout_decider::<4, 2>(
        &mut failures,
        "4-2-1",
        _4_2_1_ho,
        |prog| prog.far_cant_spinout(3),
    );
    check_holdout_decider::<2, 4>(
        &mut failures,
        "2-4-1",
        _2_4_1_ho,
        |prog| prog.far_cant_spinout(3),
    );

    assert_no_holdout_failures("far", &failures);
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
