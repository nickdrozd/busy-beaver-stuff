pub const _4_2_2_ch: &[&str] = &["1RB 1LD  1RC 1RB  1LC 1LA  0RC 0RD"];

pub const _4_2_2_ho: &[&str] = &[
    "1RB 0LB  0LC 0LA  1RD 1LC  0RC 1RA",
    "1RB 0LC  0LC 1RA  0RA 1LD  1LC 0RA",
    "1RB 0LC  1LB 1RA  0RA 1LD  1LC 0RA",
    "1RB 1RC  0RC 0RD  1LD 0LA  1LC 0RB",
];

pub const _4_2_1_ch: &[&str] = &["1RB 1LD  1RC 1RB  1LC 1LA  0RC 0RD"];

pub const _4_2_1_ho: &[&str] = &[
    "1RB 0RD  1LC 1LB  1RD 0LB  0RD 1RA",
    "1RB 1LC  1LB 1RA  0LC 0LD  0RA 0RD",
    "1RB 1LC  1LC 1RA  0LC 0LD  0RA 0RD",
];

pub const _2_4_2_ch: &[&str] = &[
    "1RB 2RA 1RA 2RB  2LB 3LA 0RB 0RA",
    "1RB 2RB 3LA 2RA  2LB 1LA 0RB 3RA",
];

pub const _2_4_2_ho: &[&str] = &[
    "1RB 0RA 3RB 1LA  2LA 0LB 1LA 2RA",
    "1RB 0RB 2RA 0LB  1LB 2RB 3LA 0RA",
    "1RB 2RB 3LA 0RB  0LB 1LA 0LA 2RA",
    "1RB 3LB 0RB 2RB  2LA 0RA 0LB 3RA",
    "1RB 3RA 1LB 1RB  2LA 0LB 3RB 1LA",
    "1RB 3RB 0RB 0LA  2LB 3RA 3LA 1LA",
];

pub const _2_4_1_ch: &[&str] = &[
    "1RB 2LA 1RA 1LB  0LB 2RB 3RB 1LA",
    "1RB 2RA 1LA 2LB  2LB 3RB 0RB 1RA",
    "1RB 2RA 1RA 2RB  2LB 3LA 0RB 0RA",
    "1RB 2RB 3LA 2RA  2LB 1LA 0RB 3RA",
    "1RB 3RB 1LA 1LB  1LB 2RA 3RB 2LA",
];

pub const _2_4_1_ho: &[&str] = &[
    "1RB 0LA 1LB 2RB  2LB 3RB 1LA 0RA",
    "1RB 2LB 0RA 1LB  2LB 3LA 1RA 0RB",
    "1RB 2RA 1LB 0LA  2LB 1LA 3RA 3LB",
    "1RB 2RA 3LA 0RB  2LB 3LA 1LB 2RB",
    "1RB 2RA 3LB 2RA  0LB 2LA 3LA 0RA",
    "1RB 2RB 0LA ...  2LB 3LA 0RB 1RB",
    "1RB 2RB 0LA 2LA  2LB 3RB 0RB 1LA",
    "1RB 2RB 3LA 1LA  2LB 2RA 0LB 0RA",
    "1RB 2RB 3LA 2RA  2LB 1LA 1LB 3RB",
    "1RB 2RB 3RB 0LA  2LB 3LA 0RB 1RB",
    "1RB 3LA 1LB 0RB  2LB 2RA 3LA 1RA",
    "1RB 3LA 3LB 2RA  0LB 2RB 1LA 0RA",
    "1RB 3LA 3LB 2RB  0LB 2RB 1LA 1RA",
    "1RB 3LB 0RA 3RA  1LB 2RA 1LA 2RB",
    "1RB 3RA 3LB 0RA  1LB 2LA 0RA 0LA",
    "1RB 3RA 3RA 1LB  2LB 2RB 3LA 1LA",
    "1RB 3RB 1LA 1LB  2LB 2RA 0LB 1RB",
    "1RB 3RB 1LA 2RB  1LB 2RA 3LA 0LB",
    "1RB 3RB 1RA 0RA  2LB 2LA 1LA 3LB",
    "1RB 3RB 3LB 1LA  2LB 1RB 3LA 2RB",
];

