use crate::holdouts::*;
use tm::Prog;

/**************************************/

const LIN_CHECK: usize = 11_000;

fn test_linrec() {
    for progs in [_4_2_1_ch, _4_2_1_ho, _4_2_2_ch, _4_2_2_ho] {
        for &prog in progs {
            assert!(
                !Prog::<4, 2>::from(prog)
                    .term_or_rec_fresh(LIN_CHECK)
                    .is_settled(),
                "{prog}",
            );
        }
    }

    for progs in [_2_4_1_ch, _2_4_1_ho, _2_4_2_ch, _2_4_2_ho] {
        for &prog in progs {
            assert!(
                !Prog::<2, 4>::from(prog)
                    .term_or_rec_fresh(LIN_CHECK)
                    .is_settled(),
                "{prog}",
            );
        }
    }
}

fn test_far() {
    println!("4-2-2");
    for &prog in _4_2_2_ho {
        assert!(!Prog::<4, 2>::from(prog).far_cant_blank(3));
    }

    println!("2-4-2");
    for &prog in _2_4_2_ho {
        assert!(!Prog::<2, 4>::from(prog).far_cant_blank(3));
    }

    println!("4-2-1");
    for &prog in _4_2_1_ho {
        assert!(!Prog::<4, 2>::from(prog).far_cant_spinout(3));
    }

    println!("2-4-1");
    for &prog in _2_4_1_ho {
        assert!(!Prog::<2, 4>::from(prog).far_cant_spinout(3));
    }
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

    assert!(fp.is_empty(), "{} || {fp:?}", fp.len());
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
