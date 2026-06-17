use crate::holdouts::*;
use std::collections::BTreeSet;
use tm::Prog;

/**************************************/

const LIN_CHECK: usize = 11_000;

fn test_linrec() {
    let mut settled = vec![];

    for progs in [_4_2_1_ch, _4_2_1_ho, _4_2_2_ch, _4_2_2_ho] {
        for &prog in progs {
            if Prog::<4, 2>::from(prog)
                .term_or_rec_fresh(LIN_CHECK)
                .is_settled()
            {
                settled.push(prog.to_owned());
            }
        }
    }

    for progs in [_2_4_1_ch, _2_4_1_ho, _2_4_2_ch, _2_4_2_ho] {
        for &prog in progs {
            if Prog::<2, 4>::from(prog)
                .term_or_rec_fresh(LIN_CHECK)
                .is_settled()
            {
                settled.push(prog.to_owned());
            }
        }
    }

    assert_holdouts_match("lin rec", &[], &[], settled);
}

#[expect(clippy::shadow_unrelated)]
fn test_far() {
    println!("4-2-2");
    let mut refuted = vec![];
    for &prog in _4_2_2_ho {
        if Prog::<4, 2>::from(prog).far_cant_blank(3) {
            refuted.push(prog.to_owned());
        }
    }
    assert_holdouts_match("4-2-2 far", &[], &[], refuted);

    println!("2-4-2");
    let mut refuted = vec![];
    for &prog in _2_4_2_ho {
        if Prog::<2, 4>::from(prog).far_cant_blank(3) {
            refuted.push(prog.to_owned());
        }
    }
    assert_holdouts_match("2-4-2 far", &[], &[], refuted);

    println!("4-2-1");
    let mut refuted = vec![];
    for &prog in _4_2_1_ho {
        if Prog::<4, 2>::from(prog).far_cant_spinout(3) {
            refuted.push(prog.to_owned());
        }
    }
    assert_holdouts_match("4-2-1 far", &[], &[], refuted);

    println!("2-4-1");
    let mut refuted = vec![];
    for &prog in _2_4_1_ho {
        if Prog::<2, 4>::from(prog).far_cant_spinout(3) {
            refuted.push(prog.to_owned());
        }
    }
    assert_holdouts_match("2-4-1 far", &[], &[], refuted);
}

/**************************************/

const _2_2_2: &[&str] = &[
    "1RB ...  0LB 0LA",
    "1RB ...  0LB 0LB",
    "1RB ...  0LB 0RA",
    "1RB ...  0LB 0RB",
];

const _3_2_2: &[&str] = &[
    "1RB ...  0RC 1LB  1LA 0RB",
    "1RB 0LA  0LC 1RA  1LA 1LB",
    "1RB 0LA  1RC ...  0LC 0LA",
    "1RB 0RB  0LB 1LC  0RA ...",
    "1RB 0RB  0LB 1RC  0LA ...",
    "1RB 0RB  1LC 1RC  0LA 1LA",
    "1RB 1LB  0RC 1LA  1LA 0RA",
    "1RB 1RC  0LA 0RA  0LB ...",
];

const _4_2_2: &[&str] = &[
    "1RB ...  0LC 0LC  0RD 1LB  1RD 0LB",
    "1RB ...  0LC 0RB  1LD 1LA  0RB 1LC",
    "1RB 0RB  0RC 0LD  1LC 1RD  0RA 1LD",
    "1RB 1LA  0LA 1RC  0RB 0LD  0RC 1LD",
    "1RB 1LB  1LA 1RC  0LD 0RB  ... 0LC",
    "1RB 1LC  0LB 1LA  1RC 0LD  0RC 1LD",
];

const _3_3_2: &[&str] = &["1RB 2RC 2RA  0LC 1RA 2RB  2LC 1LA 0RB"];

const _5_2_2: &[&str] =
    &["1RB 1LA  1RC 0LD  0LA 1RE  0RE 1LD  0RC 0RB"];

const _6_2_2: &[&str] =
    &["1RB 1LC  1RD 1RB  0RE 1RE  1LD 1LA  0LF 1LF  0RD 0RC"];

fn test_blank_true_negatives() {
    let mut fp = vec![];

    for &prog in _2_2_2 {
        if Prog::<2, 2>::from(prog).far_cant_blank(3) {
            fp.push(prog);
        }
    }

    for &prog in _3_2_2 {
        if Prog::<3, 2>::from(prog).far_cant_blank(3) {
            fp.push(prog);
        }
    }

    for &prog in _4_2_2 {
        if Prog::<4, 2>::from(prog).far_cant_blank(3) {
            fp.push(prog);
        }
    }

    for &prog in _5_2_2 {
        if Prog::<5, 2>::from(prog).far_cant_blank(3) {
            fp.push(prog);
        }
    }

    for &prog in _6_2_2 {
        if Prog::<6, 2>::from(prog).far_cant_blank(3) {
            fp.push(prog);
        }
    }

    assert_holdouts_match("blank true negatives", &[], &[], fp);
}

/**************************************/

pub fn test_holdouts() {
    println!("lin rec");
    test_linrec();

    println!("true negatives");
    test_blank_true_negatives();

    println!("far");
    test_far();
}

/**************************************/

#[expect(clippy::panic)]
pub fn assert_holdouts_match<T>(
    context: impl core::fmt::Display,
    champs: &[&str],
    holdouts: &[&str],
    result: impl IntoIterator<Item = T>,
) where
    T: ToString,
{
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
        let mut message = context.to_string();
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

        panic!("{message}");
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
        let mut message = context.to_string();
        use core::fmt::Write as _;

        write!(&mut message, "\nmissing champs: {missing_champs:#?}")
            .unwrap();

        panic!("{message}");
    }

    if result != expected {
        let collected_not_expected =
            result.difference(&expected).collect::<Vec<_>>();
        let expected_not_collected =
            expected.difference(&result).collect::<Vec<_>>();

        let mut message = context.to_string();
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

        panic!("{message}");
    }
}