/**************************************/

pub const _3_2_3_ch: &[&str] = &[];
pub const _3_2_3_ho: &[&str] = &["1RB 1LA  0LA 1LC  0RB 1RC"];

pub const _2_3_3_ch: &[&str] = &[];
pub const _2_3_3_ho: &[&str] = &[
    "1RB 1RA 2RB  2LB 1LA 0RB",
    "1RB 2RA 2LA  0LB 1LA 2RB",
    "1RB 2RA 2RB  2LA 1LA 1LB",
];

/**************************************/

pub const _7_0_ch: &[&str] = &[
    "1RB 2LA 1RA  1LC 1LA 2RB  ... 1LA ...",
    "1RB 2LA 1RA  1LC 2LA 2RB  ... 1LA ...",
    "1RB 2LA 1RA 1RA  1LB 1LA 3RB ...",
];

pub const _7_0_ho: &[&str] = &[
    "1RB ... ...  2RC 2RB 1LB  2LC 1RA 0LC",
    "1RB ... ...  2RC 2RB 1LB  2LD ... ...  ... 1RA 0LD",
    "1RB 1RA 0LA  2RC 2RB 1LB  2LA ... ...",
    "1RB 2RA 1LA  2LA 0RC ...  ... 2RB 2LB",
    "1RB 2RA 1LB  0LC 0RA 1LA  ... 2LA ...",
];

pub const _7_1_ho: &[&str] = &[
    "1RB ... ...  1LB 0LC 2RB  2RC 0RB 1LC",
    "1RB ... ...  1LB 2RC 0LC  1RC 2RB 1LC",
    "1RB ... ...  2LB 0LC 1RB  2RC 0RB 1LC",
    "1RB ... ...  2LB 1LC 1RC  2RC 2LC 1RB",
    "1RB ... ...  2RC 2RB 1LB  2LC 1RA 0LC",
    "1RB 0LC ...  2LB 2RC ...  1RC 2RA 1LC",
    "1RB 1LB ...  1LB 1LC 0RB  0RC 2LA ...",
    "1RB 1RC ...  1LB 1LA ...  2RC 0LC 0RA",
    "1RB 2RB 0LA ...  2LB 3LA 0RB 1RB",
];

/**************************************/

pub const _8_0_ch: &[&str] = &[
    "1RB 1LA ... ...  1RC 3LB 1RB ...  2LA 2LC ... 0LC",
    "1RB 1LC ...  1LA 1LC 2RB  1RB 2LC 1RC",
    "1RB 2LA 1LC  0LA 2RB 1LB  ... 1RA 1RC",
    "1RB 2LA 1RA  1LC 1LA 2RB  ... 1LA ...",
    "1RB 2LA 1RA  1LC 2LA 2RB  ... 1LA ...",
    "1RB 2LA 1RA 1RA  1LB 1LA 3RB ...",
    "1RB 2LA 1RA  1RC 2RB 0RC  1LA ... 1LA",
    "1RB ... 2LC  1LC 2RB 1LB  1LA 2RC 2LA",
    "1RB 2RA 1LC  2LB 0RB 2LA  ... 1LB 1LA",
    "1RB 2RA 2RC  1LC ... 1LA  1RA 2LB 1LC",
    "1RB ... 2RB  1LC 0LB 1RA  1RA 2LC 1RC",
    "1RB 2RC 1LA  2LA 1RB ...  2RB 2RA 1LC",
];

pub const _8_2_ch: &[&str] = &[
    "1RB 0RC ...  0RC ... ...  0RD ... 1RC  2LD 0LA 0LB",
    "1RB 1LD  1RC 1RB  1LC 1LA  0RC 0RD",
    "1RB ... ... ...  1RC ... ... ...  0RD ... 3LC ...  2LD 2RD 3RD 0RC",
    "1RB ... ... ...  1RC ... ... ...  2LC 2RC 3RC 0RD  0RC ... 3LD ...",
    "1RB 2RA 1LC  2LB 0RB 2LA  ... 1LB 1LA",
    "1RB 2RA 1RA 2RB  2LB 3LA 0RB 0RA",
    "1RB 2RB 3LA 2RA  2LB 1LA 0RB 3RA",
    "1RB 2RC 1RA ...  2LB 3LA 0RB 0RA  ... ... ... 2RB",
];

/**************************************/

pub const _3_2_3_: (&[&str], &[&str]) = (&[], _3_2_3_ho);
pub const _2_3_3_: (&[&str], &[&str]) = (&[], _2_3_3_ho);
pub const _4_2_1_: (&[&str], &[&str]) = (_4_2_1_ch, _4_2_1_ho);
pub const _4_2_2_: (&[&str], &[&str]) = (_4_2_2_ch, _4_2_2_ho);
pub const _2_4_1_: (&[&str], &[&str]) = (_2_4_1_ch, _2_4_1_ho);
pub const _2_4_2_: (&[&str], &[&str]) = (_2_4_2_ch, _2_4_2_ho);

pub const _7_0_: (&[&str], &[&str]) = (_7_0_ch, _7_0_ho);
pub const _7_1_: (&[&str], &[&str]) = (&[], _7_1_ho);
pub const _7_2_: (&[&str], &[&str]) = (&[], &[]);

/**************************************/

use std::{fs, sync::LazyLock};

fn load_holdouts(name: &str) -> Vec<&'static str> {
    let path = format!(
        "{}/../../test/data/holdouts/{name}.prog",
        env!("CARGO_MANIFEST_DIR"),
    );
    let contents = fs::read_to_string(&path).unwrap();
    let contents: &'static str = Box::leak(contents.into_boxed_str());

    contents.lines().filter(|line| !line.is_empty()).collect()
}

macro_rules! holdouts {
    (
        $champs:ident,
        $holdouts:ident,
        $combined:ident,
        $file:literal
    ) => {
        pub static $holdouts: LazyLock<Vec<&'static str>> =
            LazyLock::new(|| load_holdouts($file));

        pub static $combined: (&[&str], &LazyLock<Vec<&'static str>>) =
            ($champs, &$holdouts);
    };
}

holdouts!(_8_0_ch, _8_0_ho, _8_0_, "halt");
holdouts!(_8_2_ch, _8_2_ho, _8_2_, "blank");

/**************************************/

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn assert_no_duplicates(holdouts: &[&str]) {
        let mut seen = HashSet::new();

        for holdout in holdouts {
            assert!(
                seen.insert(*holdout),
                "duplicate holdout: {holdout}"
            );
        }
    }

    fn assert_subset(subset: &[&str], superset: &[&str]) {
        let superset: HashSet<_> = superset.iter().copied().collect();
        let missing: Vec<_> = subset
            .iter()
            .copied()
            .filter(|holdout| !superset.contains(holdout))
            .collect();

        assert!(missing.is_empty(), "missing holdouts: {missing:#?}");
    }

    #[test]
    fn eight_zero_holdouts_have_no_duplicates() {
        assert_no_duplicates(_8_0_ho.as_slice());
    }

    #[test]
    fn eight_two_holdouts_have_no_duplicates() {
        assert_no_duplicates(_8_2_ho.as_slice());
    }

    #[test]
    fn four_two_two_holdouts_are_a_subset_of_eight_two() {
        assert_subset(_4_2_2_ho, _8_2_ho.as_slice());
    }

    #[test]
    fn two_four_two_holdouts_are_a_subset_of_eight_two() {
        assert_subset(_2_4_2_ho, _8_2_ho.as_slice());
    }

    #[test]
    fn seven_zero_holdouts_are_a_subset_of_eight_zero() {
        assert_subset(_7_0_ho, _8_0_ho.as_slice());
    }
}
